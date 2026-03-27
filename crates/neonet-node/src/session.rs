use std::net::SocketAddr;
use std::time::Instant;

use hkdf::Hkdf;
use neonet_core::NodeType;
use neonet_crypto::{EphemeralSecret, VerifyingKey};
use neonet_proto::frame::{FrameKind, FramePayload};
use neonet_proto::handshake::{
    Ack, Finish, HelloInit, HelloResp, IdentityPayload, HKDF_SALT, PADDING_BLOCK,
};
use rand::RngCore;
use sha2::Sha256;
use zeroize::Zeroizing;

use crate::keystore::{Keystore, UnlockedKeystore};

type Res<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

// ── Session ──────────────────────────────────────────────────────────

pub struct Session {
    pub peer_pubkey: [u8; 32],
    pub peer_addr: SocketAddr,
    pub node_type: NodeType,
    session_key_send: Zeroizing<[u8; 32]>,
    session_key_recv: Zeroizing<[u8; 32]>,
    nonce_send: u64,
    nonce_recv: u64,
    pub established_at: Instant,
}

impl Session {
    fn make_nonce(counter: u64) -> [u8; 12] {
        let mut nonce = [0u8; 12];
        nonce[4..12].copy_from_slice(&counter.to_be_bytes());
        nonce
    }

    pub fn encrypt_frame(&mut self, payload: &FramePayload) -> Res<Vec<u8>> {
        let plaintext = postcard::to_allocvec(payload)?;
        let nonce = Self::make_nonce(self.nonce_send);
        self.nonce_send += 1;
        let ct = neonet_crypto::seal_with_nonce(&self.session_key_send, &nonce, &plaintext)
            .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> { Box::new(e) })?;
        let mut out = Vec::with_capacity(12 + ct.len());
        out.extend_from_slice(&nonce);
        out.extend_from_slice(&ct);
        Ok(out)
    }

    pub fn decrypt_frame(&mut self, data: &[u8]) -> Res<FramePayload> {
        if data.len() < 12 {
            return Err("frame too short".into());
        }
        let nonce: [u8; 12] = data[..12].try_into().unwrap();
        let expected = Self::make_nonce(self.nonce_recv);
        if nonce != expected {
            return Err("nonce mismatch".into());
        }
        self.nonce_recv += 1;
        let pt = neonet_crypto::open_with_nonce(&self.session_key_recv, &nonce, &data[12..])
            .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> { Box::new(e) })?;
        Ok(postcard::from_bytes(&pt)?)
    }
}

// ── Wire helpers ─────────────────────────────────────────────────────

async fn write_bytes(send: &mut quinn::SendStream, data: &[u8]) -> Res<()> {
    send.write_all(&(data.len() as u32).to_be_bytes()).await?;
    send.write_all(data).await?;
    Ok(())
}

async fn read_bytes(recv: &mut quinn::RecvStream) -> Res<Vec<u8>> {
    let mut hdr = [0u8; 4];
    recv.read_exact(&mut hdr).await?;
    let len = u32::from_be_bytes(hdr) as usize;
    if len > 1_048_576 {
        return Err("message too large".into());
    }
    let mut buf = vec![0u8; len];
    recv.read_exact(&mut buf).await?;
    Ok(buf)
}

pub async fn write_encrypted_frame(
    send: &mut quinn::SendStream,
    session: &mut Session,
    payload: &FramePayload,
) -> Res<()> {
    let encrypted = session.encrypt_frame(payload)?;
    write_bytes(send, &encrypted).await
}

pub async fn read_encrypted_frame(
    recv: &mut quinn::RecvStream,
    session: &mut Session,
) -> Res<FramePayload> {
    let data = read_bytes(recv).await?;
    session.decrypt_frame(&data)
}

// ── Helpers ──────────────────────────────────────────────────────────

fn make_padding(content_len: usize) -> Vec<u8> {
    let padded = ((content_len + PADDING_BLOCK - 1) / PADDING_BLOCK) * PADDING_BLOCK;
    let pad_len = padded.saturating_sub(content_len).max(1);
    let mut p = vec![0u8; pad_len];
    rand::rngs::OsRng.fill_bytes(&mut p);
    p
}

