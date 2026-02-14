use actix_web::{web, HttpRequest};
use futures::StreamExt;
use qstring::QString;
use serde::Serialize;
use serde_json::Value;
use crate::helper::trace::{trace_logs,trace_warn};
use crate::helper::cookie::auth_token;

const MAX_SIZE: usize = 2_097_152;
#[derive(Debug, Clone, Serialize)]
pub struct RequestDataAPIV1 {
    // Request basic information
    pub path: String,
    pub user_ip: String,
    pub method: String,
    pub parsed_params: std::collections::HashMap<String, String>,
    pub auth_token: String,
}


pub async fn log_request_v1(path: web::Path<String>, req: HttpRequest, method:&str, version:&str) -> RequestDataAPIV1 {

    //////  GET IP ADDRESS  //////
    let user_ip = get_ip_address(&req);


    let c = auth_token(req.clone());

    trace_logs(format!("API V1 Request: {} [{}] - /api/{}/{}", method, user_ip, version, path));

    // Extract raw query string
    let parsed_params = get_parsed_params(&req);

    let r = RequestDataAPIV1 {
        path: format!("/{}", path.to_string()),
        method: method.to_string(),
        user_ip,
        parsed_params,
        auth_token: c,
    };

    trace_log_files(&r);
    r
}




//                     
//   _____ _   _ _     
//  |  |  | |_|_| |___ 
//  |  |  |  _| | |_ -|
//  |_____|_| |_|_|___|
//                     
pub async fn get_request_body(mut payload: web::Payload) -> Value {
    // payload is a stream of Bytes objects
    let mut body = web::BytesMut::new();
    while let Some(chunk) = payload.next().await {
        let chunk = match chunk {
            Ok(chunk) => chunk,
            Err(_) => {
                return Value::Null;
            }
        };
        // limit max size of in-memory payload
        if (body.len() + chunk.len()) > MAX_SIZE {
            return Value::Null;
        }
        body.extend_from_slice(&chunk);
    }

    // Get the expected data
    let str_data = std::str::from_utf8(&body).expect("Invalid UTF-8");
    let parsed_json: Value = serde_json::from_str(str_data).unwrap_or(Value::Null);

    return parsed_json;
}

fn get_parsed_params(req: &HttpRequest) -> std::collections::HashMap<String, String> {
    // Parse query string using qstring
    // Extract raw query string
    let raw_query = req.query_string();
    if !raw_query.is_empty() {
        if let Some(q) = QString::from(raw_query).into_iter().collect::<std::collections::HashMap<String, String>>().into() {
            return q;
        }
    }
    std::collections::HashMap::new()
}

fn get_ip_address(req: &HttpRequest) -> String {
    let connection_info = req.connection_info().clone();
    let User_ip = connection_info.realip_remote_addr();
    match User_ip {
        Some(ip) => ip.to_string(),
        None => match connection_info.peer_addr() {
              Some(ip) => ip.to_string(),
              None => "unknown".to_string()
          }
    }
}

fn trace_log_files(r: &impl Serialize) {
    // add text trace into the logs file /log/YYYY-MM-DD.log
    let current_date = chrono::Local::now().format("%Y-%m-%d").to_string();
    // write the json data into the log file
    let log_file_path = format!("./log/{}.log", current_date);
    let log_data = serde_json::to_string(&r).unwrap_or("{}".to_string());
    if let Err(e) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_file_path)
        .and_then(|mut file| {
            use std::io::Write;
            writeln!(file, "{}", log_data)
        }) {
        trace_warn(format!("Failed to write log to {}: {}", log_file_path, e));
    }
}