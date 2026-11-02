//! Google-owned optional request-id aliases and canonical operation status mapping.

use std::sync::Arc;

use ai_models_core::{Clock, ErrorContext};
use serde_json::Value;

pub(super) fn new(model: &str, clock: Arc<dyn Clock>) -> ErrorContext {
    ErrorContext::new(
        "google",
        model,
        &["x-goog-request-id", "x-request-id", "request-id"],
        clock,
    )
}

pub(super) fn embedded_status(body: &Value) -> Option<u16> {
    let code = body.get("error")?.get("code")?.as_u64()?;
    u16::try_from(code).ok().filter(|status| *status >= 400)
}

pub(super) fn operation_status(code: Option<i64>, status: Option<&str>) -> Option<u16> {
    match code {
        Some(400..=65535) => code.and_then(|code| u16::try_from(code).ok()),
        Some(1) => Some(499),
        Some(2 | 13 | 15) => Some(500),
        Some(3 | 9 | 11) => Some(400),
        Some(4) => Some(504),
        Some(5) => Some(404),
        Some(6 | 10) => Some(409),
        Some(7) => Some(403),
        Some(8) => Some(429),
        Some(12) => Some(501),
        Some(14) => Some(503),
        Some(16) => Some(401),
        _ => match status {
            Some("CANCELLED") => Some(499),
            Some("UNKNOWN" | "INTERNAL" | "DATA_LOSS") => Some(500),
            Some(
                "INVALID_ARGUMENT"
                | "FAILED_PRECONDITION"
                | "OUT_OF_RANGE"
                | "context_length_exceeded"
                | "model_context_window_exceeded"
                | "input_too_long"
                | "too_many_tokens",
            ) => Some(400),
            Some("DEADLINE_EXCEEDED") => Some(504),
            Some("NOT_FOUND") => Some(404),
            Some("ALREADY_EXISTS" | "ABORTED") => Some(409),
            Some("PERMISSION_DENIED") => Some(403),
            Some("RESOURCE_EXHAUSTED") => Some(429),
            Some("UNIMPLEMENTED") => Some(501),
            Some("UNAVAILABLE") => Some(503),
            Some("UNAUTHENTICATED") => Some(401),
            _ => None,
        },
    }
}
