use clap::{Parser, Subcommand};

/// NeoNet — Decentralized, privacy-first communication protocol.
#[derive(Parser)]
#[command(name = "neonet", version, about)]
pub struct Cli {
    /// Path to config.toml
    #[arg(long, global = true)]
    pub config: Option<String>,

    /// Data directory (default: ~/.neonet/)
    #[arg(long, global = true)]
    pub data_dir: Option<String>,

    /// URL of the local API (default: http://127.0.0.1:7780)
    #[arg(long, global = true)]
    pub api_url: Option<String>,

    /// Output JSON instead of human-readable text
    #[arg(long, global = true)]
    pub json: bool,

    /// Suppress non-essential output
    #[arg(long, global = true)]
    pub quiet: bool,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Create identity and initial configuration.
    Init {
        /// Associate identity with a domain.
        #[arg(long)]
        domain: Option<String>,

        /// Read passphrase from NEONET_PASSPHRASE env var.
        #[arg(long)]
        passphrase_env: bool,
    },

    /// Start the daemon (foreground by default).
    Start {
        /// Fork to background.
        #[arg(long)]
        daemon: bool,

        /// Node mode: full, relay, rendezvous, client.
        #[arg(long)]
        mode: Option<String>,

        /// Public domain (relay mode).
        #[arg(long)]
        domain: Option<String>,

        /// API port (overrides config).
        #[arg(long)]
        api_port: Option<u16>,

        /// QUIC listen port (overrides config).
        #[arg(long)]
        listen_port: Option<u16>,

        /// Rendezvous address(es) (host:port, comma-separated).
        #[arg(long)]
        rendezvous: Option<String>,

        /// Relay address (client mode, host:port).
        #[arg(long)]
        relay: Option<String>,

        /// Read passphrase from NEONET_PASSPHRASE env var.
        #[arg(long)]
        passphrase_env: bool,

        /// Log level (error, warn, info, debug, trace).
        #[arg(long)]
        log_level: Option<String>,

        /// Path to config.toml.
        #[arg(long)]
        config_path: Option<String>,
    },

    /// Stop the daemon.
    Stop,

    /// Show daemon status.
    Status,

    /// Identity management.
    Identity {
        #[command(subcommand)]
        action: IdentityAction,
    },

    /// Peer and DHT inspection.
    Peers {
        #[command(subcommand)]
        action: PeersAction,
    },

    /// Room management.
    Rooms {
        #[command(subcommand)]
        action: RoomsAction,
    },

    /// Rendezvous list management.
    Rendezvous {
        #[command(subcommand)]
        action: RendezvousAction,
    },

    /// Show daemon logs.
    Logs {
        /// Filter by level.
        #[arg(long)]
        level: Option<String>,

        /// Show logs since duration (e.g. "1h").
        #[arg(long)]
        since: Option<String>,

        /// Don't follow (print and exit).
        #[arg(long)]
        no_follow: bool,
    },

    /// Configuration management.
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },
}

// ── Identity subcommands ─────────────────────────────────────────────

#[derive(Subcommand)]
pub enum IdentityAction {
    /// Show public key and address.
    Show,
    /// Export public key.
    Export,
    /// Sign a hex-encoded payload (debug).
    Sign {
        /// Hex-encoded payload.
        payload: String,
    },
}

// ── Peers subcommands ────────────────────────────────────────────────

#[derive(Subcommand)]
pub enum PeersAction {
    /// List connected peers.
    List,
    /// Connect to a peer via QUIC and send PING/PONG.
    Connect {
        /// Address (host:port) or domain (resolved via rendezvous).
        addr: String,
        /// Rendezvous server to use for domain lookup (host:port).
        #[arg(long)]
        rendezvous: Option<String>,
    },
    /// Ping a NeoNet node.
    Ping {
        /// Address (host:port).
        addr: String,
    },
    /// Lookup a domain on a rendezvous server.
    Find {
        /// Domain or hex node ID.
        node_id: String,
        /// Rendezvous server (host:port).
        #[arg(long)]
        rendezvous: Option<String>,
    },
    /// Show rendezvous status.
    Rendezvous,
}

// ── Rooms subcommands ────────────────────────────────────────────────

#[derive(Subcommand)]
pub enum RoomsAction {
    /// List rooms.
    List,
    /// Create a room.
    Create {
        /// Room name.
        #[arg(long)]
        name: String,
        /// Comma-separated member addresses.
        #[arg(long)]
        members: Option<String>,
    },
    /// Join via invitation link.
    Join {
        /// neonet://join/... URL.
        url: String,
    },
    /// Show room details.
    Info {
        /// Room ID (base64url).
        room_id: String,
    },
    /// Leave a room.
    Leave {
        /// Room ID (base64url).
        room_id: String,
    },
}

// ── Rendezvous subcommands ───────────────────────────────────────────

#[derive(Subcommand)]
pub enum RendezvousAction {
    /// List configured rendezvous.
    List,
    /// Add a rendezvous URL.
    Add {
        /// URL of the rendezvous list.
        url: String,
    },
    /// Remove a rendezvous URL.
    Remove {
        /// URL to remove.
        url: String,
    },
    /// Verify signature of a rendezvous list.
    Verify {
        /// URL to verify.
        url: String,
    },
    /// Serve /.neonet/rendezvous.toml.
    Serve {
        /// Port to listen on.
        #[arg(long, default_value = "8080")]
        port: u16,
        /// Human-readable list name.
        #[arg(long)]
        name: Option<String>,
    },
}

// ── Config subcommands ───────────────────────────────────────────────

#[derive(Subcommand)]
pub enum ConfigAction {
    /// Show current configuration.
    Show,
    /// Set a config value.
    Set {
        /// Key (e.g. "daemon.api_port").
        key: String,
        /// Value.
        value: String,
    },
    /// Get a config value.
    Get {
        /// Key (e.g. "network.listen_port").
        key: String,
    },
}
