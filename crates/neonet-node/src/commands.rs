use std::fs;
use std::process::ExitCode;
use std::sync::Arc;

use actix_web::{App, HttpServer, web};
use colored::Colorize;

use crate::api;
use crate::auth::{BearerAuth, SessionToken, generate_session_token};
use crate::cli::*;
use crate::config::Config;
use crate::db;
use crate::keystore;
use crate::quic;
use crate::relay;
use crate::rendezvous_server;
use crate::state::AppState;
use crate::ws;

// ── Exit codes ───────────────────────────────────────────────────────

pub const EXIT_OK: u8 = 0;
pub const EXIT_ERROR: u8 = 1;
pub const EXIT_DAEMON_NOT_RUNNING: u8 = 2;
pub const EXIT_KEYSTORE_MISSING: u8 = 3;
pub const EXIT_BAD_PASSPHRASE: u8 = 4;
pub const EXIT_NETWORK: u8 = 5;

// ── init ─────────────────────────────────────────────────────────────

pub fn cmd_init(domain: Option<String>, _passphrase_env: bool) -> ExitCode {
    println!("{}", "NeoNet — Initialisation".bold());
    println!("{}", "───────────────────────".dimmed());

    if keystore::keystore_exists() {
        eprintln!("{} Keystore already exists at {}", "Error:".red().bold(),
            keystore::identity_key_path().display());
        return ExitCode::from(EXIT_ERROR);
    }

    let domain = domain.or_else(|| {
        eprint!("Domaine (optionnel, ex: mondomaine.com) : ");
        let mut buf = String::new();
        std::io::stdin().read_line(&mut buf).ok()?;
        let d = buf.trim().to_string();
        if d.is_empty() { None } else { Some(d) }
    });

    let passphrase = match keystore::get_passphrase_confirmed() {
        Some(p) if !p.is_empty() => p,
        _ => {
            eprintln!("{} Passphrase required.", "Error:".red().bold());
            return ExitCode::from(EXIT_ERROR);
        }
    };

    println!("\nGénération de la paire de clés Ed25519...");
    let vk = match keystore::create_keystore(passphrase.as_bytes()) {
        Ok(vk) => vk,
        Err(e) => {
            eprintln!("{} {e}", "Error:".red().bold());
            return ExitCode::from(EXIT_ERROR);
        }
    };

    let pubkey_str = neonet_crypto::encode_verifying_key(&vk);
    let pubkey_short = pubkey_str.strip_prefix("ed25519:").unwrap_or(&pubkey_str);
    let address = match &domain {
        Some(d) => format!("@{pubkey_short}:{d}"),
        None => format!("@{pubkey_short}"),
    };

    println!("Keystore créé : {}", keystore::identity_key_path().display().to_string().green());
    println!("Clé publique  : {}", pubkey_str.cyan());
    println!("Adresse NeoNet: {}", address.cyan());

    let config_path = Config::base_dir().join("config.toml");
    if !config_path.exists() {
        let cfg = Config::load().unwrap_or_else(|_| toml::from_str("").unwrap());
        let _ = fs::write(&config_path, toml::to_string_pretty(&cfg).unwrap_or_default());
        println!("\nConfiguration créée : {}", config_path.display().to_string().green());
    }

    println!("\nDémarrez le daemon avec : {}", "neonet start".bold());
    ExitCode::from(EXIT_OK)
}

// ── start (mode-aware) ───────────────────────────────────────────────

