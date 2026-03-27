use std::path::Path;

use rusqlite::{Connection, params};

/// Open (or create) the NeoNet SQLite database and ensure schema exists.
pub fn open_db(path: &Path) -> Result<Connection, rusqlite::Error> {
    let conn = Connection::open(path)?;
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS rooms (
            room_id     TEXT PRIMARY KEY,
            room_type   TEXT NOT NULL,
            name        TEXT NOT NULL DEFAULT '',
            description TEXT NOT NULL DEFAULT '',
            created_at  INTEGER NOT NULL
        );

        CREATE TABLE IF NOT EXISTS events (
            event_id    TEXT PRIMARY KEY,
            room_id     TEXT NOT NULL,
            author      TEXT,
            kind        TEXT NOT NULL,
            content     TEXT NOT NULL DEFAULT '',
            parents     TEXT NOT NULL DEFAULT '[]',
            ts          INTEGER NOT NULL,
            FOREIGN KEY (room_id) REFERENCES rooms(room_id)
        );

        CREATE INDEX IF NOT EXISTS idx_events_room_ts ON events (room_id, ts);
        CREATE INDEX IF NOT EXISTS idx_events_room_id ON events (room_id, event_id);

        CREATE TABLE IF NOT EXISTS known_peers (
            addr        TEXT PRIMARY KEY,
            pubkey      TEXT NOT NULL,
            first_seen  INTEGER NOT NULL,
            last_seen   INTEGER NOT NULL
        );
        ",
    )?;
    Ok(conn)
}

// ── Room operations ──────────────────────────────────────────────────

pub fn insert_room(
    conn: &Connection,
    room_id: &str,
    room_type: &str,
    name: &str,
    description: &str,
    created_at: i64,
) -> Result<(), rusqlite::Error> {
    conn.execute(
        "INSERT INTO rooms (room_id, room_type, name, description, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![room_id, room_type, name, description, created_at],
    )?;
    Ok(())
}

#[derive(Debug, serde::Serialize)]
pub struct RoomRow {
    pub room_id: String,
    pub room_type: String,
    pub name: String,
    pub description: String,
    pub created_at: i64,
    pub member_count: i64,
    pub last_event_ts: Option<i64>,
}

pub fn list_rooms(conn: &Connection) -> Result<Vec<RoomRow>, rusqlite::Error> {
    let mut stmt = conn.prepare(
        "SELECT r.room_id, r.room_type, r.name, r.description, r.created_at,
                0 as member_count,
                (SELECT MAX(e.ts) FROM events e WHERE e.room_id = r.room_id) as last_event_ts
         FROM rooms r
         ORDER BY r.created_at DESC",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(RoomRow {
            room_id: row.get(0)?,
            room_type: row.get(1)?,
            name: row.get(2)?,
            description: row.get(3)?,
            created_at: row.get(4)?,
            member_count: row.get(5)?,
            last_event_ts: row.get(6)?,
        })
    })?;
    rows.collect()
}

pub fn get_room(conn: &Connection, room_id: &str) -> Result<Option<RoomRow>, rusqlite::Error> {
    let mut stmt = conn.prepare(
        "SELECT r.room_id, r.room_type, r.name, r.description, r.created_at,
                0 as member_count,
                (SELECT MAX(e.ts) FROM events e WHERE e.room_id = r.room_id) as last_event_ts
         FROM rooms r WHERE r.room_id = ?1",
    )?;
    let mut rows = stmt.query(params![room_id])?;
    match rows.next()? {
        Some(row) => Ok(Some(RoomRow {
            room_id: row.get(0)?,
            room_type: row.get(1)?,
            name: row.get(2)?,
            description: row.get(3)?,
            created_at: row.get(4)?,
            member_count: row.get(5)?,
            last_event_ts: row.get(6)?,
        })),
        None => Ok(None),
    }
}

// ── Event operations ─────────────────────────────────────────────────

