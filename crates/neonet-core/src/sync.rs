use std::collections::{HashMap, HashSet};
use std::time::Instant;

use bloomfilter::Bloom;
use serde::{Deserialize, Serialize};

use crate::event::Event;
use crate::room::RoomId;
use neonet_crypto::Hash;

/// Alias — an event ID is a blake3 Hash.
pub type EventId = Hash;

// ── Sync parameters ──────────────────────────────────────────────────

/// Maximum events per SyncBatch message.
pub const MAX_BATCH_SIZE: usize = 256;

/// Bloom filter false-positive rate.
pub const BLOOM_FP_RATE: f64 = 0.01;

/// Global sync session timeout.
pub const SYNC_TIMEOUT_SECS: u64 = 30;

/// Pending events threshold before triggering a forced full sync.
pub const MAX_PENDING_EVENTS: usize = 1000;

/// Number of random peers to propagate a new event to.
pub const GOSSIP_FANOUT: usize = 3;

// ── EventState ───────────────────────────────────────────────────────

/// Validation state of a stored event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EventState {
    /// Received but missing parents.
    Pending,
    /// Parents known and signature verified.
    Valid,
    /// Signature invalid or ACL violation — silently ignored.
    Invalid,
}

// ── StoredEvent ──────────────────────────────────────────────────────

/// A DAG event together with local bookkeeping metadata.
#[derive(Debug, Clone)]
pub struct StoredEvent {
    /// The event as received on the wire.
    pub event: Event,
    /// When this node first received the event (local only).
    pub received_at: Instant,
    /// Known successor event IDs.
    pub children: HashSet<EventId>,
    /// Validation state.
    pub state: EventState,
}

// ── EventGraph ───────────────────────────────────────────────────────

/// In-memory DAG of events for a single room.
pub struct EventGraph {
    pub room_id: RoomId,
    /// All known events indexed by ID.
    pub events: HashMap<EventId, StoredEvent>,
    /// Tips — events with no known successors (DAG heads).
    pub tips: HashSet<EventId>,
    /// Root events (genesis, no parents).
    pub roots: HashSet<EventId>,
}

impl EventGraph {
    pub fn new(room_id: RoomId) -> Self {
        Self {
            room_id,
            events: HashMap::new(),
            tips: HashSet::new(),
            roots: HashSet::new(),
        }
    }

    /// Current tips — used as parents for new events.
    pub fn current_tips(&self) -> Vec<EventId> {
        self.tips.iter().copied().collect()
    }

    /// Total number of events.
    pub fn len(&self) -> usize {
        self.events.len()
    }

    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }
}

// ── Sync protocol messages ───────────────────────────────────────────

/// Phase 1: compact summary of a peer's DAG state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncSummary {
    pub room_id: RoomId,
    pub tip_ids: Vec<EventId>,
    pub event_count: u64,
    /// Bloom filter over all known event IDs for fast set-difference.
    #[serde(with = "bloom_serde")]
    pub have_set: Bloom<EventId>,
}

/// Phase 2: list of event IDs a peer wants.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncWant {
    pub room_id: RoomId,
    pub event_ids: Vec<EventId>,
}

/// Phase 2: list of event IDs a peer has and the other doesn't.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncHave {
    pub room_id: RoomId,
    pub event_ids: Vec<EventId>,
}

/// Phase 3: batch of events in topological order (parents before children).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncBatch {
    pub room_id: RoomId,
    pub events: Vec<Event>,
    /// More batches to follow.
    pub has_more: bool,
}

/// Gossip: propagate a new valid event to random peers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GossipEvent {
    pub event: Event,
}

// ── Bloom filter serde ───────────────────────────────────────────────

/// `Bloom` doesn't implement Serialize/Deserialize, so we go via raw bytes.
mod bloom_serde {
    use bloomfilter::Bloom;
    use neonet_crypto::Hash;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    #[derive(Serialize, Deserialize)]
    struct BloomWire {
        bitmap: Vec<u8>,
        bitmap_bits: u64,
        k_num: u32,
        sip_keys: [(u64, u64); 2],
    }

    pub fn serialize<S: Serializer>(bloom: &Bloom<Hash>, s: S) -> Result<S::Ok, S::Error> {
        let wire = BloomWire {
            bitmap: bloom.bitmap(),
            bitmap_bits: bloom.number_of_bits(),
            k_num: bloom.number_of_hash_functions(),
            sip_keys: bloom.sip_keys(),
        };
        wire.serialize(s)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Bloom<Hash>, D::Error> {
        let wire = BloomWire::deserialize(d)?;
        Ok(Bloom::from_existing(
            &wire.bitmap,
            wire.bitmap_bits,
            wire.k_num,
            wire.sip_keys,
        ))
    }
}
