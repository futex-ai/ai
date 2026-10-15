//! TypeSafe System One failure-message extraction.

use serde_json::Value;

/// Extracts the provider message from a decoded System One failure body.
///
/// TypeSafe reports failures as `{"detail": {"message": ...}}` or
/// `{"detail": "..."}`; any other body is retained as compact JSON or text.
pub(super) fn failure_message(body: &Value) -> String {
    let detail = body.get("detail");
    if let Some(message) = detail
        .and_then(Value::as_object)
        .and_then(|object| object.get("message"))
        .and_then(Value::as_str)
    {
        return message.to_owned();
    }
    if let Some(message) = detail.and_then(Value::as_str) {
        return message.to_owned();
    }
    match body {
        Value::String(message) => message.clone(),
        body => body.to_string(),
    }
}
