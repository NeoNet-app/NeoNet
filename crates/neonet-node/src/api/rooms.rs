use actix_web::{HttpResponse, web};
use serde::Deserialize;
use serde_json::json;

use crate::db;
use crate::keystore::Keystore;
use crate::state::AppState;

/// `GET /v1/rooms` — List all rooms the node is a member of.
pub async fn list_rooms(state: web::Data<AppState>) -> HttpResponse {
    let conn = state.db.lock().unwrap();
    match db::list_rooms(&conn) {
        Ok(rooms) => {
            let rooms_json: Vec<serde_json::Value> = rooms
                .iter()
                .map(|r| {
                    json!({
                        "room_id": r.room_id,
                        "room_type": r.room_type,
                        "name": r.name,
                        "description": r.description,
                        "member_count": r.member_count,
                        "last_event_ts": r.last_event_ts,
                        "created_at": r.created_at
                    })
                })
                .collect();
            HttpResponse::Ok().json(json!({ "rooms": rooms_json }))
        }
        Err(e) => HttpResponse::InternalServerError().json(json!({
            "error": "internal_error",
            "message": format!("Database error: {e}")
        })),
    }
}

/// `POST /v1/rooms` — Create a new room with genesis event.
pub async fn create_room(
    state: web::Data<AppState>,
    body: web::Json<CreateRoomRequest>,
) -> HttpResponse {
    use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;

    // Generate room_id = blake3(creator_pubkey || nonce || ts)
    let pubkey = state.keystore.pubkey();
    let mut nonce = [0u8; 16];
    rand::RngCore::fill_bytes(&mut rand::rngs::OsRng, &mut nonce);
    let mut hasher_input = Vec::new();
    hasher_input.extend_from_slice(&pubkey);
    hasher_input.extend_from_slice(&nonce);
    hasher_input.extend_from_slice(&now.to_le_bytes());
    let room_id_hash = neonet_crypto::Hash::digest(&hasher_input);
    let room_id = URL_SAFE_NO_PAD.encode(room_id_hash.as_bytes());

    let room_type = body.room_type.as_deref().unwrap_or("group");
    let name = body.name.as_deref().unwrap_or("");
    let description = body.description.as_deref().unwrap_or("");

    let conn = state.db.lock().unwrap();

    // Insert room.
    if let Err(e) = db::insert_room(&conn, &room_id, room_type, name, description, now) {
        return HttpResponse::InternalServerError().json(json!({
            "error": "internal_error",
            "message": format!("Failed to create room: {e}")
        }));
    }

    // Create genesis event (kind=genesis, no parents).
    let genesis_content = json!({
        "room_id": room_id,
        "room_type": room_type,
        "name": name,
        "description": description,
        "creator": neonet_crypto::encode_verifying_key(
            &neonet_crypto::VerifyingKey(pubkey)
        )
    })
    .to_string();

    let genesis_hash = neonet_crypto::Hash::digest(genesis_content.as_bytes());
    let genesis_id = URL_SAFE_NO_PAD.encode(genesis_hash.as_bytes());
    let author_addr = state.keystore.address(None);

    if let Err(e) = db::insert_event(
        &conn,
        &genesis_id,
        &room_id,
        Some(&author_addr),
        "genesis",
        &genesis_content,
        "[]",
        now,
    ) {
        return HttpResponse::InternalServerError().json(json!({
            "error": "internal_error",
            "message": format!("Failed to create genesis event: {e}")
        }));
    }

    HttpResponse::Ok().json(json!({
        "room_id": room_id,
        "room_type": room_type,
        "name": name,
        "created_at": now
    }))
}

/// `GET /v1/rooms/{room_id}` — Get full room state.
pub async fn get_room(
    state: web::Data<AppState>,
    path: web::Path<String>,
) -> HttpResponse {
    let room_id = path.into_inner();
    let conn = state.db.lock().unwrap();

    match db::get_room(&conn, &room_id) {
        Ok(Some(room)) => HttpResponse::Ok().json(json!({
            "room_id": room.room_id,
            "room_type": room.room_type,
            "name": room.name,
            "description": room.description,
            "members": [],
            "settings": {},
            "created_at": room.created_at
        })),
        Ok(None) => HttpResponse::NotFound().json(json!({
            "error": "not_found",
            "message": "Room not found"
        })),
        Err(e) => HttpResponse::InternalServerError().json(json!({
            "error": "internal_error",
            "message": format!("Database error: {e}")
        })),
    }
}

/// `PATCH /v1/rooms/{room_id}` — Update room metadata.
pub async fn patch_room() -> HttpResponse {
    HttpResponse::NotImplemented().json(json!({
        "error": "not_implemented",
        "message": "PATCH /v1/rooms/{room_id} is not yet implemented"
    }))
}

/// `DELETE /v1/rooms/{room_id}` — Leave or delete a room.
pub async fn delete_room() -> HttpResponse {
    HttpResponse::NotImplemented().json(json!({
        "error": "not_implemented",
        "message": "DELETE /v1/rooms/{room_id} is not yet implemented"
    }))
}

#[derive(Deserialize)]
pub struct CreateRoomRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub room_type: Option<String>,
    pub members: Option<Vec<String>>,
}
