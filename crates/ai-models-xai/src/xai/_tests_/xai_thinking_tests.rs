//! xAI thinking-level request mapping tests.

use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

use ai_interface::{ConversationMessage, Model, ModelRequest};
use ai_models_core::ThinkingLevel;
use json_http::{
    JsonHttpClient, JsonHttpRequest, JsonHttpResponse, JsonHttpTransportMock, StaticHeaderAuth,
    TransportBackedJsonHttpClient,
};
use serde_json::json;
use unimock::{MockFn, Unimock, matching};

use super::XaiModel;

type RecordedRequests = Arc<Mutex<Vec<JsonHttpRequest>>>;

#[tokio::test]
async fn builds_xai_thinking_variant_requests() {
    let (http_client, requests) = recording_http_client(JsonHttpResponse {
        status: 200,
        body: json!({
            "choices": [{
                "finish_reason": "stop",
                "message": {
                    "content": "Done",
                    "tool_calls": []
                }
            }]
        }),
    });
    let model = XaiModel::with_catalog_auth(
        http_client,
        "grok-4.20-thinking-high",
        "grok-4.20",
        ThinkingLevel::High,
        Arc::new(StaticHeaderAuth::bearer_token("xai-key")),
    );

    let response = model
        .complete(&simple_request())
        .await
        .expect("xAI thinking response should parse");

    let requests = requests
        .lock()
        .expect("requests lock should not be poisoned");
    let body = requests[0].body.as_ref().expect("body present");
    assert_eq!(body["model"], "grok-4.20");
    assert_eq!(body["reasoning_effort"], "high");
    assert_eq!(
        response.catalog_model_id.as_deref(),
        Some("grok-4.20-thinking-high")
    );
    assert_eq!(response.thinking_level.as_deref(), Some("high"));
    assert_eq!(response.model_id, "grok-4.20");
}

fn recording_http_client(
    response: JsonHttpResponse<serde_json::Value>,
) -> (Arc<dyn JsonHttpClient>, RecordedRequests) {
    let requests = Arc::new(Mutex::new(Vec::new()));
    let responses = Arc::new(Mutex::new(VecDeque::from([response])));
    let transport = Arc::new(Unimock::new(
        JsonHttpTransportMock::execute
            .each_call(matching!(_))
            .answers_arc({
                let requests = requests.clone();
                let responses = responses.clone();
                Arc::new(move |_, request: &JsonHttpRequest| {
                    requests
                        .lock()
                        .expect("requests lock should not be poisoned")
                        .push(request.clone());
                    Ok(responses
                        .lock()
                        .expect("responses lock should not be poisoned")
                        .pop_front()
                        .expect("unexpected transport call"))
                })
            }),
    ));

    (
        Arc::new(TransportBackedJsonHttpClient::new(transport)),
        requests,
    )
}

fn simple_request() -> ModelRequest {
    ModelRequest {
        system_prompt: "system".to_owned(),
        messages: vec![ConversationMessage::user("hello")],
        tools: Vec::new(),
        response_schema: None,
    }
}
