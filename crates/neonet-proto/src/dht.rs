use neonet_core::{NodeId, NodeInfo, NodeType};
use serde::{Deserialize, Serialize};

use crate::bytes64::serde_bytes64;

// ── DHT messages (Kademlia) ──────────────────────────────────────────
//
// All DHT messages are transported inside FrameKind::DhtMessage frames,
// so they are encrypted and authenticated by the session layer.

/// Top-level DHT message enum for (de)serialisation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DhtMessage {
    Ping(Ping),
    Pong(Pong),
    FindNode(FindNode),
    FindNodeResp(FindNodeResp),
    FindValue(FindValue),
    FindValueResp(FindValueResp),
    Store(Store),
    StoreResp(StoreResp),
    Bootstrap(BootstrapReq),
    BootstrapResp(BootstrapResp),
}

// ── PING / PONG ──────────────────────────────────────────────────────

/// Verify that a node is alive.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ping {
    pub request_id: [u8; 8],
    pub sender_id: NodeId,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pong {
    pub request_id: [u8; 8],
    pub sender_id: NodeId,
}

// ── FIND_NODE ────────────────────────────────────────────────────────

/// Request the K closest nodes to `target`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FindNode {
    pub request_id: [u8; 8],
    pub sender_id: NodeId,
    pub target: NodeId,
}

/// Response with up to K closest nodes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FindNodeResp {
    pub request_id: [u8; 8],
    pub sender_id: NodeId,
    pub closest: Vec<NodeInfo>,
}

// ── FIND_VALUE ───────────────────────────────────────────────────────

/// Request a value stored in the DHT by key.
/// If the node doesn't hold it, it returns the K closest nodes instead.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FindValue {
    pub request_id: [u8; 8],
    pub sender_id: NodeId,
    /// `blake3` hash of the application-level key.
    pub key: [u8; 32],
}

/// Response to `FindValue`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FindValueResp {
    Found {
        request_id: [u8; 8],
        /// Always E2EE-encrypted.
        value: Vec<u8>,
    },
    Closest {
        request_id: [u8; 8],
        nodes: Vec<NodeInfo>,
    },
}

// ── STORE ────────────────────────────────────────────────────────────

/// Ask a node to store a key/value pair.
///
/// The signature prevents a malicious node from publishing values
/// on behalf of another identity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Store {
    pub request_id: [u8; 8],
    pub sender_id: NodeId,
    pub key: [u8; 32],
    /// E2EE-encrypted value.
    pub value: Vec<u8>,
    /// Ed25519 signature over `key || value || ts`.
    #[serde(with = "serde_bytes64")]
    pub sig: [u8; 64],
    /// Non-authoritative timestamp hint.
    pub ts: u64,
    /// Seconds before expiration (max T_EXPIRE = 48h).
    pub ttl: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoreResp {
    pub request_id: [u8; 8],
    pub accepted: bool,
}

// ── BOOTSTRAP ────────────────────────────────────────────────────────

/// Sent to a rendezvous node to request initial DHT peers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BootstrapReq {
    pub request_id: [u8; 8],
    pub sender_id: NodeId,
    pub node_type: NodeType,
}

/// Response from a rendezvous node with up to 20 peers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BootstrapResp {
    pub request_id: [u8; 8],
    pub peers: Vec<NodeInfo>,
}

// ── RENDEZVOUS ───────────────────────────────────────────────────────

/// A relay registers itself on a rendezvous node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RendezvousRegister {
    pub request_id: [u8; 8],
    pub domain: String,
    pub addr: String,
    pub pubkey: [u8; 32],
    pub node_type: NodeType,
    pub ttl: u32,
    #[serde(with = "serde_bytes64")]
    pub sig: [u8; 64],
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RendezvousRegisterResp {
    pub request_id: [u8; 8],
    pub accepted: bool,
}

/// Lookup a domain on a rendezvous node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RendezvousLookup {
    pub request_id: [u8; 8],
    pub domain: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RendezvousLookupResp {
    pub request_id: [u8; 8],
    pub found: bool,
    pub addr: Option<String>,
    pub pubkey: Option<[u8; 32]>,
}

// ── DHT timing constants ─────────────────────────────────────────────

/// Refresh interval for inactive buckets.
pub const T_REFRESH_SECS: u64 = 3600; // 1 hour

/// Republish interval for stored values.
pub const T_REPUBLISH_SECS: u64 = 86400; // 24 hours

/// Maximum lifetime of a value in the DHT.
pub const T_EXPIRE_SECS: u64 = 172800; // 48 hours

/// Lookup request timeout.
pub const LOOKUP_TIMEOUT_SECS: u64 = 5;

/// Minimum confirmed peers before bootstrap is considered done.
pub const BOOTSTRAP_THRESHOLD: usize = 8;
