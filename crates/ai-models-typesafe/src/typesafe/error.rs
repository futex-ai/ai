//! TypeSafe judgment error classification.

use ai_interface::JudgmentError;
use ai_models_core::{HttpFailureClass, classify_http_status};
use serde_json::Value;

use super::redaction::redact_secrets;

const PROVIDER: &str = "typesafe";

/// Classifies one unsuccessful System One HTTP response.
pub(super) fn classify_status(
    status: u16,
    model_id: &str,
    body: &Value,
    secrets: &[String],
) -> JudgmentError {
    let message = match error_message(body) {
        Some(message) => message,
        None => http_status_message(status),
    };
    let message = redact_secrets(&message, secrets);
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

/// Classifies request failures while reserving opaque internal errors for local
/// serialization and client initialization, which cannot contain auth headers.
pub(super) fn classify_request_error(
    source: json_http::Error,
    model_id: &str,
    secrets: &[String],
) -> JudgmentError {
    match source {
        source @ (json_http::Error::Transport { .. }
        | json_http::Error::ReqwestTransport { .. }
        | json_http::Error::Auth { .. }
        | json_http::Error::IdleTimeout { .. }
        | json_http::Error::DeadlineExceeded { .. }
        | json_http::Error::SseTransport { .. }) => {
            let message = redact_secrets(&source.to_string(), secrets);
            JudgmentError::transient_provider(PROVIDER, model_id, message)
        }
        source @ (json_http::Error::SerializeRequest { .. }
        | json_http::Error::ClientInitialization { .. }) => JudgmentError::internal(source),
        source @ (json_http::Error::DeserializeResponse { .. }
        | json_http::Error::SseUnsupported
        | json_http::Error::HttpStatus { .. }
        | json_http::Error::InvalidSseContentType { .. }
        | json_http::Error::SseDecode { .. }) => {
            let message = redact_secrets(&source.to_string(), secrets);
            JudgmentError::provider(PROVIDER, model_id, message)
        }
    }
}

/// Builds the fallback diagnostic for an empty response body.
pub(super) fn http_status_message(status: u16) -> String {
    let status = status.to_string();
    ["HTTP ", &status].concat()
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
    Some(match body {
        Value::String(message) => message.clone(),
        body => body.to_string(),
    })
}