pub fn cmd_start(
    _daemon: bool,
    mode: Option<String>,
    domain: Option<String>,
    api_port: Option<u16>,
    listen_port: Option<u16>,
    rendezvous_addrs: Option<String>,
    relay_addr: Option<String>,
    _passphrase_env: bool,
    log_level: Option<String>,
    config_path: Option<String>,
) -> ExitCode {
    let effective_log = log_level
        .or_else(|| std::env::var("NEONET_LOG").ok())
        .unwrap_or_default();
    if !effective_log.is_empty() {
        unsafe { std::env::set_var("RUST_LOG", &effective_log) };
    }
    env_logger::init();

    let mut cfg = match config_path {
        Some(ref p) => Config::load_from(std::path::Path::new(p)),
        None => Config::load(),
    }
    .unwrap_or_else(|e| {
        log::warn!("Failed to load config: {e}, using defaults");
        toml::from_str("").unwrap()
    });

    // CLI overrides (highest priority).
    if let Some(m) = mode { cfg.network.mode = m; }
    if let Some(d) = domain { cfg.network.domain = d; }
    if let Some(p) = api_port { cfg.daemon.api_port = p; }
    if let Some(p) = listen_port { cfg.network.listen_port = p; }
    if let Some(r) = rendezvous_addrs {
        cfg.network.rendezvous = r.split(',').map(|s| s.trim().to_string()).collect();
    }
    if let Some(r) = relay_addr { cfg.network.relay = r; }

    // Env var overrides (priority: CLI > env > config file).
    if let Ok(d) = std::env::var("NEONET_DOMAIN") {
        if !d.is_empty() { cfg.network.domain = d; }
    }
    if let Ok(r) = std::env::var("NEONET_RENDEZVOUS") {
        if !r.is_empty() {
            cfg.network.rendezvous = r.split(',').map(|s| s.trim().to_string()).collect();
        }
    }
    if let Ok(p) = std::env::var("NEONET_LISTEN_PORT") {
        if let Ok(port) = p.parse::<u16>() { cfg.network.listen_port = port; }
    }
    if let Ok(l) = std::env::var("NEONET_LOG") {
        if !l.is_empty() { cfg.daemon.log_level = l; }
    }

    let node_mode = cfg.network.mode.clone();

    // Check keystore.
    if !keystore::keystore_exists() {
        eprintln!("{} Aucun keystore trouvé. Lancez: {}", "Error:".red().bold(), "neonet init".bold());
        return ExitCode::from(EXIT_KEYSTORE_MISSING);
    }

    let passphrase = keystore::get_passphrase("Passphrase : ");
    if passphrase.is_empty() {
        eprintln!("{} Passphrase required.", "Error:".red().bold());
        return ExitCode::from(EXIT_BAD_PASSPHRASE);
    }

    let ks = match keystore::unlock_keystore(passphrase.as_bytes()) {
        Ok(ks) => ks,
        Err(e) => {
            eprintln!("{} {e}", "Error:".red().bold());
            return ExitCode::from(EXIT_BAD_PASSPHRASE);
        }
    };

    println!("{}", "NeoNet daemon v0.1.0".bold());
    println!("{}", "────────────────────".dimmed());
    println!("Mode        : {}", node_mode.cyan());
    println!("Identité    : {}", ks.address(None).cyan());
    if !cfg.network.domain.is_empty() {
        println!("Domaine     : {}", cfg.network.domain.green());
    }
    println!("API locale  : {}", format!("http://{}:{}", cfg.daemon.api_host, cfg.daemon.api_port).green());
    println!("Réseau      : {}", format!("0.0.0.0:{} (QUIC)", cfg.network.listen_port).green());

    let _ = std::fs::create_dir_all(Config::base_dir());

    let token_path = Config::session_token_path();
    let token = match generate_session_token(&token_path) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("{} {e}", "Error:".red().bold());
            return ExitCode::from(EXIT_ERROR);
        }
    };
    println!("Token API   : écrit dans {}", token_path.display().to_string().dimmed());

    let bind_addr = format!("{}:{}", cfg.daemon.api_host, cfg.daemon.api_port);

    let db_path = Config::base_dir().join("neonet.db");
    let conn = match db::open_db(&db_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("{} DB: {e}", "Error:".red().bold());
            return ExitCode::from(EXIT_ERROR);
        }
    };
    let app_state = web::Data::new(AppState::new(ks, conn));

    // QUIC context.
    let quic_db = db::open_db(&Config::base_dir().join("neonet.db")).expect("open db for QUIC");
    let quic_ks = keystore::unlock_keystore(passphrase.as_bytes()).expect("re-unlock for QUIC");
    let quic_ks = Arc::new(quic_ks);

    // Rendezvous table (only in rendezvous mode).
    let rendezvous_table = if node_mode == "rendezvous" {
        let t = rendezvous_server::new_table();
        Some(t)
    } else {
        None
    };

    let quic_ctx = Arc::new(quic::QuicContext {
        keystore: quic_ks.clone(),
        db: Arc::new(std::sync::Mutex::new(quic_db)),
        sessions: quic::SessionMap::default(),
        rendezvous_table: rendezvous_table.clone(),
    });

    let quic_port = cfg.network.listen_port;
    let cfg_clone = cfg.clone();

    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    rt.block_on(async move {
        // Start QUIC listener.
        let _quic_endpoint = match quic::start_listener(quic_port, quic_ctx).await {
            Ok(ep) => ep,
            Err(e) => {
                eprintln!("{} QUIC: {e}", "Error:".red().bold());
                return;
            }
        };

        // Mode-specific startup.
        match node_mode.as_str() {
            "rendezvous" => {
                if let Some(ref t) = rendezvous_table {
                    rendezvous_server::spawn_expiry_task(t.clone());
                }
                println!("En écoute ({}) sur 0.0.0.0:{quic_port}", "rendezvous".cyan());
            }
            "relay" => {
                let domain = cfg_clone.network.domain.clone();
                if domain.is_empty() {
                    eprintln!("{} --domain requis en mode relay", "Error:".red().bold());
                    return;
                }
                for rdv in &cfg_clone.network.rendezvous {
                    let rdv = rdv.clone();
                    let domain = domain.clone();
                    let ks = quic_ks.clone();
                    let port = cfg_clone.network.listen_port;
                    match relay::register_on_rendezvous(&rdv, &domain, port, &ks).await {
                        Ok(()) => {
                            println!("Enregistré sur {} comme {}", rdv.green(), domain.cyan());
                            relay::spawn_reregister_loop(rdv, domain, port, ks.clone());
                            break; // only register once per rdv for now
                        }
                        Err(e) => log::warn!("Enregistrement {rdv} échoué: {e}"),
                    }
                }
            }
            "client" => {
                if !cfg_clone.network.relay.is_empty() {
                    println!("Relay configuré : {}", cfg_clone.network.relay.green());
                    // TODO: persistent connection to relay
                }
            }
            _ => {
                // "full" mode — no special startup
            }
        }

        let session_token = SessionToken(Arc::new(token));
        println!("\n{}", "Daemon prêt.".green().bold());

        HttpServer::new(move || {
            App::new()
                .app_data(web::Data::new(session_token.clone()))
                .app_data(app_state.clone())
                .route("/v1/ws", web::get().to(ws::ws_handler))
                .service(web::scope("").wrap(BearerAuth).configure(api::configure))
        })
        .bind(&bind_addr)
        .expect("Failed to bind")
        .run()
        .await
        .expect("Server error");
    });

    ExitCode::from(EXIT_OK)
}

