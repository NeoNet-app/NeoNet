use chrono::{DateTime, Utc};
use neonet_crypto::{VerifyingKey, parse_verifying_key, parse_signature, encode_signature};
use serde::{Deserialize, Serialize};
use thiserror::Error;

// ── Types ────────────────────────────────────────────────────────────

/// A parsed `rendezvous.toml` file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RendezvousList {
    pub meta: Meta,
    pub nodes: Vec<Node>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Meta {
    pub spec: u32,
    pub name: String,
    pub maintainer: String,
    #[serde(default)]
    pub created_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub updated_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub expires_at: Option<DateTime<Utc>>,
    /// `"ed25519:BASE64URL"` or `""` (blank before signing).
    #[serde(default)]
    pub sig: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    pub addr: String,
    /// `"ed25519:BASE64URL"`.
    pub pubkey: String,
    #[serde(default)]
    pub region: String,
    #[serde(default)]
    pub tor: bool,
}

// ── Errors ───────────────────────────────────────────────────────────

#[derive(Debug, Error)]
pub enum RendezvousError {
    #[error("TOML parse error: {0}")]
    Toml(#[from] toml::de::Error),

    #[error("encoding error: {0}")]
    Encoding(#[from] neonet_crypto::EncodingError),

    #[error("signature verification failed")]
    BadSignature,

    #[error("list expired at {0}")]
    Expired(DateTime<Utc>),

    #[error("unsupported spec version: {0}")]
    UnsupportedSpec(u32),

    #[error("HTTP fetch error: {0}")]
    Fetch(#[from] reqwest::Error),
}

// ── Parsing ──────────────────────────────────────────────────────────

impl RendezvousList {
    /// Parse a `rendezvous.toml` string.
    pub fn parse(toml_str: &str) -> Result<Self, RendezvousError> {
        let list: Self = toml::from_str(toml_str)?;
        if list.meta.spec != 1 {
            return Err(RendezvousError::UnsupportedSpec(list.meta.spec));
        }
        Ok(list)
    }

    /// Verify the Ed25519 signature of the file.
    ///
    /// Per spec: reconstruct the file with `meta.sig = ""`, then verify
    /// with the maintainer's public key.
    pub fn verify_signature(
        &self,
        raw_toml: &str,
        maintainer_key: &VerifyingKey,
    ) -> Result<(), RendezvousError> {
        let sig = parse_signature(&self.meta.sig)?;

        // Reconstruct the file with sig blanked out for verification.
        let canonical = blank_signature(raw_toml, &self.meta.sig);

        if !maintainer_key.verify(canonical.as_bytes(), &sig) {
            return Err(RendezvousError::BadSignature);
        }
        Ok(())
    }

    /// Check that the list has not expired.
    pub fn verify_expiry(&self) -> Result<(), RendezvousError> {
        if let Some(expires) = self.meta.expires_at {
            if Utc::now() > expires {
                return Err(RendezvousError::Expired(expires));
            }
        }
        Ok(())
    }

    /// Full validation: parse + verify signature + check expiry.
    pub fn parse_and_verify(
        toml_str: &str,
        maintainer_key: &VerifyingKey,
    ) -> Result<Self, RendezvousError> {
        let list = Self::parse(toml_str)?;
        list.verify_signature(toml_str, maintainer_key)?;
        list.verify_expiry()?;
        Ok(list)
    }

    /// Parse the pubkey of each node into a `VerifyingKey`.
    pub fn node_keys(&self) -> Result<Vec<VerifyingKey>, neonet_crypto::EncodingError> {
        self.nodes
            .iter()
            .map(|n| parse_verifying_key(&n.pubkey))
            .collect()
    }

    /// Sign this list in-place, setting `meta.sig`.
    /// The caller must serialize to TOML *after* calling this.
    pub fn sign(&mut self, signing_key: &neonet_crypto::SigningKey) {
        // 1. Blank the sig, serialize to canonical TOML.
        self.meta.sig = String::new();
        let canonical = toml::to_string(self).expect("RendezvousList is always serializable");

        // 2. Sign the canonical bytes.
        let sig = signing_key.sign(canonical.as_bytes());
        self.meta.sig = encode_signature(&sig);
    }

    /// Serialize to TOML string.
    pub fn to_toml(&self) -> String {
        toml::to_string(self).expect("RendezvousList is always serializable")
    }
}

// ── Fetcher ──────────────────────────────────────────────────────────

/// Fetch a rendezvous list from an HTTPS URL.
///
/// Returns the raw TOML string and the parsed (but not yet verified) list.
/// Callers should verify signature and expiry themselves, since they hold
/// the trusted maintainer key.
pub async fn fetch_rendezvous(url: &str) -> Result<(String, RendezvousList), RendezvousError> {
    let body = reqwest::get(url).await?.text().await?;
    let list = RendezvousList::parse(&body)?;
    Ok((body, list))
}

/// Fetch and fully verify a rendezvous list from an HTTPS URL.
pub async fn fetch_and_verify(
    url: &str,
    maintainer_key: &VerifyingKey,
) -> Result<RendezvousList, RendezvousError> {
    let (raw, list) = fetch_rendezvous(url).await?;
    list.verify_signature(&raw, maintainer_key)?;
    list.verify_expiry()?;
    Ok(list)
}

// ── Helpers ──────────────────────────────────────────────────────────

/// Replace the `sig = "ed25519:..."` value with `sig = ""` in the raw TOML
/// so the signature can be verified against the canonical content.
fn blank_signature(raw_toml: &str, sig_value: &str) -> String {
    raw_toml.replace(&format!("sig = \"{sig_value}\""), "sig = \"\"")
}

// ── Tests ────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use neonet_crypto::{SigningKey, encode_verifying_key, encode_prefixed};

    fn make_signed_list() -> (String, VerifyingKey) {
        let sk = SigningKey::generate();
        let vk = sk.verifying_key();

        let node_sk = SigningKey::generate();
        let node_vk = node_sk.verifying_key();

        let mut list = RendezvousList {
            meta: Meta {
                spec: 1,
                name: "Test list".into(),
                maintainer: format!("@{}:test.com", encode_verifying_key(&vk)),
                created_at: None,
                updated_at: None,
                expires_at: Some(Utc::now() + chrono::Duration::hours(24)),
                sig: String::new(),
            },
            nodes: vec![Node {
                addr: "node1.test.com:7777".into(),
                pubkey: encode_prefixed(node_vk.as_bytes()),
                region: "eu-west".into(),
                tor: false,
            }],
        };

        list.sign(&sk);
        (list.to_toml(), vk)
    }

    #[test]
    fn parse_roundtrip() {
        let (toml_str, vk) = make_signed_list();
        let result = RendezvousList::parse_and_verify(&toml_str, &vk);
        assert!(result.is_ok(), "verify failed: {result:?}");
    }

    #[test]
    fn bad_signature_rejected() {
        let (toml_str, _) = make_signed_list();
        // Use a different key for verification — must fail.
        let wrong_vk = SigningKey::generate().verifying_key();
        let result = RendezvousList::parse_and_verify(&toml_str, &wrong_vk);
        assert!(matches!(result, Err(RendezvousError::BadSignature)));
    }

    #[test]
    fn expired_list_rejected() {
        let sk = SigningKey::generate();
        let vk = sk.verifying_key();

        let mut list = RendezvousList {
            meta: Meta {
                spec: 1,
                name: "Expired".into(),
                maintainer: "test".into(),
                created_at: None,
                updated_at: None,
                expires_at: Some(Utc::now() - chrono::Duration::hours(1)),
                sig: String::new(),
            },
            nodes: vec![],
        };
        list.sign(&sk);
        let toml_str = list.to_toml();

        let result = RendezvousList::parse_and_verify(&toml_str, &vk);
        assert!(matches!(result, Err(RendezvousError::Expired(_))));
    }
}