/// Derive two 32-byte session keys via HKDF-SHA256.
/// info = session_id(16) || eph_pub_a(32) || eph_pub_b(32)
fn derive_keys(
    shared: &[u8; 32],
    session_id: &[u8; 16],
    eph_a: &[u8; 32],
    eph_b: &[u8; 32],
) -> (Zeroizing<[u8; 32]>, Zeroizing<[u8; 32]>) {
    let hk = Hkdf::<Sha256>::new(Some(HKDF_SALT), shared);
    let mut info = Vec::with_capacity(80);
    info.extend_from_slice(session_id);
    info.extend_from_slice(eph_a);
    info.extend_from_slice(eph_b);
    let mut out = Zeroizing::new([0u8; 64]);
    hk.expand(&info, out.as_mut()).expect("HKDF-SHA256 expand");
    let mut k_ab = Zeroizing::new([0u8; 32]);
    let mut k_ba = Zeroizing::new([0u8; 32]);
    k_ab.copy_from_slice(&out[..32]);
    k_ba.copy_from_slice(&out[32..]);
    (k_ab, k_ba)
}

fn build_identity(
    ks: &UnlockedKeystore,
    session_id: &[u8; 16],
    eph_pubkey: &[u8; 32],
) -> IdentityPayload {
    let mut msg = Vec::with_capacity(48);
    msg.extend_from_slice(session_id);
    msg.extend_from_slice(eph_pubkey);
    IdentityPayload {
        pubkey: ks.pubkey(),
        sig: ks.sign(&msg),
        node_type: NodeType::Full,
        features: 0,
    }
}

fn verify_identity(id: &IdentityPayload, session_id: &[u8; 16], eph: &[u8; 32]) -> bool {
    let vk = VerifyingKey(id.pubkey);
    let mut msg = Vec::with_capacity(48);
    msg.extend_from_slice(session_id);
    msg.extend_from_slice(eph);
    let sig = neonet_crypto::Signature::from_bytes(&id.sig);
    vk.verify(&msg, &sig)
}

// ── Handshake protocol ──────────────────────────────────────────────
//
// Key derivation strategy:
//   1. "handshake keys" derived with session_id = [0;16]
//      → used to encrypt identities in HelloResp and Finish
//      (both sides know both eph pubkeys, can derive independently)
//   2. "session keys" derived with the real session_id from Finish
//      → used for all subsequent encrypted frames
//
// Signatures use the same placeholder session_id = [0;16] so both
// sides can verify before the real session_id is established.

const HANDSHAKE_SID: [u8; 16] = [0u8; 16];

/// Initiator side of the NeoNet handshake.
pub async fn handshake_initiator(
    send: &mut quinn::SendStream,
    recv: &mut quinn::RecvStream,
    ks: &UnlockedKeystore,
    remote: SocketAddr,
) -> Res<Session> {
    // 1. Ephemeral X25519.
    let eph = EphemeralSecret::generate();
    let eph_a = eph.public_key_bytes();

    // 2. Send HelloInit.
    let hello = HelloInit {
        version: 1,
        eph_pubkey: eph_a,
        padding: make_padding(33),
    };
    write_bytes(send, &postcard::to_allocvec(&hello)?).await?;
    log::info!("HelloInit envoyé");

    // 3. Receive HelloResp.
    let resp: HelloResp = postcard::from_bytes(&read_bytes(recv).await?)?;
    log::info!("HelloResp reçu");

    // 4. DH + handshake keys (session_id = 0).
    let shared = eph.diffie_hellman(&resp.eph_pubkey);
    let (hs_key_ab, _hs_key_ba) = derive_keys(&shared.0, &HANDSHAKE_SID, &eph_a, &resp.eph_pubkey);

    // 5. Decrypt identity_b.
    let id_b_raw = neonet_crypto::open(&hs_key_ab, &resp.identity_enc)
        .map_err(|_| -> Box<dyn std::error::Error + Send + Sync> {
            "decrypt identity_b failed".into()
        })?;
    let id_b: IdentityPayload = postcard::from_bytes(&id_b_raw)?;

    // 6. Verify B's signature.
    if !verify_identity(&id_b, &HANDSHAKE_SID, &resp.eph_pubkey) {
        return Err("identity_b sig invalid".into());
    }
    log::info!(
        "Identité B vérifiée : {}",
        neonet_crypto::encode_verifying_key(&VerifyingKey(id_b.pubkey))
    );

    // 7. Generate real session_id.
    let mut session_id = [0u8; 16];
    rand::rngs::OsRng.fill_bytes(&mut session_id);

    // 8. Build + encrypt our identity (with handshake key).
    let my_id = build_identity(ks, &HANDSHAKE_SID, &eph_a);
    let my_id_enc = neonet_crypto::seal(&hs_key_ab, &postcard::to_allocvec(&my_id)?)
        .map_err(|_| -> Box<dyn std::error::Error + Send + Sync> {
            "encrypt identity_a failed".into()
        })?;

    // 9. Send Finish.
    let finish = Finish {
        session_id,
        identity_enc: my_id_enc,
        padding: make_padding(16 + 120),
    };
    write_bytes(send, &postcard::to_allocvec(&finish)?).await?;
    log::info!("Finish envoyé");

    // 10. Receive Ack.
    let ack: Ack = postcard::from_bytes(&read_bytes(recv).await?)?;
    if ack.session_id != session_id {
        return Err("Ack session_id mismatch".into());
    }
    log::info!("Ack reçu — session établie");

    // 11. Derive final session keys with real session_id.
    let (key_ab, key_ba) = derive_keys(&shared.0, &session_id, &eph_a, &resp.eph_pubkey);

    Ok(Session {
        peer_pubkey: id_b.pubkey,
        peer_addr: remote,
        node_type: id_b.node_type,
        session_key_send: key_ab,
        session_key_recv: key_ba,
        nonce_send: 0,
        nonce_recv: 0,
        established_at: Instant::now(),
    })
}

