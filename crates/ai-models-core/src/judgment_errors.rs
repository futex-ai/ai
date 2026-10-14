//! Provider-neutral judgment error classification and diagnostic redaction.
//!
//! Judgment adapters own their wire mapping and error-message extraction.
//! The helpers here turn an extracted message, a transport failure, or a
//! decoded answer problem into the shared `JudgmentError` contract while
//! redacting the credentials that the adapter transmitted.

use ai_interface::{JudgmentAnswerProblem, JudgmentError};
use serde_json::Value;

use crate::{HttpFailureClass, classify_http_status, redact_secrets};

/// Fixed auth failure diagnostic used because hook errors can contain
/// undiscovered credentials and therefore cannot be safely sanitized.
pub const AUTH_HOOK_FAILED: &str = "authentication hook failed";

/// Builds the fallback diagnostic for a failure response without a message.
pub fn http_status_message(status: u16) -> String {
    let status = status.to_string();
    ["HTTP ", &status].concat()
}

/// Decodes an unsuccessful response body for error-message extraction.
///
/// JSON bodies decode as JSON, other bodies become lossy UTF-8 text, and an
/// empty body becomes the `HTTP <status>` fallback text.
pub fn decode_failure_body(status: u16, body: &[u8]) -> Value {
    if body.is_empty() {
        return Value::String(http_status_message(status));
    }
    match serde_json::from_slice::<Value>(body) {
        Ok(body) => body,
        Err(_) => Value::String(String::from_utf8_lossy(body).into_owned()),
    }
}

/// Classifies one unsuccessful judgment HTTP status with its extracted message.
///
/// The message is redacted before the error is built. Status 429 is
/// `RateLimited`; 408, 409, 425, and 5xx are `TransientProvider`; every other
/// status is a terminal `Provider` error.
pub fn classify_judgment_status(
    provider: &str,
    status: u16,
    model_id: &str,
    message: &str,
    secrets: &[String],
) -> JudgmentError {
    let message = redact_secrets(message, secrets);
    match classify_http_status(status) {
        Some(HttpFailureClass::RateLimited) => {
            JudgmentError::rate_limited(provider, model_id, message)
        }
        Some(HttpFailureClass::Transient) => {
            JudgmentError::transient_provider(provider, model_id, message)
        }
        Some(HttpFailureClass::Terminal) | None => {
            JudgmentError::provider(provider, model_id, message)
        }
    }
}

/// Classifies a `json-http` request failure for a judgment call.
///
/// Auth-hook failures use the fixed [`AUTH_HOOK_FAILED`] diagnostic. Local
/// serialization and client initialization failures are internal because they
/// cannot contain auth headers. Every other diagnostic is redacted.
pub fn classify_judgment_request_error(
    provider: &str,
    source: json_http::Error,
    model_id: &str,
    secrets: &[String],
) -> JudgmentError {
    match source {
        json_http::Error::Auth { .. } => {
            JudgmentError::transient_provider(provider, model_id, AUTH_HOOK_FAILED)
        }
        source @ (json_http::Error::Transport { .. }
        | json_http::Error::ReqwestTransport { .. }
        | json_http::Error::IdleTimeout { .. }
        | json_http::Error::DeadlineExceeded { .. }
        | json_http::Error::SseTransport { .. }) => {
            let message = redact_secrets(&source.to_string(), secrets);
            JudgmentError::transient_provider(provider, model_id, message)
        }
        source @ (json_http::Error::SerializeRequest { .. }
        | json_http::Error::ClientInitialization { .. }) => JudgmentError::internal(source),
        source @ (json_http::Error::DeserializeResponse { .. }
        | json_http::Error::SseUnsupported
        | json_http::Error::HttpStatus { .. }
        | json_http::Error::InvalidSseContentType { .. }
        | json_http::Error::SseDecode { .. }) => {
            let message = redact_secrets(&source.to_string(), secrets);
            JudgmentError::provider(provider, model_id, message)
        }
    }
}

/// Redacts provider diagnostics produced while decoding a successful body.
///
/// `Provider` messages and the provider-controlled labels in `UnknownOption`
/// and `MissingOption` answer problems are redacted. Every other error is
/// returned unchanged because it carries no provider text.
pub fn redact_judgment_error(error: JudgmentError, secrets: &[String]) -> JudgmentError {
    match error {
        JudgmentError::Provider {
            provider,
            model_id,
            message,
        } => JudgmentError::provider(provider, model_id, redact_secrets(&message, secrets)),
        JudgmentError::InvalidAnswer {
            provider,
            model_id,
            id,
            problem,
        } => JudgmentError::invalid_answer(
            provider,
            model_id,
            id,
            redact_answer_problem(problem, secrets),
        ),
        error => error,
    }
}

/// Redacts the only answer problems that can carry provider-controlled text.
///
/// `UnknownOption` and `MissingOption` echo labels from the provider body.
/// `SelectedNotMaximal` labels have already passed membership against the
/// caller's own options, so they cannot carry provider content.
fn redact_answer_problem(
    problem: JudgmentAnswerProblem,
    secrets: &[String],
) -> JudgmentAnswerProblem {
    match problem {
        JudgmentAnswerProblem::UnknownOption { label } => JudgmentAnswerProblem::UnknownOption {
            label: redact_secrets(&label, secrets),
        },
        JudgmentAnswerProblem::MissingOption { label } => JudgmentAnswerProblem::MissingOption {
            label: redact_secrets(&label, secrets),
        },
        problem => problem,
    }
}

#[cfg(test)]
#[path = "_tests_/judgment_errors_tests.rs"]
mod judgment_errors_tests;

#[cfg(test)]
#[path = "_tests_/judgment_error_redaction_tests.rs"]
mod judgment_error_redaction_tests;
