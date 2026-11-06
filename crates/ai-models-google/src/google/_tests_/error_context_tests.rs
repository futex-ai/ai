//! Google stream errors retain opening facts without replacing specific read headers.

use std::{error::Error, time::Duration};

use ai_interface::{
    ConversationMessage, ErrorKind, Model, ModelRequest, SafeErrorSource, StructuredOutputSchema,
};
use ai_models_core::test_support::{self, SseFixture};
use serde_json::json;

use crate::GoogleModel;

use super::{
    error_context_support::{headers, unused_clock},
    google_tests::stream_support::custom_spec,
};

#[tokio::test]
async fn embedded_stream_errors_keep_the_successful_response_headers() {
    for (code, status, kind) in [
        (401, "UNAUTHENTICATED", ErrorKind::RejectedKey),
        (429, "RESOURCE_EXHAUSTED", ErrorKind::RateLimited),
        (503, "UNAVAILABLE", ErrorKind::Transient),
        (
            400,
            "context_length_exceeded",
            ErrorKind::ContextLimitExceeded,
        ),
    ] {
        let (client, _) = test_support::recording_streaming_client(vec![
            SseFixture::StreamWithHeaders {
                headers: headers("req-google-secret", "7"),
                items: vec![test_support::event(
                    json!({"error":{"code":code,"status":status,"message":"rejected google-secret"}}),
                )],
            },
        ]);
        let model = GoogleModel::new(client, custom_spec("gemini-current"), "google-secret")
            .with_clock(unused_clock());
        let error = model.complete(&request()).await.unwrap_err();
        assert_eq!(error.core().kind, kind, "{code}");
        assert_eq!(error.core().provider, "google");
        assert_eq!(error.core().model_id, "gemini-current");
        assert_eq!(
            error.core().provider_request_id.as_deref(),
            Some("req-[redacted]"),
            "{code}"
        );
        assert_eq!(
            error.core().retry_after,
            Some(Duration::from_secs(7)),
            "{code}"
        );
        assert!(!error.to_string().contains("google-secret"));
        assert!(!format!("{error:?}").contains("google-secret"));
    }
}

#[tokio::test]
async fn late_stream_and_output_errors_use_the_relevant_response_metadata() {
    for (case, kind, fixture, schema, id, retry, json_source) in [
        (
            "status",
            ErrorKind::RejectedKey,
            SseFixture::OpeningError(json_http::Error::HttpStatus {
                status: 401,
                headers: Box::new(headers("req-google-secret", "7")),
                body: json!({"error":{"message":"rejected google-secret"}}),
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
                headers: headers("req-google-secret", "7"),
                items: vec![
                    progress(),
                    Err(json_http::Error::transport("reset google-secret")),
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
                headers: headers("req-google-secret", "7"),
                items: vec![
                    progress(),
                    Err(json_http::Error::transport("reset google-secret")
                        .with_response_headers(headers("req-specific", "3"))),
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
                headers: headers("req-google-secret", "7"),
                items: vec![
                    progress(),
                    test_support::event(
                        json!({"error":{"code":401,"status":"UNAUTHENTICATED","message":"rejected google-secret"}}),
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
                headers: headers("req-google-secret", "7"),
                items: vec![test_support::event(
                    json!({"candidates":[{"finishReason":"STOP","content":{"parts":[{"text":"not JSON"}]}}]}),
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
                headers: headers("req-google-secret", "7"),
                items: vec![test_support::event(
                    json!({"candidates":[{"finishReason":"STOP","content":{"parts":[{"text":"{}"}]}}]}),
                )],
            },
            Some(json!({"type":"invalid-type"})),
            "req-[redacted]",
            7,
            false,
        ),
    ] {
        let (client, _) = test_support::recording_streaming_client(vec![fixture]);
        let model = GoogleModel::new(client, custom_spec("gemini-current"), "google-secret")
            .with_clock(unused_clock());
        let mut request = request();
        request.response_schema = schema.map(|schema| StructuredOutputSchema {
            name: "google-secret".into(),
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
        assert!(!error.to_string().contains("google-secret"), "{case}");
        assert!(!format!("{error:?}").contains("google-secret"), "{case}");
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
    test_support::event(json!({"usageMetadata":{"promptTokenCount":1}}))
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