/// Responder side of the NeoNet handshake.
pub async fn handshake_responder(
    send: &mut quinn::SendStream,
    recv: &mut quinn::RecvStream,
    ks: &UnlockedKeystore,
    remote: SocketAddr,
) -> Res<Session> {
    // 1. Receive HelloInit.
    let hello: HelloInit = postcard::from_bytes(&read_bytes(recv).await?)?;
    if hello.version != 1 {
        return Err(format!("unsupported version {}", hello.version).into());
    }
    log::info!("HelloInit reçu de {remote}");

    // 2. Ephemeral X25519.
    let eph = EphemeralSecret::generate();
    let eph_b = eph.public_key_bytes();

    // 3. DH + handshake keys (session_id = 0).
    let shared = eph.diffie_hellman(&hello.eph_pubkey);
    let (hs_key_ab, _hs_key_ba) = derive_keys(&shared.0, &HANDSHAKE_SID, &hello.eph_pubkey, &eph_b);

    // 4. Build + encrypt our identity.
    let my_id = build_identity(ks, &HANDSHAKE_SID, &eph_b);
    let id_enc = neonet_crypto::seal(&hs_key_ab, &postcard::to_allocvec(&my_id)?)
        .map_err(|_| -> Box<dyn std::error::Error + Send + Sync> {
            "encrypt identity_b failed".into()
        })?;

    // 5. Send HelloResp.
    let resp = HelloResp {
        eph_pubkey: eph_b,
        identity_enc: id_enc,
        padding: make_padding(32 + 120),
    };
    write_bytes(send, &postcard::to_allocvec(&resp)?).await?;
    log::info!("HelloResp envoyé");

    // 6. Receive Finish.
    let finish: Finish = postcard::from_bytes(&read_bytes(recv).await?)?;
    let session_id = finish.session_id;
    log::info!("Finish reçu");

    // 7. Decrypt identity_a (with handshake key).
    let id_a_raw = neonet_crypto::open(&hs_key_ab, &finish.identity_enc)
        .map_err(|_| -> Box<dyn std::error::Error + Send + Sync> {
            "decrypt identity_a failed".into()
        })?;
    let id_a: IdentityPayload = postcard::from_bytes(&id_a_raw)?;

    // 8. Verify A's signature.
    if !verify_identity(&id_a, &HANDSHAKE_SID, &hello.eph_pubkey) {
        return Err("identity_a sig invalid".into());
    }
    log::info!(
        "Identité A vérifiée : {}",
        neonet_crypto::encode_verifying_key(&VerifyingKey(id_a.pubkey))
    );

    // 9. Send Ack.
    let ack = Ack {
        session_id,
        padding: make_padding(16),
    };
    write_bytes(send, &postcard::to_allocvec(&ack)?).await?;
    log::info!("Ack envoyé — session établie");

    // 10. Derive final session keys with real session_id.
    let (key_ab, key_ba) = derive_keys(&shared.0, &session_id, &hello.eph_pubkey, &eph_b);

    Ok(Session {
        peer_pubkey: id_a.pubkey,
        peer_addr: remote,
        node_type: id_a.node_type,
        // B sends B→A (key_ba), receives A→B (key_ab)
        session_key_send: key_ba,
        session_key_recv: key_ab,
        nonce_send: 0,
        nonce_recv: 0,
        established_at: Instant::now(),
    })
}
