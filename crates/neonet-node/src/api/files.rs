use actix_web::HttpResponse;
use serde_json::json;

/// `POST /v1/rooms/{room_id}/files` — Upload a file (multipart).
pub async fn upload_file() -> HttpResponse {
    HttpResponse::NotImplemented().json(json!({
        "error": "not_implemented",
        "message": "POST /v1/rooms/{room_id}/files is not yet implemented"
    }))
}

/// `GET /v1/rooms/{room_id}/files/{file_id}` — Download and decrypt a file.
pub async fn download_file() -> HttpResponse {
    HttpResponse::NotImplemented().json(json!({
        "error": "not_implemented",
        "message": "GET /v1/rooms/{room_id}/files/{file_id} is not yet implemented"
    }))
}
