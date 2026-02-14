extern crate neonet;
use std::fs;
use actix_cors::Cors;
use actix_web::{HttpServer,App};
use neonet::api;

// init the tracing module
use neonet::helper::trace::{init_trace,trace_logs};
use neonet::helper::start::start;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    println!("{}", fs::read_to_string("utils/ascii.art")?.as_str());
    init_trace();
    trace_logs("Server is starting...".to_string());
    start().await;
    
    let port: u16 = 8080;
    HttpServer::new(|| {
        let cors = Cors::default().allow_any_origin().allow_any_method().allow_any_header();
        App::new().wrap(cors)
            .service(api::init::init_v1api())
    }).bind(("0.0.0.0",port))?.run().await
}
