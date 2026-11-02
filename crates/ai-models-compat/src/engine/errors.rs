//! Chat Completions provider-event classification.

use ai_interface::{ErrorKind, ModelError};
use ai_models_core::{ErrorContext, classify_stream_error};
use serde_json::Value;

use crate::ChatCompletionsStreamError;

/// Classifies a standard Chat Completions error event using stream progress.
pub fn classify_chat_completions_provider_error(
    context: &ErrorContext,
    events_received: u64,
    error: Value,
) -> ModelError {
    if events_received > 0 {
        let source = ChatCompletionsStreamError::ProviderEvent { error };
        return classify_stream_error(context, events_received, &source);
    }
    let code = error
        .get("code")
        .or_else(|| error.get("type"))
        .or_else(|| error.get("status"));
    let message = error
        .get("message")
        .and_then(Value::as_str)
        .unwrap_or("provider stream reported an error")
        .to_owned();
    if matches!(code.and_then(Value::as_u64), Some(401..=403))
        || matches!(
            code.and_then(Value::as_str),
            Some(
                "authentication_error"
                    | "invalid_api_key"
                    | "invalid_authentication"
                    | "permission_error"
                    | "permission_denied"
                    | "billing_error"
                    | "insufficient_balance"
                    | "insufficient_quota"
                    | "account_deactivated"
            )
        )
    {
        return context.model_error(ErrorKind::RejectedKey, message);
    }
    if code.and_then(Value::as_u64) == Some(404)
        || matches!(
            code.and_then(Value::as_str),
            Some("model_not_found" | "unknown_model" | "not_found_error")
        )
    {
        return context.model_error(ErrorKind::UnknownModel, message);
    }
    if matches!(
        code.and_then(Value::as_str),
        Some("unsupported_input" | "unsupported_media_type" | "unsupported_image_media_type")
    ) {
        return context.model_error(ErrorKind::UnsupportedInput, message);
    }
    if matches!(
        code.and_then(Value::as_str),
        Some("invalid_output" | "invalid_response")
    ) {
        return context.model_error(ErrorKind::InvalidOutput, message);
    }
    if code.and_then(Value::as_u64) == Some(429)
        || matches!(
            code.and_then(Value::as_str),
            Some("rate_limit_error" | "rate_limit_exceeded" | "rate_limited")
        )
    {
        return context.model_error(ErrorKind::RateLimited, message);
    }
    if matches!(
        code.and_then(Value::as_str),
        Some(
            "context_length_exceeded"
                | "model_context_window_exceeded"
                | "input_too_long"
                | "too_many_tokens"
        )
    ) {
        return context.model_error(ErrorKind::ContextLimitExceeded, message);
    }
    if code.and_then(Value::as_u64).is_some_and(|code| code >= 500)
        || matches!(
            code.and_then(Value::as_str),
            Some(
                "internal_error"
                    | "server_error"
                    | "overloaded"
                    | "temporarily_unavailable"
                    | "service_unavailable"
            )
        )
    {
        return context.model_error(ErrorKind::Transient, message);
    }
    context.model_error(ErrorKind::InvalidRequest, message)
}

#[cfg(test)]
#[path = "_tests_/stream_errors_tests.rs"]
mod stream_errors_tests;
