//! xAI continuation request regression tests.

use std::sync::{Arc, Mutex};

use ai_interface::{
    ConversationMessage, DeepSeekToolCallContext, KimiToolCallContext, Model, ModelRequest,
    ProviderConversationItem, QwenToolCallContext, ToolCall,
};
use json_http::{
    JsonHttpClient, JsonHttpRequest, JsonHttpResponse, JsonHttpTransportMock,
    TransportBackedJsonHttpClient,
};
use serde_json::{Value, json};
use unimock::{MockFn, Unimock, matching};

use super::XaiModel;

type RecordedRequests = Arc<Mutex<Vec<JsonHttpRequest>>>;

#[tokio::test]
async fn omits_name_from_tool_role_continuation_messages() {
    let (http_client, requests) = recording_http_client(xai_text_response("Done"));
    let model = XaiModel::new(http_client, "grok-4", "xai-key");

    model
        .complete(&ModelRequest {
            system_prompt: "system".to_owned(),
            messages: vec![
                ConversationMessage::user("start"),
                ConversationMessage::assistant(
                    "",
                    vec![ToolCall {
                        id: "call_1".to_owned(),
                        name: "memory_read".to_owned(),
                        input: json!({"path": "root"}),
                        operation_id: None,
                    }],
                ),
                ConversationMessage::tool("{\"ok\":true}", "memory_read", "call_1"),
            ],
            tools: Vec::new(),
            response_schema: None,
            controls: Default::default(),
        })
        .await
        .expect("xAI response should parse");

    let requests = requests
        .lock()
        .expect("requests lock should not be poisoned");
    let body = requests[0].body.as_ref().expect("body present");
    let messages = body["messages"].as_array().expect("messages array");
    let tool_message = messages
        .iter()
        .find(|message| message["role"] == "tool")
        .expect("tool message should be serialized");
    let tool_message_object = tool_message.as_object().expect("tool message object");

    assert_eq!(tool_message["content"], "{\"ok\":true}");
    assert_eq!(tool_message["tool_call_id"], "call_1");
    assert!(!tool_message_object.contains_key("name"));
}

#[tokio::test]
async fn serializes_legacy_function_call_continuation_messages() {
    let (http_client, requests) = recording_http_client(xai_text_response("Done"));
    let model = XaiModel::new(http_client, "grok-4", "xai-key");
    let legacy_call_id = "xai_legacy_function_call:memory_read";

    model
        .complete(&ModelRequest {
            system_prompt: "system".to_owned(),
            messages: vec![
                ConversationMessage::user("start"),
                ConversationMessage::assistant_with_provider_context(
                    "",
                    vec![ToolCall {
                        id: legacy_call_id.to_owned(),
                        name: "memory_read".to_owned(),
                        input: json!({"path": "root"}),
                        operation_id: None,
                    }],
                    vec![
                        ProviderConversationItem::DeepSeekAssistantMessage {
                            content: "DeepSeek-owned content".to_owned(),
                            reasoning_content: Some("DeepSeek-owned reasoning".to_owned()),
                            tool_calls: vec![DeepSeekToolCallContext {
                                id: "deepseek_call".to_owned(),
                                name: "ignored".to_owned(),
                                arguments: "{\"foreign\":true}".to_owned(),
                            }],
                        },
                        ProviderConversationItem::KimiAssistantMessage {
                            content: None,
                            reasoning_content: Some("Kimi-owned context".to_owned()),
                            tool_calls: vec![KimiToolCallContext {
                                id: "kimi_call".to_owned(),
                                name: "ignored".to_owned(),
                                arguments: "{}".to_owned(),
                            }],
                        },
                        ProviderConversationItem::QwenAssistantMessage {
                            content: Some("Qwen-owned content".to_owned()),
                            reasoning_content: Some("Qwen-owned reasoning".to_owned()),
                            tool_calls: vec![QwenToolCallContext {
                                id: "qwen_call".to_owned(),
                                name: "ignored".to_owned(),
                                arguments: "{}".to_owned(),
                            }],
                        },
                        ProviderConversationItem::XaiLegacyFunctionCall {
                            tool_call_id: legacy_call_id.to_owned(),
                            name: "memory_read".to_owned(),
                            arguments: "{\"path\":\"root\"}".to_owned(),
                        },
                    ],
                ),
                ConversationMessage::tool("{\"ok\":true}", "memory_read", legacy_call_id),
            ],
            tools: Vec::new(),
            response_schema: None,
            controls: Default::default(),
        })
        .await
        .expect("xAI response should parse");

    let requests = requests
        .lock()
        .expect("requests lock should not be poisoned");
    let body = requests[0].body.as_ref().expect("body present");
    let messages = body["messages"].as_array().expect("messages array");
    let assistant_message = messages
        .iter()
        .find(|message| message["role"] == "assistant")
        .expect("assistant message should be serialized");
    let function_message = messages
        .iter()
        .find(|message| message["role"] == "function")
        .expect("function message should be serialized");
    let assistant_object = assistant_message
        .as_object()
        .expect("assistant message should be an object");
    let function_object = function_message
        .as_object()
        .expect("function message should be an object");

    assert_eq!(assistant_message["function_call"]["name"], "memory_read");
    assert_eq!(
        assistant_message["function_call"]["arguments"],
        "{\"path\":\"root\"}"
    );
    assert!(!assistant_object.contains_key("tool_calls"));
    assert_eq!(function_message["name"], "memory_read");
    assert_eq!(function_message["content"], "{\"ok\":true}");
    assert!(!function_object.contains_key("tool_call_id"));
    assert!(
        !body
            .as_json()
            .expect("JSON body should be present")
            .to_string()
            .contains("DeepSeek-owned")
    );
    assert!(
        !body
            .as_json()
            .expect("JSON body should be present")
            .to_string()
            .contains("Qwen-owned")
    );
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

fn xai_text_response(text: &str) -> JsonHttpResponse<Value> {
    JsonHttpResponse {
        status: 200,
        body: json!({
            "choices": [{
                "finish_reason": "stop",
                "message": {
                    "content": text,
                    "tool_calls": []
                }
            }]
        }),
    }
}
