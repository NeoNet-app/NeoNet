use std::path::Path;
use std::time::Instant;
use std::collections::HashSet;

use rusqlite::{Connection, params};
use thiserror::Error;

use crate::event::Event;
use crate::sync::{EventId, EventState, StoredEvent};
use neonet_crypto::Hash;

// ── EventStore trait ─────────────────────────────────────────────────

/// Persistent storage for a room's event DAG.
pub trait EventStore {
    type Error: std::error::Error;

    fn insert(&mut self, event: StoredEvent) -> Result<(), Self::Error>;
    fn get(&self, id: &EventId) -> Result<Option<StoredEvent>, Self::Error>;
    fn tips(&self) -> Result<Vec<EventId>, Self::Error>;
    fn events_since(&self, ts: u64) -> Result<Vec<StoredEvent>, Self::Error>;
    fn ancestors(&self, id: &EventId, depth: usize) -> Result<Vec<StoredEvent>, Self::Error>;
}

// ── SQLite implementation ────────────────────────────────────────────

#[derive(Debug, Error)]
pub enum SqliteStoreError {
    #[error("sqlite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("postcard error: {0}")]
    Postcard(#[from] postcard::Error),
}

/// SQLite-backed event store (via `rusqlite`).
///
/// Schema: a single `events` table with an index on `(room_id, ts)`.
pub struct SqliteEventStore {
    conn: Connection,
}

impl SqliteEventStore {
    /// Open (or create) a SQLite database at the given path.
    pub fn open(path: &Path) -> Result<Self, SqliteStoreError> {
        let conn = Connection::open(path)?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS events (
                id          BLOB PRIMARY KEY,    -- 32 bytes (blake3 hash)
                room_id     BLOB NOT NULL,       -- 32 bytes
                ts          INTEGER NOT NULL,    -- ts hint (u64)
                state       INTEGER NOT NULL,    -- EventState as u8
                event_data  BLOB NOT NULL,       -- postcard-serialised Event
                children    BLOB NOT NULL DEFAULT X'' -- postcard-serialised HashSet<EventId>
            );
            CREATE INDEX IF NOT EXISTS idx_events_room_ts ON events (room_id, ts);"
        )?;
        Ok(Self { conn })
    }

    /// Open an in-memory database (useful for tests).
    pub fn in_memory() -> Result<Self, SqliteStoreError> {
        let conn = Connection::open_in_memory()?;
        conn.execute_batch(
            "CREATE TABLE events (
                id          BLOB PRIMARY KEY,
                room_id     BLOB NOT NULL,
                ts          INTEGER NOT NULL,
                state       INTEGER NOT NULL,
                event_data  BLOB NOT NULL,
                children    BLOB NOT NULL DEFAULT X''
            );
            CREATE INDEX idx_events_room_ts ON events (room_id, ts);"
        )?;
        Ok(Self { conn })
    }
}

impl EventStore for SqliteEventStore {
    type Error = SqliteStoreError;

    fn insert(&mut self, stored: StoredEvent) -> Result<(), Self::Error> {
        let event_data = postcard::to_allocvec(&stored.event)?;
        let children_data = postcard::to_allocvec(&stored.children)?;
        let state_u8 = match stored.state {
            EventState::Pending => 0u8,
            EventState::Valid => 1,
            EventState::Invalid => 2,
        };
        self.conn.execute(
            "INSERT OR REPLACE INTO events (id, room_id, ts, state, event_data, children)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                stored.event.id.as_bytes().as_slice(),
                &[] as &[u8],  // room_id set by caller context; stored for indexing
                stored.event.ts as i64,
                state_u8,
                event_data,
                children_data,
            ],
        )?;
        Ok(())
    }

    fn get(&self, id: &EventId) -> Result<Option<StoredEvent>, Self::Error> {
        let mut stmt = self.conn.prepare(
            "SELECT event_data, state, children FROM events WHERE id = ?1"
        )?;
        let mut rows = stmt.query(params![id.as_bytes().as_slice()])?;
        match rows.next()? {
            Some(row) => {
                let event_data: Vec<u8> = row.get(0)?;
                let state_u8: u8 = row.get(1)?;
                let children_data: Vec<u8> = row.get(2)?;
                let event: Event = postcard::from_bytes(&event_data)?;
                let state = match state_u8 {
                    0 => EventState::Pending,
                    1 => EventState::Valid,
                    _ => EventState::Invalid,
                };
                let children: HashSet<EventId> = if children_data.is_empty() {
                    HashSet::new()
                } else {
                    postcard::from_bytes(&children_data)?
                };
                Ok(Some(StoredEvent {
                    event,
                    received_at: Instant::now(),
                    children,
                    state,
                }))
            }
            None => Ok(None),
        }
    }

    fn tips(&self) -> Result<Vec<EventId>, Self::Error> {
        let mut stmt = self.conn.prepare(
            "SELECT id FROM events WHERE state = 1 AND children = X''"
        )?;
        let rows = stmt.query_map([], |row| {
            let bytes: Vec<u8> = row.get(0)?;
            let mut arr = [0u8; 32];
            arr.copy_from_slice(&bytes);
            Ok(Hash(arr))
        })?;
        let mut tips = Vec::new();
        for row in rows {
            tips.push(row?);
        }
        Ok(tips)
    }

    fn events_since(&self, ts: u64) -> Result<Vec<StoredEvent>, Self::Error> {
        let mut stmt = self.conn.prepare(
            "SELECT event_data, state, children FROM events WHERE ts >= ?1 ORDER BY ts ASC"
        )?;
        let rows = stmt.query_map(params![ts as i64], |row| {
            let event_data: Vec<u8> = row.get(0)?;
            let state_u8: u8 = row.get(1)?;
            let children_data: Vec<u8> = row.get(2)?;
            Ok((event_data, state_u8, children_data))
        })?;
        let mut result = Vec::new();
        for row in rows {
            let (event_data, state_u8, children_data) = row?;
            let event: Event = postcard::from_bytes(&event_data)
                .map_err(SqliteStoreError::Postcard)?;
            let state = match state_u8 {
                0 => EventState::Pending,
                1 => EventState::Valid,
                _ => EventState::Invalid,
            };
            let children: HashSet<EventId> = if children_data.is_empty() {
                HashSet::new()
            } else {
                postcard::from_bytes(&children_data)
                    .map_err(SqliteStoreError::Postcard)?
            };
            result.push(StoredEvent {
                event,
                received_at: Instant::now(),
                children,
                state,
            });
        }
        Ok(result)
    }

    fn ancestors(&self, id: &EventId, depth: usize) -> Result<Vec<StoredEvent>, Self::Error> {
        // BFS backwards through parents up to `depth` levels.
        let mut visited = HashSet::new();
        let mut queue = vec![*id];
        let mut result = Vec::new();

        for _ in 0..depth {
            let mut next_queue = Vec::new();
            for current_id in &queue {
                if !visited.insert(*current_id) {
                    continue;
                }
                if let Some(stored) = self.get(current_id)? {
                    for parent in &stored.event.parents {
                        next_queue.push(*parent);
                    }
                    result.push(stored);
                }
            }
            if next_queue.is_empty() {
                break;
            }
            queue = next_queue;
        }
        Ok(result)
    }
}
