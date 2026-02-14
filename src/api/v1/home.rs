use crate::api::v1::main::ApiResult;

pub async fn default() -> ApiResult {
    // say hello to the anonymous user of organization X
    ApiResult::Json(format!("{{\"message\": \"Hello, anonymous user !\"}}"))
}
