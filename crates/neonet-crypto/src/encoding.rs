use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};

use crate::sign::{Signature, VerifyingKey};

const ED25519_PREFIX: &str = "ed25519:";

/// Parse an `"ed25519:BASE64URL"` string into raw bytes.
pub fn decode_prefixed(s: &str) -> Result<Vec<u8>, EncodingError> {
    let b64 = s
        .strip_prefix(ED25519_PREFIX)
        .ok_or(EncodingError::MissingPrefix)?;
    URL_SAFE_NO_PAD
        .decode(b64)
        .map_err(EncodingError::Base64)
}

/// Encode raw bytes as `"ed25519:BASE64URL"`.
pub fn encode_prefixed(bytes: &[u8]) -> String {
    format!("{ED25519_PREFIX}{}", URL_SAFE_NO_PAD.encode(bytes))
}

/// Parse `"ed25519:BASE64URL"` into a `VerifyingKey`.
pub fn parse_verifying_key(s: &str) -> Result<VerifyingKey, EncodingError> {
    let bytes = decode_prefixed(s)?;
    let arr: [u8; 32] = bytes
        .try_into()
        .map_err(|_| EncodingError::InvalidKeyLength)?;
    Ok(VerifyingKey(arr))
}

/// Encode a `VerifyingKey` as `"ed25519:BASE64URL"`.
pub fn encode_verifying_key(key: &VerifyingKey) -> String {
    encode_prefixed(key.as_bytes())
}

/// Parse `"ed25519:BASE64URL"` into an Ed25519 `Signature`.
pub fn parse_signature(s: &str) -> Result<Signature, EncodingError> {
    let bytes = decode_prefixed(s)?;
    let arr: [u8; 64] = bytes
        .try_into()
        .map_err(|_| EncodingError::InvalidSignatureLength)?;
    Ok(Signature::from_bytes(&arr))
}

/// Encode a `Signature` as `"ed25519:BASE64URL"`.
pub fn encode_signature(sig: &Signature) -> String {
    encode_prefixed(&sig.to_bytes())
}

#[derive(Debug)]
pub enum EncodingError {
    MissingPrefix,
    Base64(base64::DecodeError),
    InvalidKeyLength,
    InvalidSignatureLength,
}

impl std::fmt::Display for EncodingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingPrefix => write!(f, "missing 'ed25519:' prefix"),
            Self::Base64(e) => write!(f, "base64url decode: {e}"),
            Self::InvalidKeyLength => write!(f, "expected 32-byte key"),
            Self::InvalidSignatureLength => write!(f, "expected 64-byte signature"),
        }
    }
}

impl std::error::Error for EncodingError {}