pub fn insert_event(
    conn: &Connection,
    event_id: &str,
    room_id: &str,
    author: Option<&str>,
    kind: &str,
    content: &str,
    parents: &str,
    ts: i64,
) -> Result<(), rusqlite::Error> {
    conn.execute(
        "INSERT INTO events (event_id, room_id, author, kind, content, parents, ts)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![event_id, room_id, author, kind, content, parents, ts],
    )?;
    Ok(())
}

#[derive(Debug, serde::Serialize)]
pub struct EventRow {
    pub event_id: String,
    pub room_id: String,
    pub author: Option<String>,
    pub kind: String,
    pub content: String,
    pub parents: String,
    pub ts: i64,
}

pub fn list_events(
    conn: &Connection,
    room_id: &str,
    limit: i64,
    before_ts: Option<i64>,
) -> Result<Vec<EventRow>, rusqlite::Error> {
    fn map_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<EventRow> {
        Ok(EventRow {
            event_id: row.get(0)?,
            room_id: row.get(1)?,
            author: row.get(2)?,
            kind: row.get(3)?,
            content: row.get(4)?,
            parents: row.get(5)?,
            ts: row.get(6)?,
        })
    }

    let mut result: Vec<EventRow> = if let Some(before) = before_ts {
        let mut stmt = conn.prepare(
            "SELECT event_id, room_id, author, kind, content, parents, ts
             FROM events WHERE room_id = ?1 AND ts < ?2
             ORDER BY ts DESC LIMIT ?3",
        )?;
        stmt.query_map(params![room_id, before, limit], map_row)?
            .collect::<Result<_, _>>()?
    } else {
        let mut stmt = conn.prepare(
            "SELECT event_id, room_id, author, kind, content, parents, ts
             FROM events WHERE room_id = ?1
             ORDER BY ts DESC LIMIT ?2",
        )?;
        stmt.query_map(params![room_id, limit], map_row)?
            .collect::<Result<_, _>>()?
    };

    result.reverse(); // oldest first for display
    Ok(result)
}

/// Count events in a room.
pub fn count_events(conn: &Connection, room_id: &str) -> Result<i64, rusqlite::Error> {
    conn.query_row(
        "SELECT COUNT(*) FROM events WHERE room_id = ?1",
        params![room_id],
        |row| row.get(0),
    )
}

/// Get the latest event IDs in a room (tips).
pub fn latest_event_ids(
    conn: &Connection,
    room_id: &str,
    limit: i64,
) -> Result<Vec<String>, rusqlite::Error> {
    let mut stmt = conn.prepare(
        "SELECT event_id FROM events WHERE room_id = ?1 ORDER BY ts DESC LIMIT ?2",
    )?;
    let rows = stmt.query_map(params![room_id, limit], |row| row.get(0))?;
    rows.collect()
}

// ── Known peers (TOFU) ───────────────────────────────────────────────

/// Check if a peer's pubkey matches what we have on file (TOFU).
/// Returns Ok(true) if known and matches, Ok(false) if new, Err if mismatch.
pub enum TofuResult {
    /// First time seeing this peer — stored.
    NewPeer,
    /// Known peer, pubkey matches.
    Known,
    /// Known peer but pubkey CHANGED — possible MITM.
    Mismatch { expected: String },
}

pub fn tofu_check(
    conn: &Connection,
    addr: &str,
    pubkey: &str,
    now: i64,
) -> Result<TofuResult, rusqlite::Error> {
    let existing: Option<String> = conn
        .query_row(
            "SELECT pubkey FROM known_peers WHERE addr = ?1",
            params![addr],
            |row| row.get(0),
        )
        .ok();

    match existing {
        Some(ref stored) if stored == pubkey => {
            conn.execute(
                "UPDATE known_peers SET last_seen = ?1 WHERE addr = ?2",
                params![now, addr],
            )?;
            Ok(TofuResult::Known)
        }
        Some(stored) => Ok(TofuResult::Mismatch { expected: stored }),
        None => {
            conn.execute(
                "INSERT INTO known_peers (addr, pubkey, first_seen, last_seen)
                 VALUES (?1, ?2, ?3, ?3)",
                params![addr, pubkey, now],
            )?;
            Ok(TofuResult::NewPeer)
        }
    }
}
