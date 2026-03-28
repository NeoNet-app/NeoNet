use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// Top-level daemon configuration (`~/.neonet/config.toml`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "default_daemon")]
    pub daemon: DaemonConfig,
    #[serde(default = "default_identity")]
    pub identity: IdentityConfig,
    #[serde(default = "default_network")]
    pub network: NetworkConfig,
    #[serde(default = "default_storage")]
    pub storage: StorageConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaemonConfig {
    #[serde(default = "default_api_port")]
    pub api_port: u16,
    #[serde(default = "default_api_host")]
    pub api_host: String,
    #[serde(default = "default_log_level")]
    pub log_level: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdentityConfig {
    #[serde(default = "default_keystore")]
    pub keystore: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    #[serde(default = "default_mode")]
    pub mode: String,
    #[serde(default)]
    pub domain: String,
    #[serde(default = "default_listen_port")]
    pub listen_port: u16,
    #[serde(default)]
    pub rendezvous: Vec<String>,
    #[serde(default)]
    pub relay: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    #[serde(default = "default_db_path")]
    pub db_path: String,
    #[serde(default = "default_max_db_size")]
    pub max_db_size: String,
}

// ── Defaults ─────────────────────────────────────────────────────────

fn default_daemon() -> DaemonConfig {
    DaemonConfig {
        api_port: default_api_port(),
        api_host: default_api_host(),
        log_level: default_log_level(),
    }
}

fn default_identity() -> IdentityConfig {
    IdentityConfig {
        keystore: default_keystore(),
    }
}

fn default_network() -> NetworkConfig {
    NetworkConfig {
        mode: default_mode(),
        domain: String::new(),
        listen_port: default_listen_port(),
        rendezvous: Vec::new(),
        relay: String::new(),
    }
}

fn default_mode() -> String {
    "full".into()
}

fn default_storage() -> StorageConfig {
    StorageConfig {
        db_path: default_db_path(),
        max_db_size: default_max_db_size(),
    }
}

fn default_api_port() -> u16 {
    7780
}
fn default_api_host() -> String {
    "127.0.0.1".into()
}
fn default_log_level() -> String {
    "info".into()
}
fn default_keystore() -> String {
    "~/.neonet/keystore".into()
}
fn default_listen_port() -> u16 {
    7777
}
fn default_db_path() -> String {
    "~/.neonet/neonet.db".into()
}
fn default_max_db_size() -> String {
    "10GB".into()
}

// ── Loading ──────────────────────────────────────────────────────────

impl Config {
    /// NeoNet base directory.
    pub fn base_dir() -> PathBuf {
        dirs_home().join(".neonet")
    }

    /// Load config from `~/.neonet/config.toml`, creating defaults if absent.
    pub fn load() -> Result<Self, ConfigError> {
        let path = Self::base_dir().join("config.toml");
        Self::load_from(&path)
    }

    pub fn load_from(path: &Path) -> Result<Self, ConfigError> {
        if path.exists() {
            let content = std::fs::read_to_string(path)?;
            let config: Config = toml::from_str(&content)?;
            Ok(config)
        } else {
            // Return defaults — caller can write them out if desired.
            Ok(Config {
                daemon: default_daemon(),
                identity: default_identity(),
                network: default_network(),
                storage: default_storage(),
            })
        }
    }

    /// Session token file path.
    pub fn session_token_path() -> PathBuf {
        Self::base_dir().join("session.token")
    }

    /// Daemon PID file path.
    pub fn pid_path() -> PathBuf {
        Self::base_dir().join("daemon.pid")
    }

    /// Daemon log file path.
    pub fn log_path() -> PathBuf {
        Self::base_dir().join("daemon.log")
    }
}

fn dirs_home() -> PathBuf {
    std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."))
}

// ── Errors ───────────────────────────────────────────────────────────

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("TOML parse error: {0}")]
    Toml(#[from] toml::de::Error),
}
