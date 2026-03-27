use neonet_core::{Event, CrdtState};

#[derive(Debug)]
pub enum CodecError {
    Serialize(postcard::Error),
    Deserialize(postcard::Error),
}

impl std::fmt::Display for CodecError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Serialize(e) => write!(f, "serialize: {e}"),
            Self::Deserialize(e) => write!(f, "deserialize: {e}"),
        }
    }
}

impl std::error::Error for CodecError {}

/// Serialize an `Event` to compact binary (postcard).
pub fn encode_event(event: &Event) -> Result<Vec<u8>, CodecError> {
    postcard::to_allocvec(event).map_err(CodecError::Serialize)
}

/// Deserialize an `Event` from binary.
pub fn decode_event(data: &[u8]) -> Result<Event, CodecError> {
    postcard::from_bytes(data).map_err(CodecError::Deserialize)
}

/// Serialize a `CrdtState` to compact binary.
pub fn encode_crdt_state(state: &CrdtState) -> Result<Vec<u8>, CodecError> {
    postcard::to_allocvec(state).map_err(CodecError::Serialize)
}

/// Deserialize a `CrdtState` from binary.
pub fn decode_crdt_state(data: &[u8]) -> Result<CrdtState, CodecError> {
    postcard::from_bytes(data).map_err(CodecError::Deserialize)
}
