//! OpenAI Decisions error classification.

use ai_interface::JudgmentError;
use ai_models_core::classify_judgment_status;
use serde_json::Value;

const PROVIDER: &str = "openai";

/// Classifies one unsuccessful Decisions HTTP response.
///
/// OpenAI reports failures as `{"error": {"message": ...}}`. A JSON string
/// body is used directly, and any other body is retained as compact JSON.
pub(super) fn classify_status(
    status: u16,
    model_id: &str,
    body: &Value,
    secrets: &[String],
) -> JudgmentError {
    let message = error_message(body);
    classify_judgment_status(PROVIDER, status, model_id, &message, secrets)
}

fn error_message(body: &Value) -> String {
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
