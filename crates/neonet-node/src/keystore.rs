use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

use argon2::Argon2;
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::Utc;
use neonet_crypto::{SigningKey, VerifyingKey, encode_verifying_key};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use zeroize::Zeroizing;

use crate::config::Config;

// ── Errors ───────────────────────────────────────────────────────────

#[derive(Debug, Error)]
pub enum KeystoreError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("TOML error: {0}")]
    Toml(#[from] toml::de::Error),
    #[error("TOML serialize error: {0}")]
    TomlSer(#[from] toml::ser::Error),
    #[error("Argon2 error: {0}")]
    Argon2(argon2::Error),
    #[error("decryption failed — wrong passphrase?")]
    Decrypt,
    #[error("pubkey mismatch — corrupted keystore")]
    PubkeyMismatch,
    #[error("base64 decode error: {0}")]
    Base64(#[from] base64::DecodeError),
    #[error("keystore already exists at {0}")]
    AlreadyExists(PathBuf),
}

impl From<argon2::Error> for KeystoreError {
    fn from(e: argon2::Error) -> Self {
        Self::Argon2(e)
    }
}

// ── identity.key TOML structure ──────────────────────────────────────

#[derive(Debug, Serialize, Deserialize)]
pub struct IdentityKeyFile {
    pub meta: KeyMeta,
    pub kdf: KdfParams,
    pub cipher: CipherParams,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct KeyMeta {
    pub version: u32,
    pub created_at: String,
    pub pubkey: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct KdfParams {
    pub algorithm: String,
    pub salt: String,
    pub m_cost: u32,
    pub t_cost: u32,
    pub p_cost: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CipherParams {
    pub algorithm: String,
    pub nonce: String,
    pub ciphertext: String,
}

// ── Argon2id parameters ──────────────────────────────────────────────

const ARGON2_M_COST: u32 = 65536; // 64 MB
const ARGON2_T_COST: u32 = 3;
const ARGON2_P_COST: u32 = 1;

// ── Keystore trait ───────────────────────────────────────────────────

/// High-level keystore operations — the private key never leaks.
pub trait Keystore {
    fn pubkey(&self) -> [u8; 32];
    fn sign(&self, payload: &[u8]) -> [u8; 64];
    fn address(&self, domain: Option<&str>) -> String;
}

// ── Unlocked keystore ────────────────────────────────────────────────

/// A keystore whose private key is held in memory (zeroized on drop).
pub struct UnlockedKeystore {
    signing_key: SigningKey,
    verifying_key: VerifyingKey,
}

impl Keystore for UnlockedKeystore {
    fn pubkey(&self) -> [u8; 32] {
        *self.verifying_key.as_bytes()
    }

    fn sign(&self, payload: &[u8]) -> [u8; 64] {
        self.signing_key.sign(payload).to_bytes()
    }

    fn address(&self, domain: Option<&str>) -> String {
        let vk_str = encode_verifying_key(&self.verifying_key);
        // Strip "ed25519:" prefix for the address format.
        let pubkey_part = vk_str.strip_prefix("ed25519:").unwrap_or(&vk_str);
        match domain {
            Some(d) => format!("@{pubkey_part}:{d}"),
            None => format!("@{pubkey_part}"),
        }
    }
}

// ── Keystore directory helpers ───────────────────────────────────────

fn keystore_dir() -> PathBuf {
    Config::base_dir().join("keystore")
}

pub fn identity_key_path() -> PathBuf {
    keystore_dir().join("identity.key")
}

pub fn identity_pub_path() -> PathBuf {
    keystore_dir().join("identity.pub")
}

pub fn keystore_exists() -> bool {
    identity_key_path().exists()
}

// ── Create keystore ──────────────────────────────────────────────────

/// Create a new keystore: generate Ed25519 keypair, encrypt with passphrase.
pub fn create_keystore(passphrase: &[u8]) -> Result<VerifyingKey, KeystoreError> {
    let key_path = identity_key_path();
    if key_path.exists() {
        return Err(KeystoreError::AlreadyExists(key_path));
    }

    let dir = keystore_dir();
    fs::create_dir_all(&dir)?;

    // 1. Generate Ed25519 keypair.
    let signing_key = SigningKey::generate();
    let verifying_key = signing_key.verifying_key();
    let secret = Zeroizing::new(signing_key.secret_bytes());

    // 2. Generate salt (32 bytes).
    let mut salt = [0u8; 32];
    rand::rngs::OsRng.fill_bytes(&mut salt);

    // 3. Derive encryption key via Argon2id.
    let mut enc_key = Zeroizing::new([0u8; 32]);
    let params = argon2::Params::new(ARGON2_M_COST, ARGON2_T_COST, ARGON2_P_COST, Some(32))
        .map_err(KeystoreError::Argon2)?;
    let argon2 = Argon2::new(argon2::Algorithm::Argon2id, argon2::Version::V0x13, params);
    argon2
        .hash_password_into(passphrase, &salt, enc_key.as_mut())
        .map_err(KeystoreError::Argon2)?;

    // 4. Encrypt private key with ChaCha20-Poly1305.
    let encrypted = neonet_crypto::seal(&enc_key, &*secret)
        .map_err(|_| KeystoreError::Decrypt)?;
    // seal() returns nonce(12) || ciphertext+tag
    let nonce = &encrypted[..12];
    let ciphertext = &encrypted[12..];

    // 5. Build TOML structure.
    let key_file = IdentityKeyFile {
        meta: KeyMeta {
            version: 1,
            created_at: Utc::now().to_rfc3339(),
            pubkey: encode_verifying_key(&verifying_key),
        },
        kdf: KdfParams {
            algorithm: "argon2id".into(),
            salt: URL_SAFE_NO_PAD.encode(salt),
            m_cost: ARGON2_M_COST,
            t_cost: ARGON2_T_COST,
            p_cost: ARGON2_P_COST,
        },
        cipher: CipherParams {
            algorithm: "chacha20poly1305".into(),
            nonce: URL_SAFE_NO_PAD.encode(nonce),
            ciphertext: URL_SAFE_NO_PAD.encode(ciphertext),
        },
    };

    // 6. Write identity.key (chmod 600).
    let toml_str = toml::to_string_pretty(&key_file)?;
    fs::write(&key_path, &toml_str)?;
    let mut perms = fs::metadata(&key_path)?.permissions();
    perms.set_mode(0o600);
    fs::set_permissions(&key_path, perms)?;

    // 7. Write identity.pub (chmod 644).
    let pub_path = identity_pub_path();
    fs::write(&pub_path, encode_verifying_key(&verifying_key))?;
    let mut perms = fs::metadata(&pub_path)?.permissions();
    perms.set_mode(0o644);
    fs::set_permissions(&pub_path, perms)?;

    Ok(verifying_key)
}

// ── Unlock keystore ──────────────────────────────────────────────────

/// Unlock the keystore: read identity.key, derive key from passphrase, decrypt.
pub fn unlock_keystore(passphrase: &[u8]) -> Result<UnlockedKeystore, KeystoreError> {
    let key_path = identity_key_path();
    let content = fs::read_to_string(&key_path)?;
    let key_file: IdentityKeyFile = toml::from_str(&content)?;

    // 1. Decode KDF params.
    let salt = URL_SAFE_NO_PAD.decode(&key_file.kdf.salt)?;

    // 2. Derive encryption key.
    let mut enc_key = Zeroizing::new([0u8; 32]);
    let params = argon2::Params::new(
        key_file.kdf.m_cost,
        key_file.kdf.t_cost,
        key_file.kdf.p_cost,
        Some(32),
    )
    .map_err(KeystoreError::Argon2)?;
    let argon2 = Argon2::new(argon2::Algorithm::Argon2id, argon2::Version::V0x13, params);
    argon2
        .hash_password_into(passphrase, &salt, enc_key.as_mut())
        .map_err(KeystoreError::Argon2)?;

    // 3. Decode nonce + ciphertext.
    let nonce = URL_SAFE_NO_PAD.decode(&key_file.cipher.nonce)?;
    let ciphertext = URL_SAFE_NO_PAD.decode(&key_file.cipher.ciphertext)?;
    let mut sealed = Vec::with_capacity(nonce.len() + ciphertext.len());
    sealed.extend_from_slice(&nonce);
    sealed.extend_from_slice(&ciphertext);

    // 4. Decrypt.
    let plaintext = neonet_crypto::open(&enc_key, &sealed)
        .map_err(|_| KeystoreError::Decrypt)?;

    if plaintext.len() != 32 {
        return Err(KeystoreError::Decrypt);
    }
    let mut secret = [0u8; 32];
    secret.copy_from_slice(&plaintext);
    let signing_key = SigningKey::from_bytes(&secret);
    // Zeroize the plaintext copy.
    zeroize::Zeroize::zeroize(&mut secret);

    // 5. Verify pubkey matches.
    let verifying_key = signing_key.verifying_key();
    let expected_pubkey = neonet_crypto::parse_verifying_key(&key_file.meta.pubkey)
        .map_err(|_| KeystoreError::PubkeyMismatch)?;
    if verifying_key != expected_pubkey {
        return Err(KeystoreError::PubkeyMismatch);
    }

    Ok(UnlockedKeystore {
        signing_key,
        verifying_key,
    })
}

// ── Passphrase helpers ───────────────────────────────────────────────

/// Get passphrase from (in priority order): env, or interactive stdin.
pub fn get_passphrase(prompt: &str) -> String {
    if let Ok(pp) = std::env::var("NEONET_PASSPHRASE") {
        return pp;
    }
    rpassword::prompt_password(prompt).unwrap_or_default()
}

/// Get passphrase with confirmation (for init).
pub fn get_passphrase_confirmed() -> Option<String> {
    if let Ok(pp) = std::env::var("NEONET_PASSPHRASE") {
        return Some(pp);
    }
    let p1 = rpassword::prompt_password("Passphrase : ").ok()?;
    let p2 = rpassword::prompt_password("Confirmer la passphrase : ").ok()?;
    if p1 != p2 {
        eprintln!("Les passphrases ne correspondent pas.");
        return None;
    }
    Some(p1)
}

/// Read pubkey from identity.pub file.
pub fn read_public_key() -> Result<String, KeystoreError> {
    let pub_path = identity_pub_path();
    Ok(fs::read_to_string(pub_path)?.trim().to_string())
}
