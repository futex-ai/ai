//! Tests for xAI synthetic legacy tool-call operation identity.

use ai_interface::{ConversationMessage, Model, ModelRequest};
use json_http::JsonHttpResponse;
use serde_json::json;

use super::test_support::custom_spec;
use super::{XaiModel, test_support::recording_http_client};

#[tokio::test]
async fn legacy_function_calls_get_request_scoped_operation_ids() {
    let first = complete_legacy_tool_call("first request").await;
    let second = complete_legacy_tool_call("second request").await;
    let first_call = &first.tool_calls()[0];
    let second_call = &second.tool_calls()[0];

    assert_ne!(first_call.id, second_call.id);
    assert_eq!(
        first_call.operation_id.as_deref(),
        Some(first_call.id.as_str())
    );
    assert_eq!(
        second_call.operation_id.as_deref(),
        Some(second_call.id.as_str())
    );
    assert_eq!(legacy_context_id(&first), Some(first_call.id.as_str()));
    assert_eq!(legacy_context_id(&second), Some(second_call.id.as_str()));
}

async fn complete_legacy_tool_call(content: &str) -> ai_interface::ModelResponse {
    let (http_client, _) = recording_http_client(JsonHttpResponse {
        status: 200,
        body: json!({
            "choices": [{
                "finish_reason": "function_call",
                "message": {
                    "content": "",
                    "function_call": {
                        "name": "memory_read",
                        "arguments": "{\"path\":\"root\"}"
                    }
                }
            }]
        }),
    });
    let model = XaiModel::new(http_client, custom_spec("grok-4"), "xai-key");

    model
        .complete(&ModelRequest {
            system_prompt: "system".to_owned(),
            messages: vec![ConversationMessage::user(content)],
            tools: Vec::new(),
            response_schema: None,
            controls: Default::default(),
        })
        .await
        .expect("xAI legacy function call should parse")
}

fn legacy_context_id(response: &ai_interface::ModelResponse) -> Option<&str> {
    response
        .parts
        .iter()
        .filter_map(|part| part.provider_state())
        .find_map(|state| {
            let call = state.payload.get("call")?;
            state
                .payload
                .get("legacy")
                .and_then(serde_json::Value::as_bool)
                .filter(|legacy| *legacy)
                .and_then(|_| call.get("id"))
                .and_then(serde_json::Value::as_str)
        })
}
