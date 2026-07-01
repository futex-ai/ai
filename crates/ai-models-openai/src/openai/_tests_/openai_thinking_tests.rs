//! OpenAI thinking-level request mapping tests.

use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

use ai_interface::{ConversationMessage, Model, ModelRequest, ToolDefinition};
use ai_models_core::ThinkingLevel;
use json_http::{
    JsonHttpClient, JsonHttpRequest, JsonHttpResponse, JsonHttpTransportMock, StaticHeaderAuth,
    TransportBackedJsonHttpClient,
};
use serde_json::json;
use unimock::{MockFn, Unimock, matching};

use super::OpenAiModel;

type RecordedRequests = Arc<Mutex<Vec<JsonHttpRequest>>>;

#[tokio::test]
async fn builds_openai_thinking_variant_requests() {
    let (http_client, requests) = recording_http_client(JsonHttpResponse {
        status: 200,
        body: json!({
            "status": "completed",
            "output": [{
                "type": "message",
                "role": "assistant",
                "content": [{ "type": "output_text", "text": "Done" }]
            }]
        }),
    });
    let model = OpenAiModel::with_catalog_auth(
        http_client,
        "gpt-5.5-thinking-extra-high",
        "gpt-5.5",
        ThinkingLevel::ExtraHigh,
        Arc::new(StaticHeaderAuth::bearer_token("sk-openai")),
    );

    let response = model
        .complete(&simple_request())
        .await
        .expect("OpenAI thinking response should parse");

    let requests = requests
        .lock()
        .expect("requests lock should not be poisoned");
    let body = requests[0].body.as_ref().expect("body present");
    assert_eq!(requests[0].url, "https://api.openai.com/v1/responses");
    assert_eq!(body["model"], "gpt-5.5");
    assert_eq!(body["reasoning"]["effort"], "xhigh");
    assert_eq!(body["include"][0], "reasoning.encrypted_content");
    assert_eq!(body["tools"][0]["name"], "memory_read");
    assert_eq!(body["tool_choice"], "auto");
    assert!(body.get("reasoning_effort").is_none());
    assert_eq!(
        response.catalog_model_id.as_deref(),
        Some("gpt-5.5-thinking-extra-high")
    );
    assert_eq!(response.thinking_level.as_deref(), Some("extra_high"));
    assert_eq!(response.model_id, "gpt-5.5");
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
        tools: vec![ToolDefinition {
            name: "memory_read".to_owned(),
            description: "Read memory".to_owned(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": {"type": "string"}
                },
                "required": ["path"]
            }),
            activity_verb: None,
        }],
        response_schema: None,
    }
}
