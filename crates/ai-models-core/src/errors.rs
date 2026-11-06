//! Safe JSON HTTP and stream failure classification through a call context.

use std::error;

use ai_interface::{ErrorCore, ErrorKind, ModelError};
use json_http::{Error, JsonHttpHeaders};
use serde_json::Value;

use crate::{ErrorContext, classify_http_status};

/// Normalizes nullable assistant text content into a string.
pub fn assistant_text(content: Option<String>) -> String {
    content.unwrap_or_default()
}

/// Maps a provider status and response body to precise shared error details.
pub fn classify_json_http_error(
    context: &ErrorContext,
    status: u16,
    headers: &JsonHttpHeaders,
    body: &Value,
) -> ModelError {
    ModelError::new(json_http_error_core(context, status, headers, body))
}

/// Maps one failed HTTP response to shared details for every modality.
pub fn json_http_error_core(
    context: &ErrorContext,
    status: u16,
    headers: &JsonHttpHeaders,
    body: &Value,
) -> ErrorCore {
    let context = context.for_response(headers);
    let kind = if status == 400 && context_limit_code(body) {
        ErrorKind::ContextLimitExceeded
    } else {
        classify_http_status(status).unwrap_or(ErrorKind::InvalidOutput)
    };
    let message = error_message(body)
        .map(|message| format!("HTTP {status}: {message}"))
        .unwrap_or_else(|| format!("HTTP {status}"));
    context.core(kind, message)
}

/// Maps an HTTP stream failure while retaining headers and original source progress.
pub fn classify_json_http_stream_error(
    context: &ErrorContext,
    events_received: u64,
    source: Error,
) -> ModelError {
    let context = match source.response_headers() {
        Some(headers) => context.for_response(headers),
        None => context.clone(),
    };
    let progress = events_received.max(stream_progress(source.cause()));
    let core = if source.is_transport_failure() {
        context.core(
            if progress == 0 {
                ErrorKind::Transient
            } else {
                ErrorKind::Interrupted
            },
            http_error_message(&source),
        )
    } else {
        match source.cause() {
            Error::HttpStatus {
                status,
                headers,
                body,
            } => json_http_error_core(
                &context,
                *status,
                source.response_headers().unwrap_or(headers),
                body,
            ),
            Error::SerializeRequest { .. }
            | Error::ClientInitialization { .. }
            | Error::SseUnsupported => context.core(ErrorKind::Internal, "internal error"),
            Error::Auth { .. } => context.core(ErrorKind::Transient, "authentication hook failed"),
            _ => context.core(
                if progress == 0 {
                    ErrorKind::Transient
                } else {
                    ErrorKind::Interrupted
                },
                http_error_message(&source),
            ),
        }
    };
    ModelError::with_source(core, context.source(source))
}

/// Maps a typed provider stream failure using provider-event progress.
pub fn classify_stream_error(
    context: &ErrorContext,
    events_received: u64,
    source: &(dyn error::Error + Send + Sync),
) -> ModelError {
    context.model_error(
        if events_received == 0 {
            ErrorKind::Transient
        } else {
            ErrorKind::Interrupted
        },
        source.to_string(),
    )
}

pub(crate) fn request_error_core(context: &ErrorContext, source: &Error) -> ErrorCore {
    if source.is_transport_failure() {
        return context.core(ErrorKind::Transient, http_error_message(source));
    }
    match source.cause() {
        Error::HttpStatus {
            status,
            headers,
            body,
        } => json_http_error_core(
            context,
            *status,
            source.response_headers().unwrap_or(headers),
            body,
        ),
        Error::Auth { .. } => context.core(ErrorKind::Transient, "authentication hook failed"),
        Error::DeserializeResponse { .. } => {
            context.core(ErrorKind::InvalidOutput, "invalid provider response")
        }
        Error::InvalidSseContentType { .. }
        | Error::SseDecode { .. }
        | Error::IdleTimeout { .. }
        | Error::DeadlineExceeded { .. } => {
            context.core(ErrorKind::Transient, http_error_message(source))
        }
        _ => context.core(ErrorKind::Internal, "internal error"),
    }
}

fn stream_progress(source: &Error) -> u64 {
    match source.cause() {
        Error::IdleTimeout {
            events_received, ..
        }
        | Error::DeadlineExceeded {
            events_received, ..
        }
        | Error::SseTransport {
            events_received, ..
        }
        | Error::SseDecode {
            events_received, ..
        } => *events_received,
        _ => 0,
    }
}

fn context_limit_code(body: &Value) -> bool {
    let error = body.get("error").unwrap_or(body);
    [
        error.get("code"),
        error.get("type"),
        error.get("status"),
        body.get("code"),
        body.get("status"),
    ]
    .into_iter()
    .flatten()
    .filter_map(Value::as_str)
    .any(|value| {
        matches!(
            value,
            "context_length_exceeded"
                | "model_context_window_exceeded"
                | "input_too_long"
                | "too_many_tokens"
        )
    })
}

fn error_message(body: &Value) -> Option<String> {
    body.get("error")
        .and_then(|error| error.get("message"))
        .and_then(Value::as_str)
        .or_else(|| body.get("error").and_then(Value::as_str))
        .or_else(|| body.get("message").and_then(Value::as_str))
        .or_else(|| body.as_str())
        .map(ToOwned::to_owned)
}

fn http_error_message(source: &Error) -> String {
    let message = source.to_string();
    let url = match source.cause() {
        Error::ReqwestTransport { source } | Error::SseTransport { source, .. } => source.url(),
        _ => None,
    };
    match url {
        Some(url) => message.replace(url.as_str(), "[redacted URL]"),
        None => message,
    }
}
