//! Tests for xAI synthetic legacy tool-call operation identity.

use std::sync::{Arc, Mutex};

use ai_interface::{ConversationMessage, Model, ModelRequest, ProviderConversationItem};
use json_http::{
    JsonHttpClient, JsonHttpRequest, JsonHttpResponse, JsonHttpTransportMock,
    TransportBackedJsonHttpClient,
};
use serde_json::{Value, json};
use unimock::{MockFn, Unimock, matching};

use super::XaiModel;

type RecordedRequests = Arc<Mutex<Vec<JsonHttpRequest>>>;

#[tokio::test]
async fn legacy_function_calls_get_request_scoped_operation_ids() {
    let first = complete_legacy_tool_call("first request").await;
    let second = complete_legacy_tool_call("second request").await;
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
    let model = XaiModel::new(http_client, "grok-4", "xai-key");

    model
        .complete(&ModelRequest {
            system_prompt: "system".to_owned(),
            messages: vec![ConversationMessage::user(content)],
            tools: Vec::new(),
            response_schema: None,
        })
        .await
        .expect("xAI legacy function call should parse")
}

fn legacy_context_id(response: &ai_interface::ModelResponse) -> Option<&str> {
    response
        .provider_context
        .iter()
        .find_map(|item| match item {
            ProviderConversationItem::XaiLegacyFunctionCall { tool_call_id, .. } => {
                Some(tool_call_id.as_str())
            }
            _ => None,
        })
}

fn recording_http_client(
    response: JsonHttpResponse<Value>,
) -> (Arc<dyn JsonHttpClient>, RecordedRequests) {
    let requests = Arc::new(Mutex::new(Vec::new()));
    let transport = Arc::new(Unimock::new(
        JsonHttpTransportMock::execute
            .each_call(matching!(_))
            .answers_arc({
                let requests = requests.clone();
                Arc::new(move |_, request: &JsonHttpRequest| {
                    requests
                        .lock()
                        .expect("requests lock should not be poisoned")
                        .push(request.clone());
                    Ok(response.clone())
                })
            }),
    ));

    (
        Arc::new(TransportBackedJsonHttpClient::new(transport)),
        requests,
    )
}
