use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::sync::Arc;

use actix_web::body::EitherBody;
use actix_web::dev::{Service, ServiceRequest, ServiceResponse, Transform};
use actix_web::{HttpResponse, web};
use serde_json::json;

use std::future::{Ready, ready};
use std::pin::Pin;
use std::task::{Context, Poll};

// ── Token generation ─────────────────────────────────────────────────

/// Generate a session token (UUID v4), write it to `path` with chmod 600.
pub fn generate_session_token(path: &Path) -> std::io::Result<String> {
    // Ensure parent directory exists.
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let token = uuid::Uuid::new_v4().to_string();
    fs::write(path, &token)?;

    // chmod 600
    let mut perms = fs::metadata(path)?.permissions();
    perms.set_mode(0o600);
    fs::set_permissions(path, perms)?;

    Ok(token)
}

/// Shared state holding the current session token.
#[derive(Clone)]
pub struct SessionToken(pub Arc<String>);

// ── Bearer token middleware ──────────────────────────────────────────

/// Actix-web middleware that enforces `Authorization: Bearer <token>` on
/// every request. Returns 401 if the token is missing or wrong.
pub struct BearerAuth;

impl<S, B> Transform<S, ServiceRequest> for BearerAuth
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = actix_web::Error> + 'static,
    B: 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = actix_web::Error;
    type Transform = BearerAuthMiddleware<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(BearerAuthMiddleware { service }))
    }
}

pub struct BearerAuthMiddleware<S> {
    service: S,
}

impl<S, B> Service<ServiceRequest> for BearerAuthMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = actix_web::Error> + 'static,
    B: 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = actix_web::Error;
    type Future = Pin<Box<dyn std::future::Future<Output = Result<Self::Response, Self::Error>>>>;

    fn poll_ready(&self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.service.poll_ready(cx)
    }

    fn call(&self, req: ServiceRequest) -> Self::Future {
        // Extract the expected token from app data.
        let token = req.app_data::<web::Data<SessionToken>>().cloned();

        let authorized = (|| {
            let expected = &token?.0;
            let header = req.headers().get("Authorization")?.to_str().ok()?;
            let bearer = header.strip_prefix("Bearer ")?;
            if bearer == expected.as_str() {
                Some(())
            } else {
                None
            }
        })();

        if authorized.is_some() {
            let fut = self.service.call(req);
            Box::pin(async move {
                let res = fut.await?;
                Ok(res.map_into_left_body())
            })
        } else {
            let response = HttpResponse::Unauthorized().json(json!({
                "error": "unauthorized",
                "message": "Missing or invalid session token"
            }));
            Box::pin(async move {
                Ok(req.into_response(response).map_into_right_body())
            })
        }
    }
}
