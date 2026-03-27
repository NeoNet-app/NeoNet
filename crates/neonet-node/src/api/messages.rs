use actix_web::{HttpResponse, web};
use serde::Deserialize;
use serde_json::json;

use crate::db;
use crate::keystore::Keystore;
use crate::state::{AppState, WsEvent};

/// `GET /v1/rooms/{room_id}/messages` — List messages with pagination.
pub async fn list_messages(
    state: web::Data<AppState>,
    path: web::Path<String>,
    query: web::Query<ListMessagesQuery>,
) -> HttpResponse {
    let room_id = path.into_inner();
    let limit = query.limit.unwrap_or(50).min(200);

    let conn = state.db.lock().unwrap();

    // Verify room exists.
    match db::get_room(&conn, &room_id) {
        Ok(None) => {
            return HttpResponse::NotFound().json(json!({
                "error": "not_found",
                "message": "Room not found"
            }));
        }
        Err(e) => {
            return HttpResponse::InternalServerError().json(json!({
                "error": "internal_error",
                "message": format!("Database error: {e}")
            }));
        }
        Ok(Some(_)) => {}
    }

    // Fetch events.
    let before_ts = query.before.as_ref().and_then(|_| {
        // If `before` is provided as a timestamp, use it. For now treat as ts.
        query.before.as_ref()?.parse::<i64>().ok()
    });

    match db::list_events(&conn, &room_id, limit as i64, before_ts) {
        Ok(events) => {
            let total = db::count_events(&conn, &room_id).unwrap_or(0);
            let messages: Vec<serde_json::Value> = events
                .iter()
                .map(|e| {
                    // Parse content as JSON if possible.
                    let content: serde_json::Value =
                        serde_json::from_str(&e.content).unwrap_or(json!({ "text": e.content }));
                    json!({
                        "event_id": e.event_id,
                        "author": e.author,
                        "kind": e.kind,
                        "content": content,
                        "parents": serde_json::from_str::<serde_json::Value>(&e.parents)
                            .unwrap_or(json!([])),
                        "ts_hint": e.ts,
                        "edited": false,
                        "redacted": false
                    })
                })
                .collect();
            let has_more = total > messages.len() as i64;
            HttpResponse::Ok().json(json!({
                "messages": messages,
                "has_more": has_more
            }))
        }
        Err(e) => HttpResponse::InternalServerError().json(json!({
            "error": "internal_error",
            "message": format!("Database error: {e}")
        })),
    }
}

/// `POST /v1/rooms/{room_id}/messages` — Send a message.
pub async fn send_message(
    state: web::Data<AppState>,
    path: web::Path<String>,
    body: web::Json<SendMessageRequest>,
) -> HttpResponse {
    use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};

    let room_id = path.into_inner();

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;

    let conn = state.db.lock().unwrap();

    // Verify room exists.
    match db::get_room(&conn, &room_id) {
        Ok(None) => {
            return HttpResponse::NotFound().json(json!({
                "error": "not_found",
                "message": "Room not found"
            }));
        }
        Err(e) => {
            return HttpResponse::InternalServerError().json(json!({
                "error": "internal_error",
                "message": format!("Database error: {e}")
            }));
        }
        Ok(Some(_)) => {}
    }

    // Build content JSON.
    let content = json!({ "text": body.text });
    let content_str = content.to_string();

    // event_id = blake3(content_bytes)
    let event_hash = neonet_crypto::Hash::digest(content_str.as_bytes());
    // Add timestamp to hash to avoid collisions for identical messages.
    let mut unique_input = Vec::new();
    unique_input.extend_from_slice(event_hash.as_bytes());
    unique_input.extend_from_slice(&now.to_le_bytes());
    let unique_hash = neonet_crypto::Hash::digest(&unique_input);
    let event_id = URL_SAFE_NO_PAD.encode(unique_hash.as_bytes());

    // Parents = current tips (latest events in this room).
    let parents = db::latest_event_ids(&conn, &room_id, 5).unwrap_or_default();
    let parents_json = serde_json::to_string(&parents).unwrap_or_else(|_| "[]".into());

    let author = state.keystore.address(None);

    // Determine kind.
    let kind = if body.reply_to.is_some() {
        "thread_reply"
    } else {
        "message"
    };

    if let Err(e) = db::insert_event(
        &conn,
        &event_id,
        &room_id,
        Some(&author),
        kind,
        &content_str,
        &parents_json,
        now,
    ) {
        return HttpResponse::InternalServerError().json(json!({
            "error": "internal_error",
            "message": format!("Failed to store event: {e}")
        }));
    }

    // Broadcast to WebSocket subscribers.
    let ws_payload = json!({
        "type": "new_message",
        "room_id": room_id,
        "event_id": event_id,
        "author": author,
        "kind": kind,
        "content": content,
        "ts_hint": now
    })
    .to_string();

    let _ = state.ws_tx.send(WsEvent {
        room_id: room_id.clone(),
        payload: ws_payload,
    });

    HttpResponse::Ok().json(json!({
        "event_id": event_id,
        "ts_hint": now
    }))
}

/// `POST /v1/rooms/{room_id}/messages/{event_id}/react` — Add a reaction.
pub async fn react() -> HttpResponse {
    HttpResponse::NotImplemented().json(json!({
        "error": "not_implemented",
        "message": "POST /v1/rooms/{room_id}/messages/{event_id}/react is not yet implemented"
    }))
}

/// `PATCH /v1/rooms/{room_id}/messages/{event_id}` — Edit a message.
pub async fn edit_message() -> HttpResponse {
    HttpResponse::NotImplemented().json(json!({
        "error": "not_implemented",
        "message": "PATCH /v1/rooms/{room_id}/messages/{event_id} is not yet implemented"
    }))
}

/// `DELETE /v1/rooms/{room_id}/messages/{event_id}` — Delete a message.
pub async fn delete_message() -> HttpResponse {
    HttpResponse::NotImplemented().json(json!({
        "error": "not_implemented",
        "message": "DELETE /v1/rooms/{room_id}/messages/{event_id} is not yet implemented"
    }))
}

#[derive(Deserialize)]
pub struct ListMessagesQuery {
    pub since: Option<String>,
    pub before: Option<String>,
    pub limit: Option<usize>,
}

#[derive(Deserialize)]
pub struct SendMessageRequest {
    pub text: String,
    pub reply_to: Option<String>,
}
