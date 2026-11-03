//! TypeSafe judgment error classification.

use ai_interface::{ErrorKind, JudgmentError};
use ai_models_core::{ErrorContext, json_http_error_core};
use json_http::JsonHttpHeaders;
use serde_json::Value;

use super::redaction::redact_secrets;

/// Fixed auth failure diagnostic used because hook errors can contain
/// undiscovered credentials and therefore cannot be safely sanitized.
pub(super) const AUTH_HOOK_FAILED: &str = "authentication hook failed";

/// Classifies one unsuccessful System One HTTP response.
pub(super) fn classify_status(
    context: &ErrorContext,
    status: u16,
    headers: &JsonHttpHeaders,
    body: &Value,
    secrets: &[String],
) -> JudgmentError {
    let message = error_message(body).unwrap_or_else(|| http_status_message(status));
    let mut core = json_http_error_core(context, status, headers, body);
    if status == 400
        && matches!(
            body.get("detail")
                .and_then(|detail| detail.get("error_type"))
                .and_then(Value::as_str),
            Some(
                "context_length_exceeded"
                    | "model_context_window_exceeded"
                    | "input_too_long"
                    | "too_many_tokens"
            )
        )
    {
        core.kind = ErrorKind::ContextLimitExceeded;
    }
    core.message = redact_secrets(&message, secrets);
    JudgmentError::new(core)
}

/// Classifies one request failure without losing its original source or response headers.
pub(super) fn classify_request_error(
    source: json_http::Error,
    context: &ErrorContext,
    secrets: &[String],
) -> JudgmentError {
    if matches!(source.cause(), json_http::Error::Auth { .. }) {
        return JudgmentError::new(context.request_error(&source));
    }
    let core = if let json_http::Error::HttpStatus {
        status,
        headers,
        body,
    } = source.cause()
    {
        classify_status(
            context,
            *status,
            source.response_headers().unwrap_or(headers),
            body,
            secrets,
        )
        .core()
        .clone()
    } else {
        context.request_error(&source)
    };
    JudgmentError::with_source(core, context.source(source))
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

#[cfg(test)]
#[path = "_tests_/error_tests.rs"]
mod error_tests;
