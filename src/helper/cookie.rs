use actix_web::{self, HttpRequest};

pub fn auth_token(req: HttpRequest) -> String {
    // get the auth token from the request header
    if let Some(token) = req.headers().get("Authorization") {
        // secure me later
        return token.to_str().unwrap_or("none").to_string().replace("token ", "");
    }
    "none".to_string()
}