//! Chat failures retain the relevant response facts and safe original sources.

use std::{error::Error, time::Duration};

use ai_interface::{
    ConversationMessage, ErrorKind, Model, ModelRequest, SafeErrorSource, StructuredOutputSchema,
};
use ai_models_core::test_support::{self, SseFixture};
use serde_json::json;

use crate::OpenAiModel;

use super::{
    error_context_support::{headers, unused_clock},
    stream_support::{custom_spec, event, terminal_event},
};

#[tokio::test]
async fn chat_status_and_late_failures_keep_safe_response_metadata() {
    let mut source_headers = headers("req-specific", "3");
    source_headers.remove("request-id");
    for (case, kind, fixture, schema, id, retry, json_source) in [
        (
            "status",
            ErrorKind::RejectedKey,
            SseFixture::OpeningError(json_http::Error::HttpStatus {
                status: 401,
                headers: Box::new(headers("req-chat-secret", "7")),
                body: json!({"error":{"message":"rejected chat-secret"}}),
            }),
            None,
            "req-[redacted]",
            7,
            false,
        ),
        (
            "read",
            ErrorKind::Interrupted,
            SseFixture::StreamWithHeaders {
                headers: headers("req-chat-secret", "7"),
                items: vec![
                    progress(),
                    Err(json_http::Error::transport("reset chat-secret")),
                ],
            },
            None,
            "req-[redacted]",
            7,
            false,
        ),
        (
            "specific read",
            ErrorKind::Interrupted,
            SseFixture::StreamWithHeaders {
                headers: headers("req-chat-secret", "7"),
                items: vec![
                    progress(),
                    Err(json_http::Error::transport("reset chat-secret")
                        .with_response_headers(source_headers)),
                ],
            },
            None,
            "req-specific",
            3,
            false,
        ),
        (
            "provider",
            ErrorKind::Interrupted,
            SseFixture::StreamWithHeaders {
                headers: headers("req-chat-secret", "7"),
                items: vec![
                    progress(),
                    event(
                        "error",
                        json!({"type":"error","code":"invalid_api_key","message":"rejected chat-secret","param":null}),
                    ),
                ],
            },
            None,
            "req-[redacted]",
            7,
            false,
        ),
        (
            "output",
            ErrorKind::InvalidOutput,
            SseFixture::StreamWithHeaders {
                headers: headers("req-chat-secret", "7"),
                items: vec![terminal_event(
                    "response.completed",
                    json!({"status":"completed","output":[{"type":"message","content":[{"type":"output_text","text":"not JSON"}]}]}),
                )],
            },
            Some(json!({"type":"object"})),
            "req-[redacted]",
            7,
            true,
        ),
        (
            "caller schema",
            ErrorKind::InvalidRequest,
            SseFixture::StreamWithHeaders {
                headers: headers("req-chat-secret", "7"),
                items: vec![terminal_event(
                    "response.completed",
                    json!({"status":"completed","output":[{"type":"message","content":[{"type":"output_text","text":"{}"}]}]}),
                )],
            },
            Some(json!({"type":"invalid-type"})),
            "req-[redacted]",
            7,
            false,
        ),
    ] {
        let (client, _) = test_support::recording_streaming_client(vec![fixture]);
        let model = OpenAiModel::new(client, custom_spec("gpt-current"), "chat-secret")
            .with_clock(unused_clock());
        let mut request = ModelRequest {
            system_prompt: String::new(),
            messages: vec![ConversationMessage::user("hello")],
            tools: Vec::new(),
            response_schema: None,
            controls: Default::default(),
        };
        request.response_schema = schema.map(|schema| StructuredOutputSchema {
            name: "chat-secret".to_owned(),
            schema,
        });
        let error = model.complete(&request).await.unwrap_err();
        assert_eq!(error.core().kind, kind, "{case}");
        assert_eq!(
            error.core().provider_request_id.as_deref(),
            Some(id),
            "{case}"
        );
        assert_eq!(
            error.core().retry_after,
            Some(Duration::from_secs(retry)),
            "{case}"
        );
        assert!(!error.to_string().contains("chat-secret"), "{case}");
        assert!(!format!("{error:?}").contains("chat-secret"), "{case}");
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

fn progress() -> test_support::StreamItem {
    event(
        "response.created",
        json!({"type":"response.created","response":{"status":"in_progress"}}),
    )
}
