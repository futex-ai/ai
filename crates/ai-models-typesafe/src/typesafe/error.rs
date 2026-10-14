//! TypeSafe judgment error classification.

use ai_interface::JudgmentError;
use ai_models_core::{classify_judgment_status, http_status_message};
use serde_json::Value;

const PROVIDER: &str = "typesafe";

/// Classifies one unsuccessful System One HTTP response.
///
/// TypeSafe reports failures as `{"detail": {"message": ...}}` or
/// `{"detail": "..."}`; any other body is retained as compact JSON or text.
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
    classify_judgment_status(PROVIDER, status, model_id, &message, secrets)
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
