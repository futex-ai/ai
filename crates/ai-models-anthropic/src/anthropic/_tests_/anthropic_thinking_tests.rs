//! Anthropic thinking-level request mapping tests.

use std::{
    collections::{BTreeMap, VecDeque},
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

use super::AnthropicModel;

type RecordedRequests = Arc<Mutex<Vec<JsonHttpRequest>>>;

#[tokio::test]
async fn builds_anthropic_thinking_variant_requests_and_ignores_hidden_blocks() {
    let (http_client, requests) = recording_http_client(JsonHttpResponse {
        status: 200,
        body: json!({
            "stop_reason": "end_turn",
            "content": [
                { "type": "thinking", "thinking": "hidden provider trace" },
                { "type": "text", "text": "Done" }
            ],
            "usage": {
                "input_tokens": 12,
                "output_tokens": 6
            }
        }),
    });
    let model = AnthropicModel::with_catalog_auth(
        http_client,
        "claude-opus-4-7-thinking-max",
        "claude-opus-4-7",
        ThinkingLevel::Max,
        Arc::new(StaticHeaderAuth::new(BTreeMap::from([(
            "x-api-key".to_owned(),
            "anthropic-key".to_owned(),
        )]))),
    );

    let response = model
        .complete(&simple_request())
        .await
        .expect("Anthropic thinking response should parse");

    let requests = requests
        .lock()
        .expect("requests lock should not be poisoned");
    let body = requests[0].body.as_ref().expect("body present");
    assert_eq!(body["model"], "claude-opus-4-7");
    assert_eq!(body["thinking"]["type"], "adaptive");
    assert_eq!(body["thinking"]["display"], "omitted");
    assert_eq!(body["output_config"]["effort"], "max");
    assert_eq!(
        response.catalog_model_id.as_deref(),
        Some("claude-opus-4-7-thinking-max")
    );
    assert_eq!(response.thinking_level.as_deref(), Some("max"));
    assert_eq!(response.model_id, "claude-opus-4-7");
    assert_eq!(response.assistant_message, "Done");
    assert!(!response.assistant_message.contains("hidden provider trace"));
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
