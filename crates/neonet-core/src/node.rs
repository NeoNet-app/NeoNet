use std::collections::VecDeque;
use std::net::SocketAddr;
use std::time::{Duration, Instant};

use neonet_crypto::Hash;
use serde::{Deserialize, Serialize};

/// K — maximum entries per bucket (NeoNet Kademlia parameter).
pub const K: usize = 20;

/// Alpha — lookup parallelism.
pub const ALPHA: usize = 3;

/// Number of buckets (one per bit of distance).
pub const NUM_BUCKETS: usize = 160;

// ── NodeId ───────────────────────────────────────────────────────────

/// 160-bit Kademlia node identifier.
///
/// Derived from the node's Ed25519 public key:
/// `node_id = blake3(pubkey_ed25519)[0..20]`
///
/// Anchoring the ID on the public key prevents an attacker from freely
/// choosing an ID to target a DHT zone (basic Sybil resistance).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NodeId(pub [u8; 20]);

impl NodeId {
    /// Derive a `NodeId` from an Ed25519 public key.
    pub fn from_pubkey(pubkey: &[u8; 32]) -> Self {
        let hash = Hash::digest(pubkey);
        let mut id = [0u8; 20];
        id.copy_from_slice(&hash.as_bytes()[..20]);
        Self(id)
    }

    /// XOR distance between two node IDs.
    pub fn distance(&self, other: &NodeId) -> [u8; 20] {
        let mut d = [0u8; 20];
        for i in 0..20 {
            d[i] = self.0[i] ^ other.0[i];
        }
        d
    }

    /// Return the index of the highest-order differing bit (0..159),
    /// used to determine which k-bucket a peer belongs to.
    /// Returns `None` if the IDs are identical.
    pub fn bucket_index(&self, other: &NodeId) -> Option<usize> {
        let dist = self.distance(other);
        for i in 0..20 {
            if dist[i] != 0 {
                let bit = 7 - dist[i].leading_zeros() as usize;
                return Some(i * 8 + bit);
            }
        }
        None
    }

    pub fn as_bytes(&self) -> &[u8; 20] {
        &self.0
    }
}

impl std::fmt::Debug for NodeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let hex: String = self.0.iter().map(|b| format!("{b:02x}")).collect();
        write!(f, "NodeId({hex})")
    }
}

impl std::fmt::Display for NodeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let hex: String = self.0.iter().map(|b| format!("{b:02x}")).collect();
        write!(f, "{hex}")
    }
}

// ── NodeType ─────────────────────────────────────────────────────────

/// Type of NeoNet node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum NodeType {
    /// Federates, routes, and stores state.
    Full = 0,
    /// Bridge between federation and P2P; temporary encrypted cache.
    Relay = 1,
    /// Mobile / browser, P2P direct if available.
    Light = 2,
}

// ── NodeInfo ─────────────────────────────────────────────────────────

/// Information about a known DHT peer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeInfo {
    pub id: NodeId,
    pub addr: SocketAddr,
    /// Ed25519 public key — used to authenticate DHT messages.
    pub pubkey: [u8; 32],
    /// Last time this node was seen alive.
    #[serde(skip)]
    pub last_seen: Option<Instant>,
    /// Measured round-trip time.
    #[serde(skip)]
    pub rtt: Option<Duration>,
    pub node_type: NodeType,
}

// ── KBucket ──────────────────────────────────────────────────────────

/// A single Kademlia k-bucket holding up to K peers.
pub struct KBucket {
    /// Peers ordered by age — oldest at front, newest at back.
    pub nodes: VecDeque<NodeInfo>,
    /// Last time any node in this bucket was accessed.
    pub last_seen: Option<Instant>,
}

impl KBucket {
    pub fn new() -> Self {
        Self {
            nodes: VecDeque::new(),
            last_seen: None,
        }
    }

    pub fn is_full(&self) -> bool {
        self.nodes.len() >= K
    }

    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }
}

impl Default for KBucket {
    fn default() -> Self {
        Self::new()
    }
}

// ── RoutingTable ─────────────────────────────────────────────────────

/// Kademlia routing table: 160 k-buckets indexed by XOR distance bit.
pub struct RoutingTable {
    pub local_id: NodeId,
    pub buckets: Vec<KBucket>,
}

impl RoutingTable {
    pub fn new(local_id: NodeId) -> Self {
        let buckets = (0..NUM_BUCKETS).map(|_| KBucket::new()).collect();
        Self { local_id, buckets }
    }

    /// Total number of known peers across all buckets.
    pub fn len(&self) -> usize {
        self.buckets.iter().map(|b| b.len()).sum()
    }

    pub fn is_empty(&self) -> bool {
        self.buckets.iter().all(|b| b.is_empty())
    }
}
