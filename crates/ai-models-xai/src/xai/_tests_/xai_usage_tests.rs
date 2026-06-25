//! Tests for xAI usage parsing.

use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

use ai_interface::{ConversationMessage, Model, ModelRequest};
use json_http::{
    JsonHttpClient, JsonHttpRequest, JsonHttpResponse, JsonHttpTransportMock,
    TransportBackedJsonHttpClient,
};
use serde_json::json;
use unimock::{MockFn, Unimock, matching};

use super::XaiModel;

#[tokio::test]
async fn missing_xai_total_tokens_falls_back_to_normalized_usage_sum() {
    let http_client = single_response_http_client(JsonHttpResponse {
        status: 200,
        body: json!({
            "choices": [{
                "finish_reason": "stop",
                "message": {
                    "content": "Done",
                    "tool_calls": []
                }
            }],
            "usage": {
                "prompt_tokens": 120,
                "completion_tokens": 32
            }
        }),
    });
    let model = XaiModel::new(http_client, "grok-4", "xai-key");

    let response = model
        .complete(&simple_request())
        .await
        .expect("xAI response should parse");

    assert_eq!(response.usage.input_tokens, 120);
    assert_eq!(response.usage.output_tokens, 32);
    assert_eq!(response.usage.total_tokens, 152);
}

fn single_response_http_client(
    response: JsonHttpResponse<serde_json::Value>,
) -> Arc<dyn JsonHttpClient> {
    let responses = Arc::new(Mutex::new(VecDeque::from([response])));
    let transport = Arc::new(Unimock::new(
        JsonHttpTransportMock::execute
            .each_call(matching!(_))
            .answers_arc({
                let responses = responses.clone();
                Arc::new(move |_, _request: &JsonHttpRequest| {
                    Ok(responses
                        .lock()
                        .expect("responses lock should not be poisoned")
                        .pop_front()
                        .expect("unexpected transport call"))
                })
            }),
    ));

    Arc::new(TransportBackedJsonHttpClient::new(transport))
}

fn simple_request() -> ModelRequest {
    ModelRequest {
        system_prompt: "system".to_owned(),
        messages: vec![ConversationMessage::user("hello")],
        tools: Vec::new(),
        response_schema: None,
    }
}
