use actix_web::HttpResponse;
use serde_json::json;

/// `POST /v1/rooms/{room_id}/members` — Invite a member.
pub async fn invite_member() -> HttpResponse {
    HttpResponse::NotImplemented().json(json!({
        "error": "not_implemented",
        "message": "POST /v1/rooms/{room_id}/members is not yet implemented"
    }))
}

/// `PATCH /v1/rooms/{room_id}/members/{address}` — Change member role.
pub async fn change_role() -> HttpResponse {
    HttpResponse::NotImplemented().json(json!({
        "error": "not_implemented",
        "message": "PATCH /v1/rooms/{room_id}/members/{address} is not yet implemented"
    }))
}

/// `DELETE /v1/rooms/{room_id}/members/{address}` — Kick a member.
pub async fn kick_member() -> HttpResponse {
    HttpResponse::NotImplemented().json(json!({
        "error": "not_implemented",
        "message": "DELETE /v1/rooms/{room_id}/members/{address} is not yet implemented"
    }))
}
