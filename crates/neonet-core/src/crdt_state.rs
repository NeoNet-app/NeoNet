use std::collections::HashMap;

use neonet_crypto::{Hash, Signature};
use serde::{Deserialize, Serialize};

use crate::identity::Identity;

/// CRDT operation type.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CrdtOp {
    /// Last-Writer-Wins register.
    Lww,
    /// Observed-Remove Set.
    Set,
    /// PN-Counter.
    Counter,
}

/// CRDT state event.
///
/// Used for data that changes often and doesn't need causal history:
/// presence, membership, room settings. Merge is mathematically guaranteed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrdtState {
    /// `blake3(namespace)` — identifies the CRDT document.
    pub doc_id: Hash,

    /// Author identity.
    pub author: Identity,

    /// Vector clock: `{pubkey_bytes_hex -> counter}`.
    pub vclock: HashMap<[u8; 32], u64>,

    /// CRDT operation type.
    pub op: CrdtOp,

    /// Always encrypted (opaque blob for relays).
    pub payload: Vec<u8>,

    /// Ed25519 signature.
    pub signature: Signature,

    /// Optional TTL in seconds (e.g. for presence expiration).
    pub ttl: Option<u64>,
}
