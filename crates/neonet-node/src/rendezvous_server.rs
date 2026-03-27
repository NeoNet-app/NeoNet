use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use neonet_core::NodeType;
use tokio::sync::Mutex;

/// An entry in the rendezvous directory.
#[derive(Debug, Clone)]
pub struct RendezvousEntry {
    pub domain: String,
    pub addr: String,
    pub pubkey: [u8; 32],
    pub node_type: NodeType,
    pub registered_at: Instant,
    pub ttl: Duration,
}

impl RendezvousEntry {
    pub fn is_expired(&self) -> bool {
        self.registered_at.elapsed() > self.ttl
    }
}

/// In-memory rendezvous directory.
pub type RendezvousTable = Arc<Mutex<HashMap<String, RendezvousEntry>>>;

pub fn new_table() -> RendezvousTable {
    Arc::new(Mutex::new(HashMap::new()))
}

/// Register (or update) a relay in the directory.
pub async fn register(
    table: &RendezvousTable,
    domain: String,
    addr: String,
    pubkey: [u8; 32],
    node_type: NodeType,
    ttl_secs: u32,
) {
    let entry = RendezvousEntry {
        domain: domain.clone(),
        addr,
        pubkey,
        node_type,
        registered_at: Instant::now(),
        ttl: Duration::from_secs(ttl_secs as u64),
    };
    log::info!(
        "Rendezvous: enregistrement {} → {} (TTL {}s)",
        domain,
        entry.addr,
        ttl_secs
    );
    table.lock().await.insert(domain, entry);
}

/// Lookup a domain. Returns None if not found or expired.
pub async fn lookup(table: &RendezvousTable, domain: &str) -> Option<RendezvousEntry> {
    let t = table.lock().await;
    let entry = t.get(domain)?;
    if entry.is_expired() {
        log::debug!("Rendezvous: {domain} expired");
        return None;
    }
    Some(entry.clone())
}

/// Spawn a background task that prunes expired entries every 60s.
pub fn spawn_expiry_task(table: RendezvousTable) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(60));
        loop {
            interval.tick().await;
            let mut t = table.lock().await;
            let before = t.len();
            t.retain(|_, e| !e.is_expired());
            let pruned = before - t.len();
            if pruned > 0 {
                log::info!("Rendezvous: purgé {pruned} entrées expirées ({} restantes)", t.len());
            }
        }
    });
}
