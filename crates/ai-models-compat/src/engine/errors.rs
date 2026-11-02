//! Chat Completions provider-event classification.

use ai_interface::ModelError;
use ai_models_core::classify_stream_error;
use serde_json::Value;

use crate::ChatCompletionsStreamError;

/// Classifies a standard Chat Completions error event using stream progress.
pub fn classify_chat_completions_provider_error(
    provider: &str,
    model_id: &str,
    events_received: u64,
    error: Value,
) -> ModelError {
    if events_received > 0 {
        let source = ChatCompletionsStreamError::ProviderEvent { error };
        return classify_stream_error(provider, model_id, events_received, &source);
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
    if code.and_then(Value::as_u64) == Some(429)
        || matches!(
            code.and_then(Value::as_str),
            Some("rate_limit_error" | "rate_limit_exceeded" | "rate_limited")
        )
    {
        return ModelError::rate_limited(provider, model_id, message);
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
        return ModelError::context_limit_exceeded(provider, model_id, message);
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
        return ModelError::transient_provider(provider, model_id, message);
    }
    ModelError::provider(provider, model_id, message)
}
