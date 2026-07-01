//! Tests for Gemini synthetic tool-call operation identity.

use ai_interface::{ConversationMessage, Model, ModelRequest};
use json_http::JsonHttpResponse;
use serde_json::json;

use super::{GoogleModel, recording_http_client};

#[tokio::test]
async fn no_id_function_calls_get_request_scoped_operation_ids() {
    let first = complete_no_id_tool_call("first request").await;
    let second = complete_no_id_tool_call("second request").await;
    let first_call = &first.tool_calls[0];
    let second_call = &second.tool_calls[0];

    assert_ne!(first_call.id, second_call.id);
    assert_eq!(
        first_call.operation_id.as_deref(),
        Some(first_call.id.as_str())
    );
    assert_eq!(
        second_call.operation_id.as_deref(),
        Some(second_call.id.as_str())
    );
}

async fn complete_no_id_tool_call(content: &str) -> ai_interface::ModelResponse {
    let (http_client, _) = recording_http_client(JsonHttpResponse {
        status: 200,
        body: json!({
            "candidates": [{
                "finishReason": "STOP",
                "content": {
                    "parts": [{
                        "functionCall": {
                            "name": "memory_read",
                            "args": { "path": "root" }
                        }
                    }]
                }
            }]
        }),
    });
    let model = GoogleModel::new(http_client, "gemini-2.5-pro", "google-key");

    model
        .complete(&ModelRequest {
            system_prompt: "system".to_owned(),
            messages: vec![ConversationMessage::user(content)],
            tools: Vec::new(),
            response_schema: None,
        })
        .await
        .expect("Google no-id function call should parse")
}
