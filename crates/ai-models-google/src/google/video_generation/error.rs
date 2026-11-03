//! Google video HTTP classification through the shared error contract.

use ai_interface::VideoGenerationError;
use ai_models_core::{ErrorContext, json_http_error_core};
use json_http::JsonHttpHeaders;
use serde_json::Value;

pub(super) fn classify_status(
    context: &ErrorContext,
    status: u16,
    headers: &JsonHttpHeaders,
    body: &Value,
) -> VideoGenerationError {
    let message = body
        .get("error")
        .and_then(|error| error.get("message"))
        .and_then(Value::as_str)
        .or_else(|| body.as_str())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| format!("HTTP {status}"));
    let mut core = json_http_error_core(context, status, headers, body);
    core.message = context.redact(&message);
    VideoGenerationError::new(core)
}

pub(super) fn classify_request_error(
    context: &ErrorContext,
    source: json_http::Error,
) -> VideoGenerationError {
    VideoGenerationError::with_source(context.request_error(&source), context.source(source))
}
