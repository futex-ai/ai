//! Shared in-memory deferred transport and injected lifecycle runtime.

use super::test_support::custom_spec;
use super::{
    XaiModel,
    deferred::{DeferredRuntime, DeferredRuntimeMock},
};
use ai_interface::{
    ConversationMessage, ModelCallControls, ModelCompletionMode, ModelExecutionControls,
    ModelRequest,
};
use json_http::{
    JsonHttpClient, JsonHttpRequest, JsonHttpResponse, JsonHttpTransportMock, StaticHeaderAuth,
    TransportBackedJsonHttpClient,
};
use reqwest::IntoUrl;
use serde_json::{Value, json};
use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
    time::Duration,
};
use unimock::{MockFn, Unimock, matching};

type ScriptedResponse = std::result::Result<JsonHttpResponse<Value>, json_http::Error>;
type RecordedRequests = Arc<Mutex<Vec<JsonHttpRequest>>>;

pub(super) fn model(
    http_client: Arc<dyn JsonHttpClient>,
    runtime: Arc<dyn DeferredRuntime>,
) -> XaiModel {
    XaiModel::with_catalog_auth_and_runtime(
        http_client,
        custom_spec("grok-4"),
        Arc::new(StaticHeaderAuth::bearer_token("xai-key")),
        runtime,
    )
}

pub(super) fn deferred_request(total_timeout: Duration) -> ModelRequest {
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

pub(super) fn scripted_http_client(
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

pub(super) fn advancing_runtime() -> (Arc<dyn DeferredRuntime>, Arc<Mutex<Vec<Duration>>>) {
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

pub(super) fn stationary_runtime() -> Arc<dyn DeferredRuntime> {
    let now = tokio::time::Instant::now();
    Arc::new(Unimock::new(
        DeferredRuntimeMock::now.each_call(matching!()).returns(now),
    ))
}

pub(super) fn response(status: u16, body: Value) -> ScriptedResponse {
    Ok(JsonHttpResponse {
        headers: Default::default(),
        status,
        body,
    })
}

pub(super) fn completed_body() -> Value {
    json!({
        "choices": [{
            "finish_reason": "stop",
            "message": { "content": "Done", "tool_calls": [] }
        }]
    })
}

pub(super) fn reqwest_failure(url: impl IntoUrl) -> json_http::Error {
    json_http::Error::ReqwestTransport {
        source: url.into_url().unwrap_err(),
    }
}
