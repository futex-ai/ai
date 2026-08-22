//! Tests for xAI usage parsing.

use ai_interface::{ConversationMessage, Model, ModelRequest};
use json_http::JsonHttpResponse;
use serde_json::json;

use super::{XaiModel, test_support::recording_http_client};

#[tokio::test]
async fn missing_xai_total_tokens_falls_back_to_normalized_usage_sum() {
    let (http_client, _) = recording_http_client(JsonHttpResponse {
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

#[tokio::test]
async fn xai_usage_separates_cached_and_reasoning_tokens() {
    let (http_client, _) = recording_http_client(JsonHttpResponse {
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
                "completion_tokens": 50,
                "prompt_tokens_details": {
                    "cached_tokens": 80
                },
                "completion_tokens_details": {
                    "reasoning_tokens": 20
                }
            }
        }),
    });
    let model = XaiModel::new(http_client, "grok-4", "xai-key");

    let response = model
        .complete(&simple_request())
        .await
        .expect("xAI response should parse");

    assert_eq!(response.usage.input_tokens, 40);
    assert_eq!(response.usage.cached_input_tokens, 80);
    assert_eq!(response.usage.output_tokens, 30);
    assert_eq!(response.usage.reasoning_tokens, 20);
    assert_eq!(response.usage.total_tokens, 170);
}

fn simple_request() -> ModelRequest {
    ModelRequest {
        system_prompt: "system".to_owned(),
        messages: vec![ConversationMessage::user("hello")],
        tools: Vec::new(),
        response_schema: None,
        controls: Default::default(),
    }
}
