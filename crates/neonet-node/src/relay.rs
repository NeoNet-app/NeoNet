use std::sync::Arc;
use std::time::Duration;

use neonet_core::NodeType;
use neonet_proto::dht::{RendezvousRegister, RendezvousRegisterResp};
use neonet_proto::frame::{FrameKind, FramePayload};
use rand::RngCore;

use crate::keystore::{Keystore, UnlockedKeystore};
use crate::session::{self, write_encrypted_frame, read_encrypted_frame};
use crate::quic;

type Res<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

/// Detect public IP via ipify.
async fn detect_public_ip() -> Res<String> {
    let resp = reqwest::get("https://api.ipify.org").await?.text().await?;
    Ok(resp.trim().to_string())
}

/// Register this relay on a rendezvous server.
/// Returns Ok(()) on success.
pub async fn register_on_rendezvous(
    rendezvous_addr: &str,
    domain: &str,
    listen_port: u16,
    ks: &UnlockedKeystore,
) -> Res<()> {
    let addr: std::net::SocketAddr = rendezvous_addr.parse()?;

    // Use 127.0.0.1 if the rendezvous is on localhost (local testing),
    // otherwise detect public IP.
    let is_local = rendezvous_addr.starts_with("127.0.0.1")
        || rendezvous_addr.starts_with("localhost")
        || rendezvous_addr.starts_with("[::1]");
    let public_ip = if is_local {
        "127.0.0.1".into()
    } else {
        match detect_public_ip().await {
            Ok(ip) => ip,
            Err(_) => {
                log::warn!("Impossible de détecter l'IP publique, utilisation de 127.0.0.1");
                "127.0.0.1".into()
            }
        }
    };
    let our_addr = format!("{public_ip}:{listen_port}");

    // Connect + handshake.
    let mut endpoint = quinn::Endpoint::client("0.0.0.0:0".parse()?)?;
    endpoint.set_default_client_config(quic::make_client_config_pub());

    let conn = endpoint.connect(addr, "neonet-node")?.await?;
    let (mut send, mut recv) = conn.open_bi().await?;

    let mut session = session::handshake_initiator(&mut send, &mut recv, ks, addr).await?;

    // Build RendezvousRegister.
    let mut request_id = [0u8; 8];
    rand::rngs::OsRng.fill_bytes(&mut request_id);

    // Sign: domain || addr || ttl
    let ttl: u32 = 300;
    let mut sig_msg = Vec::new();
    sig_msg.extend_from_slice(domain.as_bytes());
    sig_msg.extend_from_slice(our_addr.as_bytes());
    sig_msg.extend_from_slice(&ttl.to_le_bytes());
    let sig = ks.sign(&sig_msg);

    let reg = RendezvousRegister {
        request_id,
        domain: domain.into(),
        addr: our_addr.clone(),
        pubkey: ks.pubkey(),
        node_type: NodeType::Relay,
        ttl,
        sig,
    };

    let payload = FramePayload {
        kind: FrameKind::RendezvousRegister,
        data: postcard::to_allocvec(&reg)?,
    };

    // Open a new bi-stream for the register message.
    let (mut s2, mut r2) = conn.open_bi().await?;
    write_encrypted_frame(&mut s2, &mut session, &payload).await?;
    s2.finish()?;

    let resp_frame = read_encrypted_frame(&mut r2, &mut session).await?;
    if resp_frame.kind != FrameKind::RendezvousRegisterResp {
        return Err(format!("unexpected response: {:?}", resp_frame.kind).into());
    }
    let resp: RendezvousRegisterResp = postcard::from_bytes(&resp_frame.data)?;
    if !resp.accepted {
        return Err("registration rejected".into());
    }

    log::info!("Enregistré sur {rendezvous_addr} comme {domain} ({our_addr})");
    conn.close(0u32.into(), b"registered");
    endpoint.wait_idle().await;
    Ok(())
}

/// Spawn a background task that re-registers every 240s.
pub fn spawn_reregister_loop(
    rendezvous_addr: String,
    domain: String,
    listen_port: u16,
    ks: Arc<UnlockedKeystore>,
) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(240));
        interval.tick().await; // skip first immediate tick
        loop {
            interval.tick().await;
            log::info!("Re-enregistrement sur {rendezvous_addr}...");
            if let Err(e) =
                register_on_rendezvous(&rendezvous_addr, &domain, listen_port, &ks).await
            {
                log::warn!("Re-enregistrement échoué: {e}");
            }
        }
    });
}

/// Lookup a domain on a rendezvous server (client-side).
pub async fn lookup_domain(
    rendezvous_addr: &str,
    domain: &str,
    ks: &UnlockedKeystore,
) -> Res<(String, [u8; 32])> {
    use neonet_proto::dht::{RendezvousLookup, RendezvousLookupResp};

    let addr: std::net::SocketAddr = rendezvous_addr.parse()?;

    let mut endpoint = quinn::Endpoint::client("0.0.0.0:0".parse()?)?;
    endpoint.set_default_client_config(quic::make_client_config_pub());

    let conn = endpoint.connect(addr, "neonet-node")?.await?;
    let (mut send, mut recv) = conn.open_bi().await?;

    let mut session = session::handshake_initiator(&mut send, &mut recv, ks, addr).await?;

    let mut request_id = [0u8; 8];
    rand::rngs::OsRng.fill_bytes(&mut request_id);

    let lookup = RendezvousLookup {
        request_id,
        domain: domain.into(),
    };
    let payload = FramePayload {
        kind: FrameKind::RendezvousLookup,
        data: postcard::to_allocvec(&lookup)?,
    };

    let (mut s2, mut r2) = conn.open_bi().await?;
    write_encrypted_frame(&mut s2, &mut session, &payload).await?;
    s2.finish()?;

    let resp_frame = read_encrypted_frame(&mut r2, &mut session).await?;
    if resp_frame.kind != FrameKind::RendezvousLookupResp {
        return Err(format!("unexpected: {:?}", resp_frame.kind).into());
    }
    let resp: RendezvousLookupResp = postcard::from_bytes(&resp_frame.data)?;

    conn.close(0u32.into(), b"done");
    endpoint.wait_idle().await;

    if resp.found {
        let peer_addr = resp.addr.ok_or("found but no addr")?;
        let peer_pubkey = resp.pubkey.ok_or("found but no pubkey")?;
        Ok((peer_addr, peer_pubkey))
    } else {
        Err(format!("{domain} not found on {rendezvous_addr}").into())
    }
}
