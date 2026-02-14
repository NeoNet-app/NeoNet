use actix_web::web;
use actix_web::Scope;

pub fn init_v1api() -> Scope {
    web::scope("/api/v1")
        .service(crate::api::v1::main::v1_handler)
        .service(crate::api::v1::main::v1_handlerpost)
        .service(crate::api::v1::main::v1_handlerdelete)
        .service(crate::api::v1::main::v1_handlerput)
}