// ── Stub/helper commands ─────────────────────────────────────────────

fn api_url_from(cli_url: Option<&str>) -> String {
    cli_url
        .map(String::from)
        .or_else(|| std::env::var("NEONET_API_URL").ok())
        .unwrap_or_else(|| "http://127.0.0.1:7780".into())
}

fn read_session_token() -> Option<String> {
    std::fs::read_to_string(Config::session_token_path()).ok()
}

pub fn cmd_stop(api_url: Option<&str>) -> ExitCode {
    let _url = api_url_from(api_url);
    eprintln!("neonet stop: not yet implemented");
    ExitCode::from(EXIT_ERROR)
}

pub fn cmd_status(api_url: Option<&str>, json: bool) -> ExitCode {
    let url = api_url_from(api_url);
    let token = match read_session_token() {
        Some(t) => t,
        None => {
            eprintln!("{} Daemon not running", "Error:".red().bold());
            return ExitCode::from(EXIT_DAEMON_NOT_RUNNING);
        }
    };
    let rt = tokio::runtime::Runtime::new().expect("tokio");
    match rt.block_on(async {
        reqwest::Client::new()
            .get(format!("{url}/v1/peers"))
            .header("Authorization", format!("Bearer {token}"))
            .send().await
    }) {
        Ok(resp) if resp.status().is_success() => {
            if json {
                let body: serde_json::Value = rt.block_on(resp.json()).unwrap_or(serde_json::json!({}));
                println!("{}", serde_json::to_string_pretty(&body).unwrap_or_default());
            } else {
                println!("{}", "Daemon en cours".green().bold());
                if let Ok(pk) = keystore::read_public_key() { println!("Identité: {}", pk.cyan()); }
                println!("API     : {}", url.green());
            }
            ExitCode::from(EXIT_OK)
        }
        _ => {
            eprintln!("{} Daemon non joignable sur {url}", "Error:".red().bold());
            ExitCode::from(EXIT_DAEMON_NOT_RUNNING)
        }
    }
}

