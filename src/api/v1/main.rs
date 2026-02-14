use actix_web::{get, post, delete, put, web, CustomizeResponder, HttpRequest, HttpResponse};
use serde_json::Value;
use crate::helper::request::{log_request_v1, get_request_body, RequestDataAPIV1};
use crate::api::v1::{core, home};

//   _____     _ _____             _ _
//  |  _  |___|_| __  |___ ___ _ _| | |_
//  |     | . | |    -| -_|_ -| | | |  _|
//  |__|__|  _|_|__|__|___|___|___|_|_|
//        |_|
pub enum ApiResult {
    Json(String),
    JsonWithHeaders {
        body: String,
        headers: Vec<(String, String)>,
    },
    Redirect(String),
    NotFound,
    Unauthorized,
    BadRequest(String),
}

impl ApiResult {
    pub fn with_headers(body: &str, headers: Vec<(String, String)>) -> Self {
        ApiResult::JsonWithHeaders {
            body: body.to_string(),
            headers,
        }
    }
}

//   _____     _    _____         _
//  |  _  |___|_|  | __  |___ _ _| |_ ___ ___
//  |     | . | |  |    -| . | | |  _| -_|_ -|
//  |__|__|  _|_|  |__|__|___|___|_| |___|___|
//        |_|

pub async fn main_hander(request: RequestDataAPIV1, body: Option<Value>) -> CustomizeResponder<HttpResponse> {

    //println!("{:#?}", request);

    let response = match request.path.as_str() {
        "/" => home::default().await,

        _ => ApiResult::NotFound
    };


    core::api_response(response)
}



#[post("/{path:.*}")]
pub async fn v1_handlerpost(path: web::Path<String>, payload: Option<web::Payload>, req: HttpRequest) -> CustomizeResponder<HttpResponse> {
    let request_body = get_request_body(payload.unwrap()).await;
    main_hander(log_request_v1(path, req, "POST", "v1").await, Some(request_body)).await
}


#[get("/{path:.*}")]
pub async fn v1_handler(path: web::Path<String>, req: HttpRequest) -> CustomizeResponder<HttpResponse> {
    main_hander(log_request_v1(path, req, "GET", "v1").await, None).await
}


#[delete("/{path:.*}")]
pub async fn v1_handlerdelete(path: web::Path<String>, payload: Option<web::Payload>, req: HttpRequest) -> CustomizeResponder<HttpResponse> {
    let request_body = get_request_body(payload.unwrap()).await;
    main_hander(log_request_v1(path, req, "DELETE", "v1").await, Some(request_body)).await
}


#[put("/{path:.*}")]
pub async fn v1_handlerput(path: web::Path<String>, payload: Option<web::Payload>, req: HttpRequest) -> CustomizeResponder<HttpResponse> {
    let request_body = get_request_body(payload.unwrap()).await;
    main_hander(log_request_v1(path, req, "PUT", "v1").await, Some(request_body)).await
}