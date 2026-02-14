use actix_web::{CustomizeResponder, HttpResponse, Responder};
use actix_web::http::header;
use serde_json::json;
use crate::api::v1::main::ApiResult;

pub fn api_response(result: ApiResult) -> CustomizeResponder<HttpResponse> {
    match result {
        ApiResult::Json(data) => {
            HttpResponse::Ok()
                .content_type("application/json")
                .body(data)
                .customize()
        }

        ApiResult::JsonWithHeaders { body, headers } => {
            let mut response = HttpResponse::Ok();
            response.content_type("application/json");

            for (k, v) in headers {
                response.append_header((k.as_str(), v.as_str()));
            }

            response.body(body).customize()
        }

        ApiResult::Redirect(to) => {
            HttpResponse::TemporaryRedirect()
                .append_header((header::LOCATION, to))
                .finish()
                .customize()
        }

        ApiResult::NotFound => {
            HttpResponse::NotFound()
                .content_type("application/json")
                .body(r#"{"error":"Not found"}"#)
                .customize()
        }

        ApiResult::Unauthorized => {
            HttpResponse::Unauthorized()
                .content_type("application/json")
                .body(r#"{"error":"Unauthorized"}"#)
                .customize()
        }

        ApiResult::BadRequest(message) => {
            HttpResponse::BadRequest()
                .content_type("application/json")
                .body(json!({"error": message}).to_string())
                .customize()
        }
    }
}