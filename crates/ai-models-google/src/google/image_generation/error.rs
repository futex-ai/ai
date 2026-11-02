//! Google image HTTP error kinds and safe per-response facts.

use ai_interface::ImageGenerationError;
use ai_models_core::{ErrorContext, json_http_error_core};
use json_http::JsonHttpHeaders;
use serde_json::Value;

pub(super) fn classify_status(
    context: &ErrorContext,
    status: u16,
    headers: &JsonHttpHeaders,
    body: &Value,
) -> ImageGenerationError {
    let message = body
        .get("error")
        .and_then(|error| error.get("message"))
        .and_then(Value::as_str)
        .or_else(|| body.get("message").and_then(Value::as_str))
        .or_else(|| body.as_str())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| format!("HTTP {status}"));
    let mut core = json_http_error_core(context, status, headers, body);
    core.message = context.redact(&message);
    ImageGenerationError::new(core)
}

pub(super) fn classify_request_error(
    context: &ErrorContext,
    source: json_http::Error,
) -> ImageGenerationError {
    ImageGenerationError::with_source(context.request_error(&source), source)
}
