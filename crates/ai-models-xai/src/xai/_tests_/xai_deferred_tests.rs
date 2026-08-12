//! XAI deferred-completion lifecycle tests.

use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
    time::Duration,
};

use ai_interface::{
    ConversationMessage, Model, ModelCallControls, ModelCompletionMode, ModelError,
    ModelExecutionControls, ModelRequest,
};
use ai_models_core::ThinkingLevel;
use json_http::{
    JsonHttpClient, JsonHttpMethod, JsonHttpRequest, JsonHttpResponse, JsonHttpTransportMock,
    StaticHeaderAuth, TransportBackedJsonHttpClient,
};
use serde_json::{Value, json};
use unimock::{MockFn, Unimock, matching};

use super::{
    XaiModel,
    deferred::{DeferredRuntime, DeferredRuntimeMock},
};

type ScriptedResponse = std::result::Result<JsonHttpResponse<Value>, json_http::Error>;
type RecordedRequests = Arc<Mutex<Vec<JsonHttpRequest>>>;

#[tokio::test]
async fn polls_one_accepted_completion_through_transient_retrieval_states() {
    let (http_client, requests) = scripted_http_client(vec![
        response(200, json!({ "request_id": "req_abc-123" })),
        response(202, json!({ "status": "pending" })),
        Err(json_http::Error::transport("poll disconnected")),
        response(429, json!({ "error": "slow down" })),
        response(503, json!({ "error": "unavailable" })),
        response(200, completed_body()),
    ]);
    let (runtime, sleeps) = advancing_runtime();
    let model = model(http_client, runtime);

    let result = model
        .complete(&deferred_request(Duration::from_secs(60)))
        .await
        .expect("deferred completion should eventually succeed");

    assert_eq!(result.assistant_message, "Done");
    let requests = requests.lock().expect("request lock should be available");
    assert_eq!(
        requests
            .iter()
            .filter(|request| request.method == JsonHttpMethod::Post)
            .count(),
        1
    );
    assert_eq!(requests.len(), 6);
    assert!(requests[0].body.as_ref().expect("submit body")["deferred"] == true);
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
        vec![Duration::from_secs(5); 4]
    );
}

#[tokio::test]
async fn rejects_non_successful_submission_without_polling() {
    let (http_client, requests) =
        scripted_http_client(vec![response(302, json!({ "request_id": "redirected" }))]);
    let runtime = stationary_runtime();
    let model = model(http_client, runtime);

    let error = model
        .complete(&deferred_request(Duration::from_secs(60)))
        .await
        .expect_err("redirected submission should fail");

    assert!(matches!(error, ModelError::Provider { .. }));
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

    let error = model
        .complete(&deferred_request(Duration::from_secs(60)))
        .await
        .expect_err("terminal retrieval should fail");

    assert!(matches!(error, ModelError::Provider { .. }));
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

    let error = model
        .complete(&deferred_request(Duration::from_secs(60)))
        .await
        .expect_err("unsafe request id should fail");

    assert!(matches!(error, ModelError::Provider { .. }));
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

    let error = model
        .complete(&deferred_request(Duration::from_secs(3)))
        .await
        .expect_err("total timeout should stop polling");

    assert!(matches!(error, ModelError::TransientProvider { .. }));
    let requests = requests.lock().expect("request lock should be available");
    assert_eq!(requests.len(), 2);
    assert_eq!(requests[0].timeout, Duration::from_secs(3));
    assert_eq!(requests[1].timeout, Duration::from_secs(3));
    assert_eq!(
        *sleeps.lock().expect("sleep lock should be available"),
        vec![Duration::from_secs(3)]
    );
}

fn model(http_client: Arc<dyn JsonHttpClient>, runtime: Arc<dyn DeferredRuntime>) -> XaiModel {
    XaiModel::with_catalog_auth_and_runtime(
        http_client,
        "grok-4",
        "grok-4",
        ThinkingLevel::Disabled,
        Arc::new(StaticHeaderAuth::bearer_token("xai-key")),
        runtime,
    )
}

fn deferred_request(total_timeout: Duration) -> ModelRequest {
    ModelRequest {
        system_prompt: String::new(),
        messages: vec![ConversationMessage::user("hello")],
        tools: Vec::new(),
        response_schema: None,
        controls: ModelCallControls {
            execution: ModelExecutionControls {
                total_timeout: Some(total_timeout),
                completion_mode: ModelCompletionMode::PreferDeferred,
            },
            ..Default::default()
        },
    }
}

fn scripted_http_client(
    responses: Vec<ScriptedResponse>,
) -> (Arc<dyn JsonHttpClient>, RecordedRequests) {
    let requests = Arc::new(Mutex::new(Vec::new()));
    let responses = Arc::new(Mutex::new(VecDeque::from(responses)));
    let transport = Arc::new(Unimock::new(
        JsonHttpTransportMock::execute
            .each_call(matching!(_))
            .answers_arc({
                let requests = requests.clone();
                let responses = responses.clone();
                Arc::new(move |_, request: &JsonHttpRequest| {
                    requests
                        .lock()
                        .expect("request lock should be available")
                        .push(request.clone());
                    responses
                        .lock()
                        .expect("response lock should be available")
                        .pop_front()
                        .expect("unexpected transport call")
                })
            }),
    ));
    (
        Arc::new(TransportBackedJsonHttpClient::new(transport)),
        requests,
    )
}

fn advancing_runtime() -> (Arc<dyn DeferredRuntime>, Arc<Mutex<Vec<Duration>>>) {
    let started_at = tokio::time::Instant::now();
    let elapsed = Arc::new(Mutex::new(Duration::ZERO));
    let sleeps = Arc::new(Mutex::new(Vec::new()));
    let runtime = Arc::new(Unimock::new((
        DeferredRuntimeMock::now
            .each_call(matching!())
            .answers_arc({
                let elapsed = elapsed.clone();
                Arc::new(move |_| {
                    started_at + *elapsed.lock().expect("elapsed lock should be available")
                })
            }),
        DeferredRuntimeMock::sleep
            .each_call(matching!(_))
            .answers_arc({
                let elapsed = elapsed.clone();
                let sleeps = sleeps.clone();
                Arc::new(move |_, duration: Duration| {
                    sleeps
                        .lock()
                        .expect("sleep lock should be available")
                        .push(duration);
                    let mut elapsed = elapsed.lock().expect("elapsed lock should be available");
                    *elapsed += duration;
                })
            }),
    ))) as Arc<dyn DeferredRuntime>;
    (runtime, sleeps)
}

fn stationary_runtime() -> Arc<dyn DeferredRuntime> {
    let now = tokio::time::Instant::now();
    Arc::new(Unimock::new(
        DeferredRuntimeMock::now.each_call(matching!()).returns(now),
    ))
}

fn response(status: u16, body: Value) -> ScriptedResponse {
    Ok(JsonHttpResponse { status, body })
}

fn completed_body() -> Value {
    json!({
        "choices": [{
            "finish_reason": "stop",
            "message": { "content": "Done", "tool_calls": [] }
        }]
    })
}
