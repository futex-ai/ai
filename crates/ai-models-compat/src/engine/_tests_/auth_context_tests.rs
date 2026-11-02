//! Dynamic auth runs once and redacts only the credentials of that model call.

use std::{collections::BTreeMap, sync::Arc};

use ai_interface::ErrorKind;
use ai_models_core::test_support::{SseFixture, recording_streaming_client};
use json_http::JsonHttpAuthMock;
use serde_json::json;
use unimock::{MockFn, Unimock, matching};

use super::error_support::{headers, model, no_clock, request};

#[tokio::test]
async fn rotating_auth_and_unknown_hook_diagnostics_stay_call_scoped() {
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
    let (http, requests) = recording_streaming_client(vec![
        SseFixture::OpeningError(json_http::Error::HttpStatus {
            status: 400,
            headers: Box::new(headers("req-first-secret", "3")),
            body: json!({"error":{"message":"first-secret"}}),
        }),
        SseFixture::OpeningError(json_http::Error::HttpStatus {
            status: 400,
            headers: Box::new(headers("req-second-secret", "4")),
            body: json!({"error":{"message":"first-secret second-secret"}}),
        }),
    ]);
    let model = model("kimi", http, Arc::new(auth), no_clock());
    let first = model.complete(&request()).await.unwrap_err();
    let second = model.complete(&request()).await.unwrap_err();
    let hook = model.complete(&request()).await.unwrap_err();
    assert_eq!(first.core().kind, ErrorKind::InvalidRequest);
    assert_eq!(first.core().message, "HTTP 400: [redacted]");
    assert_eq!(second.core().message, "HTTP 400: first-secret [redacted]");
    assert_eq!(
        second.core().provider_request_id.as_deref(),
        Some("req-[redacted]")
    );
    assert!(!format!("{first:?}").contains("first-secret"));
    assert!(!second.to_string().contains("second-secret"));
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
