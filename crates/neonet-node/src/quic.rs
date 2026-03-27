use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Instant;

use quinn::{ClientConfig, Endpoint, ServerConfig};
use neonet_proto::dht::{
    RendezvousRegister, RendezvousRegisterResp,
    RendezvousLookup, RendezvousLookupResp,
};
use neonet_proto::frame::{FrameKind, FramePayload};
use tokio::sync::Mutex;

use crate::db;
use crate::keystore::UnlockedKeystore;
use crate::rendezvous_server::{self, RendezvousTable};
use crate::session::{self, Session, read_encrypted_frame, write_encrypted_frame};

type Res<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

pub type SessionMap = Arc<Mutex<HashMap<SocketAddr, Session>>>;

/// Shared context for the QUIC layer.
pub struct QuicContext {
    pub keystore: Arc<UnlockedKeystore>,
    pub db: Arc<std::sync::Mutex<rusqlite::Connection>>,
    pub sessions: SessionMap,
    /// Rendezvous directory (only populated in rendezvous mode).
    pub rendezvous_table: Option<RendezvousTable>,
}

// ── TLS ──────────────────────────────────────────────────────────────

fn generate_self_signed_cert(cn: &str) -> (
    rustls::pki_types::CertificateDer<'static>,
    rustls::pki_types::PrivateKeyDer<'static>,
) {
    let mut params = rcgen::CertificateParams::new(vec![cn.to_string()]).unwrap();
    params.distinguished_name.push(rcgen::DnType::CommonName, cn);
    let kp = rcgen::KeyPair::generate().unwrap();
    let cert = params.self_signed(&kp).unwrap();
    (
        rustls::pki_types::CertificateDer::from(cert.der().to_vec()),
        rustls::pki_types::PrivateKeyDer::Pkcs8(
            rustls::pki_types::PrivatePkcs8KeyDer::from(kp.serialize_der()),
        ),
    )
}

fn make_server_config(cn: &str) -> ServerConfig {
    let (cert, key) = generate_self_signed_cert(cn);
    let mut sc = rustls::ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(vec![cert], key)
        .expect("TLS server config");
    sc.alpn_protocols = vec![b"neonet/1".to_vec()];
    ServerConfig::with_crypto(Arc::new(
        quinn::crypto::rustls::QuicServerConfig::try_from(sc).unwrap(),
    ))
}

/// Public client config (also used by relay.rs).
pub fn make_client_config_pub() -> ClientConfig {
    let mut cc = rustls::ClientConfig::builder()
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(SkipVerify))
        .with_no_client_auth();
    cc.alpn_protocols = vec![b"neonet/1".to_vec()];
    ClientConfig::new(Arc::new(
        quinn::crypto::rustls::QuicClientConfig::try_from(cc).unwrap(),
    ))
}

#[derive(Debug)]
struct SkipVerify;

impl rustls::client::danger::ServerCertVerifier for SkipVerify {
    fn verify_server_cert(
        &self, _: &rustls::pki_types::CertificateDer<'_>,
        _: &[rustls::pki_types::CertificateDer<'_>],
        _: &rustls::pki_types::ServerName<'_>, _: &[u8],
        _: rustls::pki_types::UnixTime,
    ) -> Result<rustls::client::danger::ServerCertVerified, rustls::Error> {
        Ok(rustls::client::danger::ServerCertVerified::assertion())
    }
    fn verify_tls12_signature(
        &self, _: &[u8], _: &rustls::pki_types::CertificateDer<'_>,
        _: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }
    fn verify_tls13_signature(
        &self, _: &[u8], _: &rustls::pki_types::CertificateDer<'_>,
        _: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }
    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        rustls::crypto::ring::default_provider()
            .signature_verification_algorithms
            .supported_schemes()
    }
}

// ── TOFU ─────────────────────────────────────────────────────────────

fn do_tofu(ctx: &QuicContext, session: &Session) {
    let addr = session.peer_addr.to_string();
    let pubkey = neonet_crypto::encode_verifying_key(&neonet_crypto::VerifyingKey(session.peer_pubkey));
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;
    let conn = ctx.db.lock().unwrap();
    match db::tofu_check(&conn, &addr, &pubkey, now) {
        Ok(db::TofuResult::NewPeer) => log::info!("TOFU: nouveau pair {addr} → {pubkey}"),
        Ok(db::TofuResult::Known) => log::debug!("TOFU: pair connu {addr}"),
        Ok(db::TofuResult::Mismatch { expected }) => {
            log::warn!("TOFU MISMATCH {addr}: attendu {expected}, reçu {pubkey}")
        }
        Err(e) => log::warn!("TOFU DB error: {e}"),
    }
}

