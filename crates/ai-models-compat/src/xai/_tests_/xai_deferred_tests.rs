//! XAI deferred-completion lifecycle tests.

use crate::stream_test_support::{assistant_delta, content_events, observe};
use ai_interface::{ErrorKind, Model};
use json_http::JsonHttpMethod;
use serde_json::{Value, json};
use std::time::Duration;

use super::deferred_support::{
    advancing_runtime, completed_body, deferred_request, model, reqwest_failure, response,
    scripted_http_client, stationary_runtime,
};

#[tokio::test]
async fn polls_one_accepted_completion_and_emits_result_events() {
    let (http_client, requests) = scripted_http_client(vec![
        response(200, json!({ "request_id": "req_abc-123" })),
        response(202, json!({ "status": "pending" })),
        Err(json_http::Error::transport("poll disconnected")),
        Err(reqwest_failure("://invalid")),
        response(429, json!({ "error": "slow down" })),
        response(503, json!({ "error": "unavailable" })),
        response(200, completed_body()),
    ]);
    let (runtime, sleeps) = advancing_runtime();
    let model = model(http_client, runtime);
    let mut recorded = Vec::new();

    let result = observe(
        &model,
        &deferred_request(Duration::from_secs(60)),
        &mut recorded,
    )
    .await
    .expect("deferred completion should eventually succeed");

    assert_eq!(result.text(), "Done");
    assert_eq!(content_events(&recorded), vec![assistant_delta("Done")]);
    assert_eq!(recorded.len(), 3);
    assert_eq!(recorded[2]["Finish"]["finish_reason"], "Stop");
    let requests = requests.lock().expect("request lock should be available");
    assert_eq!(
        requests
            .iter()
            .filter(|request| request.method == JsonHttpMethod::Post)
            .count(),
        1
    );
    assert_eq!(requests.len(), 7);
    let body = serde_json::from_slice::<Value>(
        requests[0]
            .body
            .as_ref()
            .expect("submit body")
            .as_json_bytes()
            .expect("JSON body"),
    )
    .expect("valid JSON request body");
    assert!(body["deferred"] == true);
    assert!(body.get("stream").is_none());
    assert!(body.get("stream_options").is_none());
    assert_eq!(
        requests[0].headers.get("Authorization").map(String::as_str),
        Some("Bearer xai-key")
    );
    for request in &requests[1..] {
        assert_eq!(request.method, JsonHttpMethod::Get);
        assert!(request.url.ends_with("/req_abc-123"));
        assert_eq!(
            request.headers.get("Authorization").map(String::as_str),
            Some("Bearer xai-key")
        );
        assert!(request.timeout <= Duration::from_secs(30));
    }
    assert_eq!(
        *sleeps.lock().expect("sleep lock should be available"),
        vec![Duration::from_secs(5); 5]
    );
}

#[tokio::test]
async fn rejects_non_successful_submission_without_polling() {
    let (http_client, requests) =
        scripted_http_client(vec![response(302, json!({ "request_id": "redirected" }))]);
    let runtime = stationary_runtime();
    let model = model(http_client, runtime);

    let result = model
        .stream(&deferred_request(Duration::from_secs(60)))
        .await;

    assert!(matches!(result, Err(ref error) if error.core().kind == ErrorKind::InvalidOutput));
    let requests = requests.lock().expect("request lock should be available");
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].method, JsonHttpMethod::Post);
}

#[tokio::test]
async fn returns_terminal_retrieval_errors_without_resubmitting() {
    let (http_client, requests) = scripted_http_client(vec![
        response(200, json!({ "request_id": "req_terminal" })),
        response(400, json!({ "error": { "message": "invalid" } })),
    ]);
    let runtime = stationary_runtime();
    let model = model(http_client, runtime);

    let result = model
        .stream(&deferred_request(Duration::from_secs(60)))
        .await;

    assert!(matches!(result, Err(ref error) if error.core().kind == ErrorKind::InvalidRequest));
    let requests = requests.lock().expect("request lock should be available");
    assert_eq!(requests.len(), 2);
    assert_eq!(requests[0].method, JsonHttpMethod::Post);
    assert_eq!(requests[1].method, JsonHttpMethod::Get);
}

#[tokio::test]
async fn rejects_malformed_deferred_request_ids_before_polling() {
    let (http_client, requests) = scripted_http_client(vec![response(
        200,
        json!({ "request_id": "../../credentials" }),
    )]);
    let runtime = stationary_runtime();
    let model = model(http_client, runtime);

    let result = model
        .stream(&deferred_request(Duration::from_secs(60)))
        .await;

    assert!(matches!(result, Err(ref error) if error.core().kind == ErrorKind::InvalidOutput));
    assert_eq!(
        requests
            .lock()
            .expect("request lock should be available")
            .len(),
        1
    );
}

#[tokio::test]
async fn total_timeout_bounds_submission_poll_and_sleep() {
    let (http_client, requests) = scripted_http_client(vec![
        response(200, json!({ "request_id": "req_timeout" })),
        response(202, json!({ "status": "pending" })),
    ]);
    let (runtime, sleeps) = advancing_runtime();
    let model = model(http_client, runtime);

    let result = model
        .stream(&deferred_request(Duration::from_secs(3)))
        .await;

    assert!(matches!(result, Err(ref error) if error.core().kind == ErrorKind::Transient));
    let requests = requests.lock().expect("request lock should be available");
    assert_eq!(requests.len(), 2);
    assert_eq!(requests[0].timeout, Duration::from_secs(3));
    assert_eq!(requests[1].timeout, Duration::from_secs(3));
    assert_eq!(
        *sleeps.lock().expect("sleep lock should be available"),
        vec![Duration::from_secs(3)]
    );
}
