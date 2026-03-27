use neonet_crypto::{Hash, Signature};
use serde::{Deserialize, Serialize};

use crate::event_kind::EventKind;
use crate::identity::Identity;

/// Immutable DAG event.
///
/// Each message is an event with a content-derived hash that points
/// to its causal parents. Payload is always E2EE-encrypted.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    /// `blake3(payload)` — deterministic, no server-assigned IDs.
    pub id: Hash,

    /// Author identity. `None` for anonymous mode.
    pub author: Option<Identity>,

    /// Causal parent references (DAG edges).
    pub parents: Vec<Hash>,

    /// Event type.
    pub kind: EventKind,

    /// Always encrypted (opaque blob for relays).
    pub payload: Vec<u8>,

    /// Ed25519 signature over the canonical event bytes.
    pub signature: Signature,

    /// Non-authoritative timestamp hint (client-declared, not trusted).
    pub ts: u64,
}

impl Event {
    /// Compute the expected ID from the payload.
    pub fn compute_id(payload: &[u8]) -> Hash {
        Hash::digest(payload)
    }

    /// Verify that `self.id` matches `blake3(self.payload)`.
    pub fn verify_id(&self) -> bool {
        self.id == Self::compute_id(&self.payload)
    }
}