pub fn cmd_identity(action: IdentityAction) -> ExitCode {
    match action {
        IdentityAction::Show => {
            match keystore::read_public_key() {
                Ok(pk) => { println!("Clé pub: {}", pk.cyan()); ExitCode::from(EXIT_OK) }
                Err(e) => { eprintln!("{} {e}", "Error:".red().bold()); ExitCode::from(EXIT_KEYSTORE_MISSING) }
            }
        }
        IdentityAction::Export => {
            match keystore::read_public_key() {
                Ok(pk) => { println!("{pk}"); ExitCode::from(EXIT_OK) }
                Err(e) => { eprintln!("{} {e}", "Error:".red().bold()); ExitCode::from(EXIT_KEYSTORE_MISSING) }
            }
        }
        IdentityAction::Sign { .. } => {
            eprintln!("neonet identity sign: not yet implemented");
            ExitCode::from(EXIT_ERROR)
        }
    }
}

pub fn cmd_peers(action: PeersAction, _api_url: Option<&str>, _json: bool) -> ExitCode {
    match action {
        PeersAction::List => {
            eprintln!("neonet peers list: not yet implemented");
            ExitCode::from(EXIT_ERROR)
        }

        PeersAction::Connect { addr, rendezvous } => {
            let passphrase = keystore::get_passphrase("Passphrase : ");
            let ks = match keystore::unlock_keystore(passphrase.as_bytes()) {
                Ok(k) => k, Err(e) => {
                    eprintln!("{} {e}", "Error:".red().bold());
                    return ExitCode::from(EXIT_BAD_PASSPHRASE);
                }
            };
            let rt = tokio::runtime::Runtime::new().expect("tokio");

            // If the addr doesn't look like host:port, try rendezvous lookup.
            let target = if addr.contains(':') && addr.parse::<std::net::SocketAddr>().is_ok() {
                addr
            } else if let Some(rdv) = rendezvous {
                // Domain lookup.
                println!("Lookup {} sur {}...", addr.cyan(), rdv.green());
                match rt.block_on(relay::lookup_domain(&rdv, &addr, &ks)) {
                    Ok((peer_addr, peer_pk)) => {
                        let pk_str = neonet_crypto::encode_verifying_key(
                            &neonet_crypto::VerifyingKey(peer_pk),
                        );
                        println!("{} → {} (pubkey: {})", addr.cyan(), peer_addr.green(), pk_str.dimmed());
                        peer_addr
                    }
                    Err(e) => {
                        eprintln!("{} {e}", "Error:".red().bold());
                        return ExitCode::from(EXIT_NETWORK);
                    }
                }
            } else {
                eprintln!("{} Adresse invalide. Utilisez host:port ou --rendezvous", "Error:".red().bold());
                return ExitCode::from(EXIT_ERROR);
            };

            println!("Connexion QUIC vers {}...", target.green());
            match rt.block_on(quic::connect_and_ping(&target, &ks)) {
                Ok(()) => ExitCode::from(EXIT_OK),
                Err(e) => { eprintln!("{} {e}", "Error:".red().bold()); ExitCode::from(EXIT_NETWORK) }
            }
        }

        PeersAction::Ping { addr } => {
            let passphrase = keystore::get_passphrase("Passphrase : ");
            let ks = match keystore::unlock_keystore(passphrase.as_bytes()) {
                Ok(k) => k, Err(e) => {
                    eprintln!("{} {e}", "Error:".red().bold());
                    return ExitCode::from(EXIT_BAD_PASSPHRASE);
                }
            };
            let rt = tokio::runtime::Runtime::new().expect("tokio");
            match rt.block_on(quic::connect_and_ping(&addr, &ks)) {
                Ok(()) => ExitCode::from(EXIT_OK),
                Err(e) => { eprintln!("{} {e}", "Error:".red().bold()); ExitCode::from(EXIT_NETWORK) }
            }
        }

        PeersAction::Find { node_id, rendezvous } => {
            let rdv = match rendezvous {
                Some(r) => r,
                None => {
                    eprintln!("{} --rendezvous requis pour le lookup", "Error:".red().bold());
                    return ExitCode::from(EXIT_ERROR);
                }
            };
            let passphrase = keystore::get_passphrase("Passphrase : ");
            let ks = match keystore::unlock_keystore(passphrase.as_bytes()) {
                Ok(k) => k, Err(e) => {
                    eprintln!("{} {e}", "Error:".red().bold());
                    return ExitCode::from(EXIT_BAD_PASSPHRASE);
                }
            };
            let rt = tokio::runtime::Runtime::new().expect("tokio");
            match rt.block_on(relay::lookup_domain(&rdv, &node_id, &ks)) {
                Ok((peer_addr, peer_pk)) => {
                    let pk_str = neonet_crypto::encode_verifying_key(
                        &neonet_crypto::VerifyingKey(peer_pk),
                    );
                    println!("{} → {} (pubkey: {})", node_id.cyan(), peer_addr.green(), pk_str);
                    ExitCode::from(EXIT_OK)
                }
                Err(e) => {
                    eprintln!("{} {e}", "Error:".red().bold());
                    ExitCode::from(EXIT_NETWORK)
                }
            }
        }

        PeersAction::Rendezvous => {
            eprintln!("neonet peers rendezvous: not yet implemented");
            ExitCode::from(EXIT_ERROR)
        }
    }
}

