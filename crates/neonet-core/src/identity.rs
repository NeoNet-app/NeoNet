use neonet_crypto::VerifyingKey;
use serde::{Deserialize, Serialize};

/// Identity format: `@pubkey:domain`
///
/// The public key is Ed25519. The domain is a discovery hint only —
/// the identity remains valid even if the domain disappears.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Identity {
    pub pubkey: VerifyingKey,
    pub domain: Option<String>,
}

impl Identity {
    pub fn new(pubkey: VerifyingKey, domain: impl Into<String>) -> Self {
        Self {
            pubkey,
            domain: Some(domain.into()),
        }
    }

    pub fn anonymous(pubkey: VerifyingKey) -> Self {
        Self {
            pubkey,
            domain: None,
        }
    }
}

impl std::fmt::Display for Identity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let hex: String = self.pubkey.as_bytes().iter().map(|b| format!("{b:02x}")).collect();
        match &self.domain {
            Some(d) => write!(f, "@{hex}:{d}"),
            None => write!(f, "@{hex}"),
        }
    }
}