// ── Listener ─────────────────────────────────────────────────────────

pub async fn start_listener(port: u16, ctx: Arc<QuicContext>) -> Res<Endpoint> {
    let addr: SocketAddr = format!("0.0.0.0:{port}").parse()?;
    let endpoint = Endpoint::server(make_server_config("neonet-node"), addr)?;
    log::info!("QUIC listener started on {addr}");

    let ep = endpoint.clone();
    tokio::spawn(async move {
        while let Some(incoming) = ep.accept().await {
            let ctx = ctx.clone();
            tokio::spawn(async move {
                match incoming.await {
                    Ok(conn) => {
                        let remote = conn.remote_address();
                        log::info!("Connexion entrante de {remote}");
                        handle_connection(conn, ctx).await;
                    }
                    Err(e) => log::warn!("Accept failed: {e}"),
                }
            });
        }
    });

    Ok(endpoint)
}

async fn handle_connection(conn: quinn::Connection, ctx: Arc<QuicContext>) {
    let remote = conn.remote_address();

    // First bi-stream: handshake.
    let (mut send, mut recv) = match conn.accept_bi().await {
        Ok(s) => s,
        Err(e) => { log::warn!("No stream from {remote}: {e}"); return; }
    };

    let session = match session::handshake_responder(
        &mut send, &mut recv, &ctx.keystore, remote,
    ).await {
        Ok(s) => s,
        Err(e) => { log::warn!("Handshake failed {remote}: {e}"); return; }
    };

    do_tofu(&ctx, &session);
    let peer_vk = neonet_crypto::encode_verifying_key(&neonet_crypto::VerifyingKey(session.peer_pubkey));
    log::info!("Handshake OK avec {remote} — {peer_vk}");

    let session = Arc::new(Mutex::new(session));

    // Frame loop.
    loop {
        match conn.accept_bi().await {
            Ok((mut s, mut r)) => {
                let session = session.clone();
                let ctx = ctx.clone();
                tokio::spawn(async move {
                    let frame = {
                        let mut sess = session.lock().await;
                        read_encrypted_frame(&mut r, &mut sess).await
                    };
                    match frame {
                        Ok(frame) => {
                            dispatch_frame(frame, &mut s, &session, &ctx, remote).await;
                        }
                        Err(e) => log::warn!("Decrypt frame {remote}: {e}"),
                    }
                });
            }
            Err(quinn::ConnectionError::ApplicationClosed(_)) => {
                log::info!("Connection closed by {remote}");
                break;
            }
            Err(e) => { log::warn!("Stream error {remote}: {e}"); break; }
        }
    }
}

/// Dispatch a decrypted frame based on its kind.
async fn dispatch_frame(
    frame: FramePayload,
    send: &mut quinn::SendStream,
    session: &Arc<Mutex<Session>>,
    ctx: &Arc<QuicContext>,
    remote: SocketAddr,
) {
    match frame.kind {
        FrameKind::Ping => {
            log::info!("PING chiffré reçu de {remote}");
            let pong = FramePayload { kind: FrameKind::Pong, data: frame.data };
            let mut sess = session.lock().await;
            let _ = write_encrypted_frame(send, &mut sess, &pong).await;
            let _ = send.finish();
        }

        FrameKind::RendezvousRegister => {
            handle_rendezvous_register(&frame.data, send, session, ctx, remote).await;
        }

        FrameKind::RendezvousLookup => {
            handle_rendezvous_lookup(&frame.data, send, session, ctx, remote).await;
        }

        other => {
            log::info!("Frame {other:?} de {remote}");
            let _ = send.finish();
        }
    }
}

