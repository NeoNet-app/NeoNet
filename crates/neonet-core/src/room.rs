use std::collections::HashMap;

use neonet_crypto::Hash;
use serde::{Deserialize, Serialize};

// ── RoomId ───────────────────────────────────────────────────────────

/// Room identifier — globally unique without central coordination.
///
/// `room_id = blake3(creator_pubkey || nonce || ts_hint)`
pub type RoomId = Hash;

// ── RoomType ─────────────────────────────────────────────────────────

/// Type of a NeoNet room.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum RoomType {
    /// Exactly 2 members, no admin roles.
    DirectMessage = 0,
    /// N members with roles.
    Group = 1,
    /// Potentially public, read-only for guests.
    Channel = 2,
    /// Child room attached to a parent event in another room.
    Thread = 3,
}

// ── Role ─────────────────────────────────────────────────────────────

/// Member role within a room's ACL.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[repr(u8)]
pub enum Role {
    /// Can do everything, including deleting the room.
    Owner = 0,
    /// Can manage members and settings (cannot elevate to Owner).
    Admin = 1,
    /// Can read and write.
    Member = 2,
    /// Read-only.
    Guest = 3,
}

// ── MemberInfo ───────────────────────────────────────────────────────

/// Information about a room member.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemberInfo {
    /// Ed25519 public key.
    pub pubkey: [u8; 32],
    pub role: Role,
    /// Timestamp hint of when the member joined.
    pub joined_at: u64,
    /// Local display name hint (non-authoritative).
    pub display_name: Option<String>,
}

// ── RoomState (CRDT) ─────────────────────────────────────────────────

/// CRDT-based room state — converges automatically across peers.
///
/// Uses LWW maps for metadata/settings/ACL and an OR-Set for members.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomState {
    pub room_id: RoomId,
    /// Room metadata: name, description, avatar_hash, etc.
    pub meta: HashMap<String, Vec<u8>>,
    /// Current members (OR-Set semantics — add/remove handled by ops).
    pub members: Vec<MemberInfo>,
    /// Arbitrary key-value settings.
    pub settings: HashMap<String, Vec<u8>>,
    /// Access control list: pubkey → Role (LWW semantics).
    pub acl: HashMap<[u8; 32], Role>,
}

// ── RoomOp (CRDT operations on RoomState) ────────────────────────────

/// A vector clock snapshot for CRDT ordering.
pub type VClock = HashMap<[u8; 32], u64>;

/// Operation on room state (kind 0x0100–0x0102).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RoomOp {
    SetMeta {
        key: String,
        value: Vec<u8>,
        vclock: VClock,
    },
    AddMember {
        info: MemberInfo,
        vclock: VClock,
    },
    RemoveMember {
        pubkey: [u8; 32],
        vclock: VClock,
    },
    SetRole {
        pubkey: [u8; 32],
        role: Role,
        vclock: VClock,
    },
    SetSetting {
        key: String,
        value: Vec<u8>,
        vclock: VClock,
    },
}

// ── GenesisPayload ───────────────────────────────────────────────────

/// Payload of the genesis event (kind 0x0000, parents = []).
///
/// Every room begins with exactly one genesis event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenesisPayload {
    pub room_id: RoomId,
    /// Ed25519 public key of the creator.
    pub creator: [u8; 32],
    pub room_type: RoomType,
    /// Initial room state (members, settings, ACL).
    pub initial_state: RoomState,
    /// Room key encrypted per-member via ECIES(member.x25519_pubkey, room_key).
    pub encrypted_keys: HashMap<[u8; 32], Vec<u8>>,
}

// ── Presence ─────────────────────────────────────────────────────────

/// Presence status for a member in a room (kind 0x0101, with short TTL).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum PresenceStatus {
    Online = 0,
    Away = 1,
    Offline = 2,
}

/// CRDT presence operation (kind 0x0101).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PresenceOp {
    pub room_id: RoomId,
    pub pubkey: [u8; 32],
    pub status: PresenceStatus,
    /// Timestamp hint of last activity.
    pub last_active: u64,
    /// Seconds before expiration (recommended: 60).
    pub ttl: u32,
}
