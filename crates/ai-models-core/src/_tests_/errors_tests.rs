use std::time::Duration;

use ai_interface::{ErrorKind, ModelError};
use json_http::JsonHttpSseDecoder;
use reqwest::{Error, IntoUrl};
use serde_json::json;

use super::error_context_tests::context;

use crate::{classify_json_http_error, classify_json_http_stream_error};

#[test]
fn classifies_structured_context_limit_errors() {
    let error = classify_json_http_error(
        &context("openai", "gpt"),
        400,
        &Default::default(),
        &json!({
            "error": {
                "code": "context_length_exceeded",
                "message": "request is too large"
            }
        }),
    );

    assert_eq!(error.core().kind, ErrorKind::ContextLimitExceeded);
}

#[test]
fn does_not_classify_broad_invalid_argument_as_context_limit() {
    let error = classify_json_http_error(
        &context("google", "gemini"),
        400,
        &Default::default(),
        &json!({
            "error": {
                "status": "INVALID_ARGUMENT",
                "message": "invalid schema"
            }
        }),
    );

    assert_eq!(error.core().kind, ErrorKind::InvalidRequest);
}

#[test]
fn classifies_conflict_errors_as_transient_provider_failures() {
    let error = classify_json_http_error(
        &context("openai", "gpt"),
        409,
        &Default::default(),
        &json!({
            "error": {
                "message": "request conflict"
            }
        }),
    );

    assert_eq!(error.core().kind, ErrorKind::Transient);
}

#[test]
fn preserves_http_status_classification_for_stream_open_failures() {
    let error = classify_json_http_stream_error(
        &context("openai", "gpt"),
        0,
        json_http::Error::HttpStatus {
            headers: Default::default(),
            status: 429,
            body: json!({"error": {"message": "slow down"}}),
        },
    );

    assert_eq!(error.core().kind, ErrorKind::RateLimited);
}

#[test]
fn classifies_transport_failure_by_observed_progress() {
    let before = classify_json_http_stream_error(
        &context("deepseek", "reasoner"),
        0,
        json_http::Error::transport("connection reset"),
    );
    let after = classify_json_http_stream_error(
        &context("deepseek", "reasoner"),
        1,
        json_http::Error::transport("connection reset"),
    );

    assert_eq!(before.core().kind, ErrorKind::Transient);
    assert_eq!(after.core().kind, ErrorKind::Interrupted);
}

#[test]
fn uses_timeout_progress_reported_by_the_transport() {
    let before = classify_json_http_stream_error(
        &context("anthropic", "claude"),
        0,
        json_http::Error::IdleTimeout {
            idle: Duration::from_secs(120),
            events_received: 0,
        },
    );
    let after = classify_json_http_stream_error(
        &context("anthropic", "claude"),
        0,
        json_http::Error::DeadlineExceeded {
            timeout: Duration::from_secs(3_600),
            events_received: 3,
        },
    );

    assert_eq!(before.core().kind, ErrorKind::Transient);
    assert_eq!(after.core().kind, ErrorKind::Interrupted);
}

#[test]
fn uses_decode_progress_reported_by_the_transport() {
    let mut decoder = JsonHttpSseDecoder::new();
    decoder.push(&[0xff, b'\n', b'\n']);
    let source = decoder
        .next_event(false)
        .expect_err("fixture should contain invalid UTF-8");
    let error = classify_json_http_stream_error(
        &context("google", "gemini"),
        0,
        json_http::Error::SseDecode {
            events_received: 2,
            source,
        },
    );

    assert_eq!(error.core().kind, ErrorKind::Interrupted);
}

#[test]
fn classifies_unsupported_streaming_as_internal() {
    let error = classify_json_http_stream_error(
        &context("kimi", "kimi-k2"),
        0,
        json_http::Error::SseUnsupported,
    );

    assert!(matches!(error, ModelError::Internal { core, .. } if core.kind == ErrorKind::Internal));
}

#[test]
fn status_kinds_and_context_override_follow_the_exact_error_contract() {
    let context = context("provider", "model");
    for (status, kind) in [
        (400, ErrorKind::InvalidRequest),
        (401, ErrorKind::RejectedKey),
        (402, ErrorKind::RejectedKey),
        (403, ErrorKind::RejectedKey),
        (404, ErrorKind::UnknownModel),
        (408, ErrorKind::Transient),
        (409, ErrorKind::Transient),
        (413, ErrorKind::InvalidRequest),
        (422, ErrorKind::InvalidRequest),
        (425, ErrorKind::Transient),
        (429, ErrorKind::RateLimited),
        (500, ErrorKind::Transient),
        (599, ErrorKind::Transient),
    ] {
        let error = classify_json_http_error(
            &context,
            status,
            &Default::default(),
            &json!({"error":{"message":"failure"}}),
        );
        assert_eq!(error.core().kind, kind, "HTTP {status}");
    }
    for (status, expected) in [
        (400, ErrorKind::ContextLimitExceeded),
        (401, ErrorKind::RejectedKey),
        (404, ErrorKind::UnknownModel),
        (422, ErrorKind::InvalidRequest),
        (429, ErrorKind::RateLimited),
    ] {
        let error = classify_json_http_error(
            &context,
            status,
            &Default::default(),
            &json!({"error":{"code":"context_length_exceeded", "message":"too much"}}),
        );
        assert_eq!(error.core().kind, expected, "HTTP {status}");
    }
}

#[test]
fn transport_diagnostics_omit_signed_urls_and_retain_original_sources() {
    let url = "https://downloads.example/video?signature=download-secret";
    for streamed in [false, true] {
        let source = reqwest_source("://invalid").with_url(url.parse().unwrap());
        let source = if streamed {
            json_http::Error::SseTransport {
                events_received: 3,
                source,
            }
        } else {
            json_http::Error::ReqwestTransport { source }
        };
        let context = context("provider", "model");
        let error = classify_json_http_stream_error(&context, 0, source);
        assert!(
            !error.core().message.contains("download-secret"),
            "{}",
            error.core().message
        );
        assert_eq!(
            error.core().kind,
            if streamed {
                ErrorKind::Interrupted
            } else {
                ErrorKind::Transient
            }
        );
        let ModelError::Internal { source, .. } = error else {
            panic!("HTTP source lost");
        };
        let source = source
            .raw_cause()
            .downcast_ref::<json_http::Error>()
            .unwrap();
        let original = match source.cause() {
            json_http::Error::ReqwestTransport { source }
            | json_http::Error::SseTransport { source, .. } => source,
            _ => panic!("typed reqwest source lost"),
        };
        assert_eq!(original.url().unwrap().as_str(), url);
        assert!(
            !context
                .request_error(source)
                .message
                .contains("download-secret")
        );
    }
}

fn reqwest_source(url: impl IntoUrl) -> Error {
    url.into_url().unwrap_err()
}
