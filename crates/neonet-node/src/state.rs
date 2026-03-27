use std::sync::Mutex;

use tokio::sync::broadcast;

use crate::keystore::UnlockedKeystore;

/// Shared application state, injected into all handlers via `web::Data<AppState>`.
pub struct AppState {
    /// Unlocked keystore — holds the signing key in memory.
    pub keystore: UnlockedKeystore,
    /// SQLite database connection (single-writer, behind a Mutex).
    pub db: Mutex<rusqlite::Connection>,
    /// Broadcast channel for real-time WS push (new_message events).
    pub ws_tx: broadcast::Sender<WsEvent>,
}

/// Events pushed to WebSocket subscribers.
#[derive(Clone, Debug)]
pub struct WsEvent {
    pub room_id: String,
    pub payload: String, // JSON string ready to send
}

impl AppState {
    pub fn new(
        keystore: UnlockedKeystore,
        db: rusqlite::Connection,
    ) -> Self {
        let (ws_tx, _) = broadcast::channel(256);
        Self {
            keystore,
            db: Mutex::new(db),
            ws_tx,
        }
    }
}
