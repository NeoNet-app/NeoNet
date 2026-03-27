use chacha20poly1305::{
    ChaCha20Poly1305, KeyInit, AeadCore,
    aead::{Aead, OsRng, generic_array::GenericArray},
};

#[derive(Debug)]
pub enum AeadError {
    Encrypt,
    Decrypt,
}

impl std::fmt::Display for AeadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Encrypt => write!(f, "encryption failed"),
            Self::Decrypt => write!(f, "decryption failed"),
        }
    }
}

impl std::error::Error for AeadError {}

/// Encrypts `plaintext` with a 256-bit `key`. Returns `nonce || ciphertext`.
pub fn seal(key: &[u8; 32], plaintext: &[u8]) -> Result<Vec<u8>, AeadError> {
    let cipher = ChaCha20Poly1305::new(GenericArray::from_slice(key));
    let nonce = ChaCha20Poly1305::generate_nonce(&mut OsRng);
    let ciphertext = cipher.encrypt(&nonce, plaintext).map_err(|_| AeadError::Encrypt)?;
    let mut out = nonce.to_vec();
    out.extend_from_slice(&ciphertext);
    Ok(out)
}

/// Decrypts data produced by `seal`. Expects `nonce (12 bytes) || ciphertext`.
pub fn open(key: &[u8; 32], data: &[u8]) -> Result<Vec<u8>, AeadError> {
    if data.len() < 12 {
        return Err(AeadError::Decrypt);
    }
    let (nonce_bytes, ciphertext) = data.split_at(12);
    let cipher = ChaCha20Poly1305::new(GenericArray::from_slice(key));
    let nonce = GenericArray::from_slice(nonce_bytes);
    cipher.decrypt(nonce, ciphertext).map_err(|_| AeadError::Decrypt)
}

/// Encrypt with an explicit nonce (for session frames with incremental counter).
pub fn seal_with_nonce(key: &[u8; 32], nonce: &[u8; 12], plaintext: &[u8]) -> Result<Vec<u8>, AeadError> {
    let cipher = ChaCha20Poly1305::new(GenericArray::from_slice(key));
    let n = GenericArray::from_slice(nonce);
    cipher.encrypt(n, plaintext).map_err(|_| AeadError::Encrypt)
}

/// Decrypt with an explicit nonce (for session frames with incremental counter).
pub fn open_with_nonce(key: &[u8; 32], nonce: &[u8; 12], ciphertext: &[u8]) -> Result<Vec<u8>, AeadError> {
    let cipher = ChaCha20Poly1305::new(GenericArray::from_slice(key));
    let n = GenericArray::from_slice(nonce);
    cipher.decrypt(n, ciphertext).map_err(|_| AeadError::Decrypt)
}
