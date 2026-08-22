use std::time::Duration;

use ai_interface::ModelError;
use json_http::JsonHttpSseDecoder;
use serde_json::json;

use crate::{
    ChatCompletionsStreamError, classify_json_http_error, classify_json_http_stream_error,
    classify_stream_error,
};

#[test]
fn classifies_structured_context_limit_errors() {
    let error = classify_json_http_error(
        "openai",
        "gpt",
        400,
        &json!({
            "error": {
                "code": "context_length_exceeded",
                "message": "request is too large"
            }
        }),
    );

    assert!(matches!(error, ModelError::ContextLimitExceeded { .. }));
}

#[test]
fn does_not_classify_broad_invalid_argument_as_context_limit() {
    let error = classify_json_http_error(
        "google",
        "gemini",
        400,
        &json!({
            "error": {
                "status": "INVALID_ARGUMENT",
                "message": "invalid schema"
            }
        }),
    );

    assert!(matches!(error, ModelError::Provider { .. }));
}

#[test]
fn classifies_conflict_errors_as_transient_provider_failures() {
    let error = classify_json_http_error(
        "openai",
        "gpt",
        409,
        &json!({
            "error": {
                "message": "request conflict"
            }
        }),
    );

    assert!(matches!(error, ModelError::TransientProvider { .. }));
}

#[test]
fn preserves_http_status_classification_for_stream_open_failures() {
    let error = classify_json_http_stream_error(
        "openai",
        "gpt",
        0,
        json_http::Error::HttpStatus {
            status: 429,
            body: json!({"error": {"message": "slow down"}}),
        },
    );

    assert!(matches!(error, ModelError::RateLimited { .. }));
}

#[test]
fn classifies_transport_failure_by_observed_progress() {
    let before = classify_json_http_stream_error(
        "deepseek",
        "reasoner",
        0,
        json_http::Error::transport("connection reset"),
    );
    let after = classify_json_http_stream_error(
        "deepseek",
        "reasoner",
        1,
        json_http::Error::transport("connection reset"),
    );

    assert!(matches!(before, ModelError::TransientProvider { .. }));
    assert!(matches!(after, ModelError::Interrupted { .. }));
}

#[test]
fn uses_timeout_progress_reported_by_the_transport() {
    let before = classify_json_http_stream_error(
        "anthropic",
        "claude",
        0,
        json_http::Error::IdleTimeout {
            idle: Duration::from_secs(120),
            events_received: 0,
        },
    );
    let after = classify_json_http_stream_error(
        "anthropic",
        "claude",
        0,
        json_http::Error::DeadlineExceeded {
            timeout: Duration::from_secs(3_600),
            events_received: 3,
        },
    );

    assert!(matches!(before, ModelError::TransientProvider { .. }));
    assert!(matches!(after, ModelError::Interrupted { .. }));
}

#[test]
fn uses_decode_progress_reported_by_the_transport() {
    let mut decoder = JsonHttpSseDecoder::new();
    decoder.push(&[0xff, b'\n', b'\n']);
    let source = decoder
        .next_event(false)
        .expect_err("fixture should contain invalid UTF-8");
    let error = classify_json_http_stream_error(
        "google",
        "gemini",
        0,
        json_http::Error::SseDecode {
            events_received: 2,
            source,
        },
    );

    assert!(matches!(error, ModelError::Interrupted { .. }));
}

#[test]
fn classifies_unsupported_streaming_as_internal() {
    let error =
        classify_json_http_stream_error("kimi", "kimi-k2", 0, json_http::Error::SseUnsupported);

    assert!(matches!(error, ModelError::Internal { .. }));
}

#[test]
fn classifies_eof_and_native_errors_by_provider_event_progress() {
    let eof = ChatCompletionsStreamError::MissingDone;
    let before = classify_stream_error("qwen", "qwen-plus", 0, &eof);
    let native = std::io::Error::other("provider error event");
    let after = classify_stream_error("qwen", "qwen-plus", 2, &native);

    assert!(matches!(before, ModelError::TransientProvider { .. }));
    assert!(matches!(after, ModelError::Interrupted { .. }));
}
