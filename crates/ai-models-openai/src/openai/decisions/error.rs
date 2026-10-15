//! OpenAI Decisions failure-message extraction.

use serde_json::Value;

/// Extracts the provider message from a decoded Decisions failure body.
///
/// OpenAI reports failures as `{"error": {"message": ...}}`. A JSON string
/// body is used directly, and any other body is retained as compact JSON.
pub(super) fn failure_message(body: &Value) -> String {
    if let Some(message) = body
        .get("error")
        .and_then(|error| error.get("message"))
        .and_then(Value::as_str)
    {
        return message.to_owned();
    }
    match body {
        Value::String(message) => message.clone(),
        body => body.to_string(),
    }
}
