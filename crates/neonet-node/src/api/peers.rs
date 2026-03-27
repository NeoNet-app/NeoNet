use actix_web::HttpResponse;
use serde_json::json;

/// `GET /v1/peers` — List connected peers and DHT state.
pub async fn list_peers() -> HttpResponse {
    HttpResponse::NotImplemented().json(json!({
        "error": "not_implemented",
        "message": "GET /v1/peers is not yet implemented"
    }))
}

/// `GET /v1/peers/rendezvous` — List configured rendezvous nodes.
pub async fn list_rendezvous() -> HttpResponse {
    HttpResponse::NotImplemented().json(json!({
        "error": "not_implemented",
        "message": "GET /v1/peers/rendezvous is not yet implemented"
    }))
}
