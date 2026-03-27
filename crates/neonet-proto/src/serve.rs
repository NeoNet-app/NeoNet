use actix_web::{HttpResponse, web, get};
use std::path::PathBuf;

/// Shared state for the `.neonet/` HTTP handler.
pub struct NeoNetWellKnown {
    /// Path to the directory containing `rendezvous.toml` (and future files).
    pub dir: PathBuf,
}

/// `GET /.neonet/rendezvous.toml`
///
/// Serves the rendezvous file with:
/// - `Content-Type: application/toml`
/// - `Access-Control-Allow-Origin: *`
/// - `Cache-Control: max-age=3600`
#[get("/.neonet/rendezvous.toml")]
pub async fn rendezvous_handler(
    state: web::Data<NeoNetWellKnown>,
) -> HttpResponse {
    let path = state.dir.join("rendezvous.toml");

    let body = match tokio::fs::read_to_string(&path).await {
        Ok(contents) => contents,
        Err(_) => {
            return HttpResponse::NotFound()
                .body("rendezvous.toml not found");
        }
    };

    HttpResponse::Ok()
        .content_type("application/toml")
        .insert_header(("Access-Control-Allow-Origin", "*"))
        .insert_header(("Cache-Control", "max-age=3600"))
        .body(body)
}

/// Register the `.neonet/` routes on an actix-web app.
///
/// ```rust,no_run
/// use actix_web::{App, HttpServer, web};
/// use neonet_proto::serve::NeoNetWellKnown;
/// use std::path::PathBuf;
///
/// #[actix_web::main]
/// async fn main() -> std::io::Result<()> {
///     HttpServer::new(|| {
///         App::new()
///             .app_data(web::Data::new(NeoNetWellKnown {
///                 dir: PathBuf::from("/etc/neonet"),
///             }))
///             .configure(neonet_proto::serve::configure)
///     })
///     .bind("0.0.0.0:8080")?
///     .run()
///     .await
/// }
/// ```
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(rendezvous_handler);
}
