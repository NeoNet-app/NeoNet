use serde::{Deserialize, Serialize};

// ── Frame (encrypted session transport) ──────────────────────────────
//
// After the 4-message handshake, all traffic is wrapped in encrypted
// frames using the derived session keys (ChaCha20-Poly1305).

/// Encrypted frame on the wire.
///
/// The nonce is a 96-bit counter incremented per frame. It must never
/// be reused with the same key. If the counter reaches 2^64 the session
/// must be renegotiated.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Frame {
    /// Total frame length (header included).
    pub len: u32,
    /// ChaCha20-Poly1305 nonce (96 bits, incremental counter).
    pub nonce: [u8; 12],
    /// Encrypted payload + 16-byte Poly1305 tag.
    pub ciphertext: Vec<u8>,
    /// Uniform padding (multiple of 256 bytes).
    pub padding: Vec<u8>,
}

/// Decrypted content of a frame.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FramePayload {
    /// Type of content carried by this frame.
    pub kind: FrameKind,
    /// Serialised inner data (postcard-encoded).
    pub data: Vec<u8>,
}

/// Discriminant for frame content types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum FrameKind {
    /// NeoNet DAG event (cf. 01-concept.md).
    DagEvent = 0x01,
    /// NeoNet CRDT operation.
    CrdtOp = 0x02,
    /// DHT message (cf. 04-dht.md).
    DhtMessage = 0x10,
    /// Keep-alive ping.
    Ping = 0x20,
    /// Keep-alive pong.
    Pong = 0x21,
    /// Bootstrap request to a rendezvous node.
    Bootstrap = 0x30,
    /// Bootstrap response with peer list.
    BootstrapResp = 0x31,

    // ── Sync (cf. 06-sync.md) ──────────────────────────────────────

    /// Full DAG state summary with Bloom filter.
    SyncSummary = 0x40,
    /// Partial summary (with `since` timestamp for large rooms).
    SyncSummaryPartial = 0x41,
    /// Request specific events by ID.
    SyncWant = 0x42,
    /// Advertise available events the peer doesn't have.
    SyncHave = 0x43,
    /// Batch of events in topological order.
    SyncBatch = 0x44,

    /// Gossip: propagate a new valid event to random peers.
    GossipEvent = 0x50,

    // ── Rendezvous ───────────────────────────────────────────────────

    /// Register a relay on a rendezvous node.
    RendezvousRegister = 0x60,
    /// Response to a register request.
    RendezvousRegisterResp = 0x61,
    /// Lookup a domain on a rendezvous node.
    RendezvousLookup = 0x62,
    /// Response to a lookup request.
    RendezvousLookupResp = 0x63,
}
