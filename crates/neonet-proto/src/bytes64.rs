/// Serde helper for `[u8; 64]` — serializes as a byte sequence.
///
/// serde only derives Serialize/Deserialize for arrays up to 32 elements.
/// This module provides the missing impl for 64-byte arrays (Ed25519 signatures).
pub mod serde_bytes64 {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S: Serializer>(bytes: &[u8; 64], s: S) -> Result<S::Ok, S::Error> {
        // Serialize as a fixed-length byte slice so postcard stays compact.
        bytes.as_slice().serialize(s)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<[u8; 64], D::Error> {
        let v: Vec<u8> = Vec::deserialize(d)?;
        v.try_into()
            .map_err(|v: Vec<u8>| serde::de::Error::invalid_length(v.len(), &"64 bytes"))
    }
}
