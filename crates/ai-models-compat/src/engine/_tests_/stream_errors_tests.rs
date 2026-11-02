//! Shared error classification over the moved accumulator error contract.

use ai_interface::ErrorKind;

use crate::ChatCompletionsStreamError;
use crate::classify_chat_completions_provider_error;
use ai_models_core::{ErrorContext, SystemClock, classify_stream_error};
use serde_json::json;
use std::sync::Arc;

#[test]
fn classifies_eof_and_native_errors_by_provider_event_progress() {
    let context = ErrorContext::new("qwen", "qwen-plus", &[], Arc::new(SystemClock));
    let eof = ChatCompletionsStreamError::MissingDone;
    let before = classify_stream_error(&context, 0, &eof);
    let native = std::io::Error::other("provider error event");
    let after = classify_stream_error(&context, 2, &native);

    assert!(matches!(before.core().kind, ErrorKind::Transient));
    assert!(matches!(after.core().kind, ErrorKind::Interrupted));
    for (code, expected) in [
        (json!(401), ErrorKind::RejectedKey),
        (json!(402), ErrorKind::RejectedKey),
        (json!(403), ErrorKind::RejectedKey),
        (json!("authentication_error"), ErrorKind::RejectedKey),
        (json!("invalid_api_key"), ErrorKind::RejectedKey),
        (json!("invalid_authentication"), ErrorKind::RejectedKey),
        (json!("permission_error"), ErrorKind::RejectedKey),
        (json!("permission_denied"), ErrorKind::RejectedKey),
        (json!("billing_error"), ErrorKind::RejectedKey),
        (json!("insufficient_balance"), ErrorKind::RejectedKey),
        (json!("insufficient_quota"), ErrorKind::RejectedKey),
        (json!("account_deactivated"), ErrorKind::RejectedKey),
        (json!(404), ErrorKind::UnknownModel),
        (json!("model_not_found"), ErrorKind::UnknownModel),
        (json!("unknown_model"), ErrorKind::UnknownModel),
        (json!("not_found_error"), ErrorKind::UnknownModel),
        (json!("unsupported_input"), ErrorKind::UnsupportedInput),
        (json!("unsupported_media_type"), ErrorKind::UnsupportedInput),
        (
            json!("unsupported_image_media_type"),
            ErrorKind::UnsupportedInput,
        ),
        (json!("invalid_output"), ErrorKind::InvalidOutput),
        (json!("invalid_response"), ErrorKind::InvalidOutput),
        (json!(429), ErrorKind::RateLimited),
        (json!("rate_limit_error"), ErrorKind::RateLimited),
        (json!("rate_limit_exceeded"), ErrorKind::RateLimited),
        (json!("rate_limited"), ErrorKind::RateLimited),
        (
            json!("context_length_exceeded"),
            ErrorKind::ContextLimitExceeded,
        ),
        (
            json!("model_context_window_exceeded"),
            ErrorKind::ContextLimitExceeded,
        ),
        (json!("input_too_long"), ErrorKind::ContextLimitExceeded),
        (json!("too_many_tokens"), ErrorKind::ContextLimitExceeded),
        (json!(503), ErrorKind::Transient),
        (json!("overloaded"), ErrorKind::Transient),
        (json!("bad_request"), ErrorKind::InvalidRequest),
        (json!(null), ErrorKind::InvalidRequest),
    ] {
        let error = json!({"code":code,"message":"provider detail"});
        let before = classify_chat_completions_provider_error(&context, 0, error.clone());
        let after = classify_chat_completions_provider_error(&context, 1, error);
        assert_eq!(before.core().kind, expected);
        assert_eq!(before.core().message, "provider detail");
        assert_eq!(after.core().kind, ErrorKind::Interrupted);
    }

    for (error, expected) in [
        (
            json!({"code":"unknown","type":"authentication_error","message":"model_not_found"}),
            ErrorKind::InvalidRequest,
        ),
        (
            json!({"code":null,"type":"model_not_found"}),
            ErrorKind::InvalidRequest,
        ),
        (
            json!({"type":"invalid_api_key","status":404}),
            ErrorKind::RejectedKey,
        ),
        (json!({"status":404}), ErrorKind::UnknownModel),
    ] {
        let before = classify_chat_completions_provider_error(&context, 0, error.clone());
        let after = classify_chat_completions_provider_error(&context, 1, error);
        assert_eq!(before.core().kind, expected);
        assert_eq!(after.core().kind, ErrorKind::Interrupted);
    }
}