async fn handle_rendezvous_register(
    data: &[u8],
    send: &mut quinn::SendStream,
    session: &Arc<Mutex<Session>>,
    ctx: &Arc<QuicContext>,
    remote: SocketAddr,
) {
    let table = match &ctx.rendezvous_table {
        Some(t) => t,
        None => {
            log::warn!("Register reçu mais ce nœud n'est pas un rendezvous");
            let resp = RendezvousRegisterResp { request_id: [0; 8], accepted: false };
            let payload = FramePayload {
                kind: FrameKind::RendezvousRegisterResp,
                data: postcard::to_allocvec(&resp).unwrap_or_default(),
            };
            let mut sess = session.lock().await;
            let _ = write_encrypted_frame(send, &mut sess, &payload).await;
            let _ = send.finish();
            return;
        }
    };

    let reg: RendezvousRegister = match postcard::from_bytes(data) {
        Ok(r) => r,
        Err(e) => { log::warn!("Malformed register from {remote}: {e}"); return; }
    };

    log::info!(
        "Register: {} → {} (pubkey: {}, TTL {}s) de {remote}",
        reg.domain, reg.addr,
        neonet_crypto::encode_verifying_key(&neonet_crypto::VerifyingKey(reg.pubkey)),
        reg.ttl
    );

    rendezvous_server::register(
        table,
        reg.domain,
        reg.addr,
        reg.pubkey,
        reg.node_type,
        reg.ttl,
    ).await;

    let resp = RendezvousRegisterResp { request_id: reg.request_id, accepted: true };
    let payload = FramePayload {
        kind: FrameKind::RendezvousRegisterResp,
        data: postcard::to_allocvec(&resp).unwrap_or_default(),
    };
    let mut sess = session.lock().await;
    let _ = write_encrypted_frame(send, &mut sess, &payload).await;
    let _ = send.finish();
}

async fn handle_rendezvous_lookup(
    data: &[u8],
    send: &mut quinn::SendStream,
    session: &Arc<Mutex<Session>>,
    ctx: &Arc<QuicContext>,
    remote: SocketAddr,
) {
    let table = match &ctx.rendezvous_table {
        Some(t) => t,
        None => {
            log::warn!("Lookup reçu mais ce nœud n'est pas un rendezvous");
            let resp = RendezvousLookupResp {
                request_id: [0; 8], found: false, addr: None, pubkey: None,
            };
            let payload = FramePayload {
                kind: FrameKind::RendezvousLookupResp,
                data: postcard::to_allocvec(&resp).unwrap_or_default(),
            };
            let mut sess = session.lock().await;
            let _ = write_encrypted_frame(send, &mut sess, &payload).await;
            let _ = send.finish();
            return;
        }
    };

    let lookup: RendezvousLookup = match postcard::from_bytes(data) {
        Ok(l) => l,
        Err(e) => { log::warn!("Malformed lookup from {remote}: {e}"); return; }
    };

    log::info!("Lookup: {} de {remote}", lookup.domain);

    let entry = rendezvous_server::lookup(table, &lookup.domain).await;

    let resp = match entry {
        Some(e) => {
            log::info!("Lookup {} → {} trouvé", lookup.domain, e.addr);
            RendezvousLookupResp {
                request_id: lookup.request_id,
                found: true,
                addr: Some(e.addr),
                pubkey: Some(e.pubkey),
            }
        }
        None => {
            log::info!("Lookup {} → non trouvé", lookup.domain);
            RendezvousLookupResp {
                request_id: lookup.request_id,
                found: false,
                addr: None,
                pubkey: None,
            }
        }
    };

    let payload = FramePayload {
        kind: FrameKind::RendezvousLookupResp,
        data: postcard::to_allocvec(&resp).unwrap_or_default(),
    };
    let mut sess = session.lock().await;
    let _ = write_encrypted_frame(send, &mut sess, &payload).await;
    let _ = send.finish();
}

// ── Client connect + ping ────────────────────────────────────────────

pub async fn connect_and_ping(target: &str, ks: &UnlockedKeystore) -> Res<()> {
    let addr: SocketAddr = target.parse()?;

    let mut endpoint = Endpoint::client("0.0.0.0:0".parse()?)?;
    endpoint.set_default_client_config(make_client_config_pub());

    let conn = endpoint.connect(addr, "neonet-node")?.await?;
    let (mut send, mut recv) = conn.open_bi().await?;
    let t0 = Instant::now();

    let mut session = session::handshake_initiator(&mut send, &mut recv, ks, addr).await?;

    let hs_time = t0.elapsed();
    let peer_vk = neonet_crypto::encode_verifying_key(&neonet_crypto::VerifyingKey(session.peer_pubkey));
    println!("Handshake OK — session établie avec {} ({}ms)", peer_vk, hs_time.as_millis());

    let (mut s2, mut r2) = conn.open_bi().await?;
    let ping = FramePayload { kind: FrameKind::Ping, data: vec![] };
    let t1 = Instant::now();
    write_encrypted_frame(&mut s2, &mut session, &ping).await?;
    s2.finish()?;

    let resp = read_encrypted_frame(&mut r2, &mut session).await?;
    let rtt = t1.elapsed();

    if resp.kind == FrameKind::Pong {
        println!("PONG reçu de {target} en {}ms (chiffré)", rtt.as_millis());
    } else {
        println!("Réponse inattendue: {:?}", resp.kind);
    }

    conn.close(0u32.into(), b"done");
    endpoint.wait_idle().await;
    Ok(())
}
