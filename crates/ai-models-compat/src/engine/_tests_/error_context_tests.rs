//! Actual dialect status, stream, and terminal-output metadata boundaries.

use std::{
    error::Error,
    sync::Arc,
    time::{Duration, SystemTime},
};

use ai_interface::{ErrorKind, SafeErrorSource, StructuredOutputSchema};
use ai_models_core::{
    ClockMock,
    test_support::{SseFixture, done_event, event, recording_streaming_client},
};
use json_http::StaticHeaderAuth;
use serde_json::json;
use unimock::{MockFn, Unimock, matching};

use super::error_support::{assert_metadata, headers, model, no_clock, request};

#[tokio::test]
async fn dialect_http_errors_keep_provider_detail_identity_and_response_metadata() {
    for (provider, body, message) in [
        (
            "deepseek",
            json!({"error":{"message":"limited secret-status"}}),
            "HTTP 429: limited [redacted]",
        ),
        (
            "kimi",
            json!({"error":{"message":"limited secret-status"}}),
            "HTTP 429: limited [redacted]",
        ),
        (
            "qwen",
            json!({"error":{"message":"limited secret-status"}}),
            "HTTP 429: limited [redacted]",
        ),
        (
            "minimax",
            json!({"base_resp":{"status_code":1002,"status_msg":"limited secret-status"}}),
            "HTTP 429: base_resp status_code 1002: limited [redacted]",
        ),
        (
            "xai",
            json!({"code":"TooManyRequests","error":"limited secret-status"}),
            "HTTP 429: limited [redacted]",
        ),
    ] {
        let mut response_headers = headers("req-status", "Wed, 21 Oct 2015 07:28:10 GMT");
        if provider == "minimax" {
            response_headers.insert("trace-id", "req-trace".parse().unwrap());
        }
        let (client, requests) = recording_streaming_client(vec![SseFixture::OpeningError(
            json_http::Error::HttpStatus {
                status: 429,
                headers: Box::new(response_headers),
                body,
            },
        )]);
        let clock =
            Arc::new(Unimock::new(ClockMock::now.each_call(matching!()).returns(
                SystemTime::UNIX_EPOCH + Duration::from_secs(1_445_412_480),
            )));
        let model = model(
            provider,
            client,
            Arc::new(StaticHeaderAuth::bearer_token("secret-status")),
            clock,
        );
        let error = model.complete(&request()).await.unwrap_err();
        assert_eq!(error.core().kind, ErrorKind::RateLimited, "{provider}");
        assert_eq!(error.core().provider, provider);
        assert!(!error.core().model_id.is_empty());
        assert_metadata(
            &error,
            if provider == "minimax" {
                "req-trace"
            } else {
                "req-status"
            },
            10,
        );
        assert_eq!(error.core().message, message, "{provider}");
        assert!(!error.to_string().contains("secret-status"));
        assert!(!format!("{error:?}").contains("secret-status"));
        let requests = requests.lock().unwrap();
        assert_eq!(requests.len(), 1);
        assert_eq!(
            requests[0].headers.get("Authorization").map(String::as_str),
            Some("Bearer secret-status")
        );
    }
}

#[tokio::test]
async fn late_stream_and_output_failures_keep_response_facts_and_sources() {
    for (kind, schema, items) in [
        (
            ErrorKind::Interrupted,
            None,
            vec![
                event(json!({"choices":[{"index":0,"delta":{"content":"partial"}}]})),
                Err(json_http::Error::transport("reset secret-late")),
            ],
        ),
        (
            ErrorKind::Interrupted,
            None,
            vec![
                event(json!({"choices":[]})),
                event(json!({"error":{"code":"bad_request","message":"echo secret-late"}})),
            ],
        ),
        (
            ErrorKind::InvalidOutput,
            Some(json!({"type":"object"})),
            vec![
                event(
                    json!({"choices":[{"index":0,"delta":{"content":"secret-late"},"finish_reason":"stop"}],"usage":{}}),
                ),
                done_event(),
            ],
        ),
        (
            ErrorKind::InvalidRequest,
            Some(json!({"type":7})),
            vec![
                event(
                    json!({"choices":[{"index":0,"delta":{"content":"{}"},"finish_reason":"stop"}],"usage":{}}),
                ),
                done_event(),
            ],
        ),
    ] {
        let (http, _) = recording_streaming_client(vec![SseFixture::StreamWithHeaders {
            headers: headers("req-secret-late", "7"),
            items,
        }]);
        let model = model(
            "deepseek",
            http,
            Arc::new(StaticHeaderAuth::bearer_token("secret-late")),
            no_clock(),
        );
        let mut input = request();
        input.response_schema = schema.map(|schema| StructuredOutputSchema {
            name: "value".to_owned(),
            schema,
        });
        let error = model.complete(&input).await.unwrap_err();
        assert_eq!(error.core().kind, kind);
        assert_metadata(&error, "req-[redacted]", 7);
        assert!(!error.to_string().contains("secret-late"));
        assert!(!format!("{error:?}").contains("secret-late"));
        assert_eq!(
            error.source().is_some_and(|source| source
                .downcast_ref::<SafeErrorSource>()
                .unwrap()
                .raw_cause()
                .is::<serde_json::Error>()),
            kind == ErrorKind::InvalidOutput
        );
    }
}

#[tokio::test]
async fn read_failure_headers_take_priority_over_opening_metadata() {
    let error = json_http::Error::HttpStatus {
        status: 401,
        headers: Box::new(headers("req-inner", "2")),
        body: json!({"error":{"message":"rejected"}}),
    }
    .with_response_headers(headers("req-read", "9"));
    let (http, _) = recording_streaming_client(vec![SseFixture::StreamWithHeaders {
        headers: headers("req-opening", "1"),
        items: vec![Err(error)],
    }]);
    let model = model(
        "qwen",
        http,
        Arc::new(StaticHeaderAuth::default()),
        no_clock(),
    );
    let error = model.complete(&request()).await.unwrap_err();
    assert_eq!(error.core().kind, ErrorKind::RejectedKey);
    assert_metadata(&error, "req-read", 9);
    assert!(
        error
            .source()
            .unwrap()
            .downcast_ref::<SafeErrorSource>()
            .unwrap()
            .raw_cause()
            .is::<json_http::Error>()
    );
}

#[tokio::test]
async fn first_provider_events_keep_precise_kinds_and_response_metadata() {
    for (code, kind) in [
        ("invalid_api_key", ErrorKind::RejectedKey),
        ("model_not_found", ErrorKind::UnknownModel),
        ("unsupported_media_type", ErrorKind::UnsupportedInput),
        ("invalid_response", ErrorKind::InvalidOutput),
    ] {
        let (http, _) = recording_streaming_client(vec![SseFixture::StreamWithHeaders {
            headers: headers("req-event-secret", "4"),
            items: vec![event(
                json!({"error":{"code":code,"message":"provider rejected event-secret"}}),
            )],
        }]);
        let model = model(
            "qwen",
            http,
            Arc::new(StaticHeaderAuth::bearer_token("event-secret")),
            no_clock(),
        );
        let error = model.complete(&request()).await.unwrap_err();
        assert_eq!(error.core().kind, kind, "{code}");
        assert_eq!(error.core().provider, "qwen");
        assert_eq!(error.core().message, "provider rejected [redacted]");
        assert_metadata(&error, "req-[redacted]", 4);
        assert!(!error.to_string().contains("event-secret"));
        assert!(!format!("{error:?}").contains("event-secret"));
    }
}
