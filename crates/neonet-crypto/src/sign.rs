use serde::{Deserialize, Serialize};

pub use ed25519_dalek::Signature;

pub struct SigningKey(ed25519_dalek::SigningKey);

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerifyingKey(pub [u8; 32]);

impl SigningKey {
    pub fn generate() -> Self {
        use rand::rngs::OsRng;
        let mut csprng = OsRng;
        Self(ed25519_dalek::SigningKey::generate(&mut csprng))
    }

    /// Reconstruct a signing key from raw 32-byte secret.
    pub fn from_bytes(bytes: &[u8; 32]) -> Self {
        Self(ed25519_dalek::SigningKey::from_bytes(bytes))
    }

    /// Return the raw 32-byte secret key material.
    ///
    /// Callers must zeroize the returned bytes when done.
    pub fn secret_bytes(&self) -> [u8; 32] {
        self.0.to_bytes()
    }

    pub fn verifying_key(&self) -> VerifyingKey {
        VerifyingKey(self.0.verifying_key().to_bytes())
    }

    pub fn sign(&self, message: &[u8]) -> Signature {
        use ed25519_dalek::Signer;
        self.0.sign(message)
    }
}

impl VerifyingKey {
    pub fn verify(&self, message: &[u8], signature: &Signature) -> bool {
        use ed25519_dalek::Verifier;
        let Ok(key) = ed25519_dalek::VerifyingKey::from_bytes(&self.0) else {
            return false;
        };
        key.verify(message, signature).is_ok()
    }

    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl std::fmt::Debug for VerifyingKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let hex: String = self.0.iter().map(|b| format!("{b:02x}")).collect();
        write!(f, "VerifyingKey({hex})")
    }
}
