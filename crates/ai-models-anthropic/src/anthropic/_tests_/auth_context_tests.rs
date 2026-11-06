//! Dynamic authentication stays per-call and never enters public diagnostics.

use std::{collections::BTreeMap, sync::Arc};

use ai_interface::{ConversationMessage, ErrorKind, Model, ModelRequest};
use ai_models_core::test_support::{self, SseFixture};
use json_http::JsonHttpAuthMock;
use serde_json::json;
use unimock::{MockFn, Unimock, matching};

use crate::{AnthropicModel, CLAUDE_SONNET_4_6};

#[tokio::test]
async fn dynamic_auth_failures_and_echoed_secrets_stay_safe_and_call_scoped() {
    let auth = Unimock::new(JsonHttpAuthMock::apply_headers.stub(|calls| {
        calls
            .call(matching!(_))
            .answers(&|_, headers: &mut BTreeMap<String, String>| {
                headers.insert("X-Custom-Key".to_owned(), "Scheme first-secret".to_owned());
                Ok(())
            })
            .once()
            .then()
            .answers(&|_, headers: &mut BTreeMap<String, String>| {
                headers.insert("X-Custom-Key".to_owned(), "Scheme second-secret".to_owned());
                Ok(())
            })
            .once()
            .then()
            .answers(&|_, _| Err(json_http::Error::auth("undiscovered-secret")))
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
    let model = AnthropicModel::with_auth(client, CLAUDE_SONNET_4_6, Arc::new(auth))
        .with_clock(Arc::new(Unimock::new(())));
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
    assert_eq!(first.core().kind, ErrorKind::InvalidRequest);
    assert_eq!(first.core().message, "HTTP 400: rejected [redacted]");
    assert!(!format!("{first:?}").contains("first-secret"));
    assert_eq!(
        second.core().message,
        "HTTP 400: rejected first-secret [redacted]"
    );
    assert!(!format!("{second:?}").contains("second-secret"));
    assert_eq!(hook.core().kind, ErrorKind::Transient);
    assert_eq!(hook.core().message, "authentication hook failed");
    assert!(!hook.to_string().contains("undiscovered-secret"));
    assert!(!format!("{hook:?}").contains("undiscovered-secret"));
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
