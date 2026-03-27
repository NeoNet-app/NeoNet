use actix_web::{HttpResponse, web};
use serde::Deserialize;
use serde_json::json;

use crate::keystore::Keystore;
use crate::state::AppState;

/// `GET /v1/identity` — Return the local node identity.
pub async fn get_identity(state: web::Data<AppState>) -> HttpResponse {
    let pubkey = neonet_crypto::encode_verifying_key(
        &neonet_crypto::VerifyingKey(state.keystore.pubkey()),
    );
    let address = state.keystore.address(None);

    HttpResponse::Ok().json(json!({
        "pubkey": pubkey,
        "address": address,
        "node_type": "full"
    }))
}

/// `POST /v1/identity/sign` — Sign an arbitrary payload with the local key.
pub async fn sign_payload(
    state: web::Data<AppState>,
    body: web::Json<SignRequest>,
) -> HttpResponse {
    use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};

    let payload_bytes = match URL_SAFE_NO_PAD.decode(&body.payload) {
        Ok(b) => b,
        Err(_) => {
            return HttpResponse::BadRequest().json(json!({
                "error": "bad_request",
                "message": "Invalid base64url payload"
            }));
        }
    };

    let sig = state.keystore.sign(&payload_bytes);
    let sig_encoded = neonet_crypto::encode_prefixed(&sig);
    let pubkey = neonet_crypto::encode_verifying_key(
        &neonet_crypto::VerifyingKey(state.keystore.pubkey()),
    );

    HttpResponse::Ok().json(json!({
        "sig": sig_encoded,
        "pubkey": pubkey
    }))
}

#[derive(Deserialize)]
pub struct SignRequest {
    pub payload: String,
}
