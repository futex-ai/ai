//! Public Anthropic error boundaries retain metadata and sanitize diagnostics.

use std::{
    error::Error,
    sync::Arc,
    time::{Duration, SystemTime},
};

use ai_interface::{
    ConversationMessage, ErrorKind, Model, ModelRequest, SafeErrorSource, StructuredOutputSchema,
};
use ai_models_core::{
    ClockMock,
    test_support::{self, SseFixture},
};
use json_http::JsonHttpHeaders;
use serde_json::json;
use unimock::{MockFn, Unimock, matching};

use crate::{AnthropicModel, CLAUDE_SONNET_4_6};

use super::stream_support::{event, events_from_buffered_body};

#[tokio::test]
async fn status_failures_keep_response_facts_and_hide_transmitted_credentials() {
    for (status, kind) in [
        (401, ErrorKind::RejectedKey),
        (429, ErrorKind::RateLimited),
        (400, ErrorKind::ContextLimitExceeded),
    ] {
        let headers = headers("req-status", "Wed, 21 Oct 2015 07:28:10 GMT");
        let (client, requests) = test_support::recording_streaming_client(vec![
            SseFixture::OpeningError(json_http::Error::HttpStatus {
                status,
                headers: Box::new(headers),
                body: json!({"error":{"code":"context_length_exceeded","message":"rejected secret-status"}}),
            }),
        ]);
        let clock =
            Arc::new(Unimock::new(ClockMock::now.each_call(matching!()).returns(
                SystemTime::UNIX_EPOCH + Duration::from_secs(1_445_412_480),
            )));
        let model =
            AnthropicModel::new(client, CLAUDE_SONNET_4_6, "secret-status").with_clock(clock);
        let error = model.complete(&request()).await.unwrap_err();
        let core = error.core();
        assert_eq!(core.kind, kind, "HTTP {status}");
        assert_eq!(core.provider, "anthropic");
        assert_eq!(core.model_id, "claude-sonnet-4-6");
        assert_eq!(core.provider_request_id.as_deref(), Some("req-status"));
        assert_eq!(core.retry_after, Some(Duration::from_secs(10)));
        assert!(!error.to_string().contains("secret-status"));
        assert!(!format!("{error:?}").contains("secret-status"));
        let requests = requests.lock().unwrap();
        assert_eq!(requests.len(), 1);
        assert_eq!(
            requests[0].headers.get("x-api-key").map(String::as_str),
            Some("secret-status")
        );
    }
}

#[tokio::test]
async fn late_stream_and_output_errors_keep_opening_metadata_and_safe_sources() {
    for (case, kind, schema, items, json_source) in [
        (
            "read",
            ErrorKind::Interrupted,
            None,
            vec![
                start(),
                Err(json_http::Error::transport("reset secret-late")),
            ],
            false,
        ),
        (
            "provider",
            ErrorKind::Interrupted,
            None,
            vec![
                start(),
                event(
                    "error",
                    json!({"type":"error","error":{"type":"authentication_error","message":"rejected secret-late"}}),
                ),
            ],
            false,
        ),
        (
            "output",
            ErrorKind::InvalidOutput,
            Some(json!({"type":"object"})),
            events_from_buffered_body(
                json!({"stop_reason":"end_turn","content":[{"type":"text","text":"not JSON"}]}),
            ),
            true,
        ),
        (
            "caller schema",
            ErrorKind::InvalidRequest,
            Some(json!({"type":"invalid-type"})),
            events_from_buffered_body(
                json!({"stop_reason":"end_turn","content":[{"type":"text","text":"{}"}]}),
            ),
            false,
        ),
    ] {
        let (client, _) =
            test_support::recording_streaming_client(vec![SseFixture::StreamWithHeaders {
                headers: headers("req-secret-late", "7"),
                items,
            }]);
        let model = AnthropicModel::new(client, CLAUDE_SONNET_4_6, "secret-late")
            .with_clock(Arc::new(Unimock::new(())));
        let mut request = request();
        request.response_schema = schema.map(|schema| StructuredOutputSchema {
            name: "secret-late".to_owned(),
            schema,
        });
        let error = model.complete(&request).await.unwrap_err();
        let core = error.core();
        assert_eq!(core.kind, kind, "{case}");
        assert_eq!(
            core.provider_request_id.as_deref(),
            Some("req-[redacted]"),
            "{case}"
        );
        assert_eq!(core.retry_after, Some(Duration::from_secs(7)), "{case}");
        assert!(!error.to_string().contains("secret-late"), "{case}");
        assert!(!format!("{error:?}").contains("secret-late"), "{case}");
        assert_eq!(
            error
                .source()
                .and_then(|source| source
                    .downcast_ref::<SafeErrorSource>()
                    .unwrap()
                    .raw_cause()
                    .downcast_ref::<serde_json::Error>())
                .is_some(),
            json_source,
            "{case}"
        );
    }
}

fn headers(request_id: &str, retry_after: &str) -> JsonHttpHeaders {
    JsonHttpHeaders::from_iter([
        ("request-id".parse().unwrap(), request_id.parse().unwrap()),
        ("x-request-id".parse().unwrap(), "alias-id".parse().unwrap()),
        ("retry-after".parse().unwrap(), retry_after.parse().unwrap()),
    ])
}

#[tokio::test]
async fn read_failure_headers_override_the_opening_response_metadata() {
    let source = json_http::Error::transport("reset")
        .with_response_headers(headers("req-read-failure", "3"));
    let (client, _) =
        test_support::recording_streaming_client(vec![SseFixture::StreamWithHeaders {
            headers: headers("req-opening", "7"),
            items: vec![start(), Err(source)],
        }]);
    let model = AnthropicModel::new(client, CLAUDE_SONNET_4_6, "key")
        .with_clock(Arc::new(Unimock::new(())));
    let error = model.complete(&request()).await.unwrap_err();
    assert_eq!(error.core().kind, ErrorKind::Interrupted);
    assert_eq!(
        error.core().provider_request_id.as_deref(),
        Some("req-read-failure")
    );
    assert_eq!(error.core().retry_after, Some(Duration::from_secs(3)));
}

fn start() -> test_support::StreamItem {
    event(
        "message_start",
        json!({"type":"message_start","message":{}}),
    )
}

fn request() -> ModelRequest {
    ModelRequest {
        system_prompt: String::new(),
        messages: vec![ConversationMessage::user("hello")],
        tools: Vec::new(),
        response_schema: None,
        controls: Default::default(),
    }
}
