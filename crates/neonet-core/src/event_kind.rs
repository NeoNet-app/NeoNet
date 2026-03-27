use serde::{Deserialize, Serialize};

/// Event kinds encoded on 16 bits.
/// `0x0001..0x00FF`: DAG events (immutable)
/// `0x0100..0x01FF`: CRDT state events
/// `0xF000+`: third-party extensions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u16)]
pub enum EventKind {
    // DAG events
    Message = 0x0001,
    ThreadReply = 0x0002,
    Reaction = 0x0003,
    Edit = 0x0010,
    Redact = 0x0011,
    FileRef = 0x0200,

    // CRDT state events
    RoomState = 0x0100,
    Presence = 0x0101,
    Membership = 0x0102,
}
