use rand::rngs::OsRng;
use x25519_dalek::{EphemeralSecret as X25519Secret, PublicKey};

pub struct EphemeralSecret {
    secret: X25519Secret,
    public: PublicKey,
}

pub struct SharedSecret(pub [u8; 32]);

impl EphemeralSecret {
    pub fn generate() -> Self {
        let secret = X25519Secret::random_from_rng(OsRng);
        let public = PublicKey::from(&secret);
        Self { secret, public }
    }

    pub fn public_key_bytes(&self) -> [u8; 32] {
        self.public.to_bytes()
    }

    pub fn diffie_hellman(self, their_public: &[u8; 32]) -> SharedSecret {
        let their_key = PublicKey::from(*their_public);
        let shared = self.secret.diffie_hellman(&their_key);
        SharedSecret(*shared.as_bytes())
    }
}
