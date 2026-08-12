//! MiniMax text request and response tests.

use std::{collections::BTreeMap, sync::Arc};

use ai_interface::{
    ConversationMessage, ConversationRole, FinishReason, Model, ModelRequest, ModelUsage,
};
use ai_models_core::ThinkingLevel;
use json_http::{JsonHttpResponse, StaticHeaderAuth};
use serde_json::json;

use super::{MiniMaxModel, support::recording_http_client};

#[tokio::test]
async fn builds_text_request_and_parses_stopped_response() {
    let (http_client, requests) = recording_http_client([JsonHttpResponse {
        status: 200,
        body: json!({
            "choices": [{
                "finish_reason": "stop",
                "message": {"content": "Done"}
            }],
            "usage": {
                "prompt_tokens": 12,
                "completion_tokens": 3,
                "total_tokens": 15
            }
        }),
    }]);
    let model = MiniMaxModel::with_catalog_auth(
        http_client,
        "MiniMax-M3-routing",
        "MiniMax-M3",
        ThinkingLevel::Medium,
        Arc::new(StaticHeaderAuth::bearer_token("minimax-key")),
    );

    let response = model
        .complete(&ModelRequest {
            system_prompt: "You are concise.".to_owned(),
            messages: vec![
                ConversationMessage {
                    role: ConversationRole::User,
                    content: "Hello".to_owned(),
                    content_parts: Vec::new(),
                    name: Some("caller".to_owned()),
                    tool_call_id: None,
                    tool_calls: Vec::new(),
                    provider_context: Vec::new(),
                },
                ConversationMessage::assistant("Working", Vec::new()),
            ],
            tools: Vec::new(),
            response_schema: None,
        })
        .await
        .expect("MiniMax response should parse");

    let requests = requests
        .lock()
        .expect("requests lock should not be poisoned");
    assert_eq!(requests.len(), 1);
    let request = &requests[0];
    assert_eq!(request.url, "https://api.minimax.io/v1/chat/completions");
    assert_eq!(
        request.headers.get("Authorization"),
        Some(&"Bearer minimax-key".to_owned())
    );
    assert_eq!(
        request
            .body
            .as_ref()
            .expect("body should be present")
            .as_json(),
        Some(&json!({
            "model": "MiniMax-M3",
            "messages": [
                {"role": "system", "content": "You are concise."},
                {"role": "user", "content": "Hello", "name": "caller"},
                {"role": "assistant", "content": "Working"}
            ],
            "stream": false,
            "reasoning_split": true,
            "thinking": {"type": "adaptive"}
        }))
    );

    assert_eq!(response.provider, "minimax");
    assert_eq!(response.model_id, "MiniMax-M3");
    assert_eq!(
        response.catalog_model_id,
        Some("MiniMax-M3-routing".to_owned())
    );
    assert_eq!(response.thinking_level, Some("medium".to_owned()));
    assert_eq!(response.assistant_message, "Done");
    assert_eq!(response.finish_reason, FinishReason::Stop);
    assert!(response.tool_calls.is_empty());
    assert!(response.provider_context.is_empty());
    assert_eq!(
        response.usage,
        ModelUsage {
            input_tokens: 12,
            output_tokens: 3,
            cached_input_tokens: 0,
            cache_write_input_tokens: 0,
            reasoning_tokens: 0,
            total_tokens: 15,
            estimated_cost_microusd: 0,
            cost_lines: Vec::new(),
        }
    );
}

#[tokio::test]
async fn constructors_apply_bearer_and_injected_auth() {
    let (bearer_client, bearer_requests) = recording_http_client([stopped_response()]);
    MiniMaxModel::new(bearer_client, "MiniMax-M3", "direct-key")
        .complete(&simple_request())
        .await
        .expect("direct auth request should succeed");
    assert_eq!(
        bearer_requests
            .lock()
            .expect("requests lock should not be poisoned")[0]
            .headers
            .get("Authorization"),
        Some(&"Bearer direct-key".to_owned())
    );

    let (injected_client, injected_requests) = recording_http_client([stopped_response()]);
    let auth = Arc::new(StaticHeaderAuth::new(BTreeMap::from([(
        "X-MiniMax-Test".to_owned(),
        "injected".to_owned(),
    )])));
    MiniMaxModel::with_auth(injected_client, "MiniMax-M3", auth)
        .complete(&simple_request())
        .await
        .expect("injected auth request should succeed");
    assert_eq!(
        injected_requests
            .lock()
            .expect("requests lock should not be poisoned")[0]
            .headers
            .get("X-MiniMax-Test"),
        Some(&"injected".to_owned())
    );
}

fn simple_request() -> ModelRequest {
    ModelRequest {
        system_prompt: "system".to_owned(),
        messages: Vec::new(),
        tools: Vec::new(),
        response_schema: None,
    }
}

fn stopped_response() -> JsonHttpResponse<serde_json::Value> {
    JsonHttpResponse {
        status: 200,
        body: json!({
            "choices": [{
                "finish_reason": "stop",
                "message": {"content": "Done"}
            }]
        }),
    }
}
