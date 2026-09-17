//! TypeSafe judgment error classification.

use ai_interface::JudgmentError;
use ai_models_core::{HttpFailureClass, classify_http_status};
use serde_json::Value;

const PROVIDER: &str = "typesafe";

/// Classifies one unsuccessful System One HTTP response.
pub(super) fn classify_status(status: u16, model_id: &str, body: &Value) -> JudgmentError {
    let message = error_message(body).unwrap_or_else(|| format!("HTTP {status}"));
    match classify_http_status(status) {
        Some(HttpFailureClass::RateLimited) => {
            JudgmentError::rate_limited(PROVIDER, model_id, message)
        }
        Some(HttpFailureClass::Transient) => {
            JudgmentError::transient_provider(PROVIDER, model_id, message)
        }
        Some(HttpFailureClass::Terminal) | None => {
            JudgmentError::provider(PROVIDER, model_id, message)
        }
    }
}

/// Classifies a failure while building or dispatching a System One request.
pub(super) fn classify_request_error(source: json_http::Error, model_id: &str) -> JudgmentError {
    match source {
        source @ (json_http::Error::Transport { .. }
        | json_http::Error::ReqwestTransport { .. }
        | json_http::Error::Auth { .. }) => {
            JudgmentError::transient_provider(PROVIDER, model_id, source.to_string())
        }
        source @ (json_http::Error::SerializeRequest { .. }
        | json_http::Error::DeserializeResponse { .. }
        | json_http::Error::ClientInitialization { .. }
        | json_http::Error::SseUnsupported
        | json_http::Error::HttpStatus { .. }
        | json_http::Error::InvalidSseContentType { .. }
        | json_http::Error::IdleTimeout { .. }
        | json_http::Error::DeadlineExceeded { .. }
        | json_http::Error::SseTransport { .. }
        | json_http::Error::SseDecode { .. }) => JudgmentError::internal(source),
    }
}

fn error_message(body: &Value) -> Option<String> {
    let detail = body.get("detail");
    if let Some(message) = detail
        .and_then(Value::as_object)
        .and_then(|object| object.get("message"))
        .and_then(Value::as_str)
    {
        return Some(message.to_owned());
    }
    if let Some(message) = detail.and_then(Value::as_str) {
        return Some(message.to_owned());
    }
    match body {
        Value::Null => None,
        Value::String(message) => Some(message.clone()),
        body => Some(body.to_string()),
    }
}
