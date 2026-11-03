//! Deferred errors use the actual failing phase and capture each applied auth value.

use std::{collections::BTreeMap, error::Error, sync::Arc, time::Duration};

use ai_interface::{ErrorKind, Model, SafeErrorSource, StructuredOutputSchema};
use json_http::{JsonHttpAuthMock, JsonHttpHeaders, JsonHttpResponse};
use serde_json::{Value, json};
use unimock::{MockFn, Unimock, matching};

use super::{
    XaiModel,
    deferred_support::{
        advancing_runtime, deferred_request, model, response, scripted_http_client,
        stationary_runtime,
    },
    test_support::custom_spec,
};

#[tokio::test]
async fn deferred_errors_keep_submission_final_poll_and_missing_response_facts_separate() {
    let malformed = serde_json::from_str::<Value>("{").unwrap_err();
    let cases = [
        (
            vec![with_headers(
                200,
                json!({"request_id":"../bad"}),
                "req-submit",
                "1",
            )],
            ErrorKind::InvalidOutput,
            Some("req-submit"),
            Some(1),
            false,
        ),
        (
            vec![with_headers(
                200,
                json!({"request_id":7}),
                "req-submit",
                "2",
            )],
            ErrorKind::InvalidOutput,
            Some("req-submit"),
            Some(2),
            true,
        ),
        (
            vec![
                response(200, json!({"request_id":"req_job"})),
                with_headers(202, json!({}), "req-pending", "60"),
                with_headers(
                    401,
                    json!({"error":{"message":"rejected"}}),
                    "req-final",
                    "3",
                ),
            ],
            ErrorKind::RejectedKey,
            Some("req-final"),
            Some(3),
            false,
        ),
        (
            vec![
                response(200, json!({"request_id":"req_job"})),
                with_headers(
                    200,
                    json!({"choices":[{"finish_reason":"tool_calls","message":{"tool_calls":[{"id":"call_1","function":{"name":"lookup","arguments":"{bad"}}]}}]}),
                    "req-output",
                    "4",
                ),
            ],
            ErrorKind::InvalidOutput,
            Some("req-output"),
            Some(4),
            true,
        ),
        (
            vec![
                response(200, json!({"request_id":"req_job"})),
                with_headers(202, json!({}), "req-pending", "60"),
                Err(json_http::Error::DeserializeResponse {
                    body: Value::Null,
                    source: malformed,
                }),
            ],
            ErrorKind::InvalidOutput,
            None,
            None,
            false,
        ),
    ];
    for (responses, kind, id, retry, json_source) in cases {
        let pending = responses
            .iter()
            .any(|result| matches!(result, Ok(response) if response.status == 202));
        let (http, requests) = scripted_http_client(responses);
        let runtime = if pending {
            advancing_runtime().0
        } else {
            stationary_runtime()
        };
        let model = model(http, runtime).with_clock(Arc::new(Unimock::new(())));
        let error = model
            .complete(&deferred_request(Duration::from_secs(60)))
            .await
            .unwrap_err();
        assert_eq!(error.core().kind, kind);
        assert_eq!(error.core().provider, "xai");
        assert_eq!(error.core().model_id, "grok-4");
        assert_eq!(error.core().provider_request_id.as_deref(), id);
        assert_eq!(error.core().retry_after, retry.map(Duration::from_secs));
        assert_eq!(
            error.source().is_some_and(|source| source
                .downcast_ref::<SafeErrorSource>()
                .unwrap()
                .raw_cause()
                .is::<serde_json::Error>()),
            json_source
        );
        assert_eq!(
            requests
                .lock()
                .unwrap()
                .iter()
                .filter(|request| request.method == json_http::JsonHttpMethod::Post)
                .count(),
            1
        );
    }
}

#[tokio::test]
async fn deferred_polls_capture_rotating_secrets_once_and_preserve_auth_retries() {
    let auth = Unimock::new(JsonHttpAuthMock::apply_headers.stub(|calls| {
        calls
            .call(matching!(_))
            .answers(&|_, headers: &mut BTreeMap<String, String>| {
                headers.insert("X-Auth".to_owned(), "Scheme first-secret".to_owned());
                Ok(())
            })
            .once()
            .then()
            .answers(&|_, headers: &mut BTreeMap<String, String>| {
                headers.insert("X-Auth".to_owned(), "Scheme second-secret".to_owned());
                Ok(())
            })
            .once()
            .then()
            .answers(&|_, _| Err(json_http::Error::auth("undiscovered-secret")))
            .once()
            .then()
            .answers(&|_, headers: &mut BTreeMap<String, String>| {
                headers.insert("X-Auth".to_owned(), "Scheme third-secret".to_owned());
                Ok(())
            })
            .once();
    }));
    let (http, requests) = scripted_http_client(vec![
        response(200, json!({"request_id":"req_job"})),
        with_headers(202, json!({}), "req-pending", "60"),
        with_headers(
            400,
            json!({"error":{"message":"first-secret second-secret third-secret"}}),
            "req-third-secret",
            "5",
        ),
    ]);
    let (runtime, sleeps) = advancing_runtime();
    let model = XaiModel::with_catalog_auth_and_runtime(
        http,
        custom_spec("grok-4"),
        Arc::new(auth),
        runtime,
    )
    .with_clock(Arc::new(Unimock::new(())));
    let mut request = deferred_request(Duration::from_secs(60));
    request.response_schema = Some(StructuredOutputSchema {
        name: "value".to_owned(),
        schema: json!({"type":"object"}),
    });
    let error = model.complete(&request).await.unwrap_err();
    assert_eq!(error.core().kind, ErrorKind::InvalidRequest);
    assert_eq!(
        error.core().message,
        "HTTP 400: [redacted] [redacted] [redacted]"
    );
    assert_eq!(
        error.core().provider_request_id.as_deref(),
        Some("req-[redacted]")
    );
    assert_eq!(error.core().retry_after, Some(Duration::from_secs(5)));
    for secret in [
        "first-secret",
        "second-secret",
        "third-secret",
        "undiscovered-secret",
    ] {
        assert!(!error.to_string().contains(secret));
        assert!(!format!("{error:?}").contains(secret));
    }
    let requests = requests.lock().unwrap();
    assert_eq!(requests.len(), 3);
    for (request, value) in requests.iter().zip([
        "Scheme first-secret",
        "Scheme second-secret",
        "Scheme third-secret",
    ]) {
        assert_eq!(
            request.headers.get("X-Auth").map(String::as_str),
            Some(value)
        );
    }
    assert_eq!(*sleeps.lock().unwrap(), vec![Duration::from_secs(5); 2]);
}

fn with_headers(
    status: u16,
    body: Value,
    id: &str,
    retry: &str,
) -> json_http::Result<JsonHttpResponse<Value>> {
    let mut headers = JsonHttpHeaders::new();
    headers.insert("x-request-id", id.parse().unwrap());
    headers.insert("retry-after", retry.parse().unwrap());
    Ok(JsonHttpResponse {
        status,
        headers,
        body,
    })
}