pub fn cmd_rooms(action: RoomsAction, _api_url: Option<&str>, _json: bool) -> ExitCode {
    match action {
        RoomsAction::List => eprintln!("neonet rooms list: not yet implemented"),
        RoomsAction::Create { name, .. } => eprintln!("neonet rooms create --name {name}: not yet implemented"),
        RoomsAction::Join { url } => eprintln!("neonet rooms join {url}: not yet implemented"),
        RoomsAction::Info { room_id } => eprintln!("neonet rooms info {room_id}: not yet implemented"),
        RoomsAction::Leave { room_id } => eprintln!("neonet rooms leave {room_id}: not yet implemented"),
    }
    ExitCode::from(EXIT_ERROR)
}

pub fn cmd_rendezvous(action: RendezvousAction) -> ExitCode {
    match action {
        RendezvousAction::List => eprintln!("not yet implemented"),
        RendezvousAction::Add { url } => eprintln!("add {url}: not yet implemented"),
        RendezvousAction::Remove { url } => eprintln!("remove {url}: not yet implemented"),
        RendezvousAction::Verify { url } => eprintln!("verify {url}: not yet implemented"),
        RendezvousAction::Serve { port, .. } => eprintln!("serve --port {port}: not yet implemented"),
    }
    ExitCode::from(EXIT_ERROR)
}

pub fn cmd_logs(_level: Option<String>, _since: Option<String>, _no_follow: bool) -> ExitCode {
    eprintln!("neonet logs: not yet implemented");
    ExitCode::from(EXIT_ERROR)
}

pub fn cmd_config(action: ConfigAction) -> ExitCode {
    match action {
        ConfigAction::Show => {
            match Config::load() {
                Ok(cfg) => println!("{}", toml::to_string_pretty(&cfg).unwrap_or_default()),
                Err(e) => { eprintln!("{} {e}", "Error:".red().bold()); return ExitCode::from(EXIT_ERROR); }
            }
            ExitCode::from(EXIT_OK)
        }
        ConfigAction::Set { key, value } => { eprintln!("set {key}={value}: stub"); ExitCode::from(EXIT_ERROR) }
        ConfigAction::Get { key } => { eprintln!("get {key}: stub"); ExitCode::from(EXIT_ERROR) }
    }
}

use crate::keystore::Keystore;
