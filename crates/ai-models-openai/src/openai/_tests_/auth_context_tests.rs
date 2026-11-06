//! Chat auth hooks apply once and keep rotating credentials isolated per call.

use std::{collections::BTreeMap, sync::Arc};

use ai_interface::{ConversationMessage, ErrorKind, Model, ModelRequest};
use ai_models_core::test_support::{self, SseFixture};
use json_http::JsonHttpAuthMock;
use serde_json::json;
use unimock::{MockFn, Unimock, matching};

use crate::OpenAiModel;

use super::{error_context_support::unused_clock, stream_support::custom_spec};

#[tokio::test]
async fn rotating_chat_auth_and_unknown_hook_errors_keep_diagnostics_safe() {
    let auth = Unimock::new(JsonHttpAuthMock::apply_headers.stub(|calls| {
        calls
            .call(matching!(_))
            .answers(&|_, headers: &mut BTreeMap<String, String>| {
                headers.insert("X-Custom-Key".into(), "Scheme first-secret".into());
                Ok(())
            })
            .once()
            .then()
            .answers(&|_, headers: &mut BTreeMap<String, String>| {
                headers.insert("X-Custom-Key".into(), "Scheme second-secret".into());
                Ok(())
            })
            .once()
            .then()
            .answers(&|_, _| Err(json_http::Error::auth("unknown-secret")))
            .once();
    }));
    let (client, requests) = test_support::recording_streaming_client(vec![
        SseFixture::OpeningError(json_http::Error::HttpStatus {
            status: 400,
            headers: Default::default(),
            body: json!({"error":{"message":"rejected first-secret"}}),
        }),
        SseFixture::OpeningError(json_http::Error::HttpStatus {
            status: 400,
            headers: Default::default(),
            body: json!({"error":{"message":"rejected first-secret second-secret"}}),
        }),
    ]);
    let model = OpenAiModel::with_auth(client, custom_spec("gpt-current"), Arc::new(auth))
        .with_clock(unused_clock());
    let request = ModelRequest {
        system_prompt: String::new(),
        messages: vec![ConversationMessage::user("hello")],
        tools: Vec::new(),
        response_schema: None,
        controls: Default::default(),
    };
    let first = model.complete(&request).await.unwrap_err();
    let second = model.complete(&request).await.unwrap_err();
    let hook = model.complete(&request).await.unwrap_err();
    assert_eq!(first.core().message, "HTTP 400: rejected [redacted]");
    assert!(!format!("{first:?}").contains("first-secret"));
    assert_eq!(
        second.core().message,
        "HTTP 400: rejected first-secret [redacted]"
    );
    assert!(!format!("{second:?}").contains("second-secret"));
    assert_eq!(hook.core().kind, ErrorKind::Transient);
    assert_eq!(hook.core().message, "authentication hook failed");
    assert!(!hook.to_string().contains("unknown-secret"));
    assert!(!format!("{hook:?}").contains("unknown-secret"));
    let requests = requests.lock().unwrap();
    assert_eq!(requests.len(), 2);
    assert_eq!(
        requests[0].headers.get("X-Custom-Key").map(String::as_str),
        Some("Scheme first-secret")
    );
    assert_eq!(
        requests[1].headers.get("X-Custom-Key").map(String::as_str),
        Some("Scheme second-secret")
    );
}
