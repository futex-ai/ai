//! DeepSeek raw assistant continuation replay tests.

use ai_interface::{
    ConversationMessage, DeepSeekToolCallContext, ModelRequest, ProviderConversationItem, ToolCall,
};
use ai_models_core::ThinkingLevel;
use serde_json::{Value, json};

use crate::DEEPSEEK_V4_PRO;

use super::{request::build_request, response::parse_response};

#[test]
fn response_context_survives_serde_and_replays_exactly_before_tool_results() {
    let raw_first = "{ \"path\": \"one\" }";
    let raw_second = "{\n  \"path\": \"two\",\n  \"value\": 2\n}";
    let response = parse_response(
        DEEPSEEK_V4_PRO,
        DEEPSEEK_V4_PRO,
        ThinkingLevel::High,
        json!({
            "choices": [{
                "finish_reason": "tool_calls",
                "message": {
                    "content": null,
                    "reasoning_content": "Need both results.",
                    "tool_calls": [
                        raw_call("call_1", "memory_read", raw_first),
                        raw_call("call_2", "memory_write", raw_second)
                    ]
                }
            }]
        }),
        None,
    )
    .expect("tool response should parse");
    let serialized =
        serde_json::to_value(&response.provider_context).expect("context should serialize");
    let provider_context = serde_json::from_value(serialized).expect("context should deserialize");
    let request = ModelRequest {
        system_prompt: "system".to_owned(),
        messages: vec![
            ConversationMessage::user("start"),
            ConversationMessage::assistant_with_provider_context(
                response.assistant_message,
                response.tool_calls,
                provider_context,
            ),
            ConversationMessage::tool("first", "memory_read", "call_1"),
            ConversationMessage::tool("second", "memory_write", "call_2"),
        ],
        tools: Vec::new(),
        response_schema: None,
    };
    let body = request_json(&request);
    let messages = body["messages"].as_array().expect("messages array");
    let assistant = &messages[2];

    assert_eq!(assistant["content"], "");
    assert_eq!(assistant["reasoning_content"], "Need both results.");
    assert_eq!(
        assistant["tool_calls"][0]["function"]["arguments"],
        raw_first
    );
    assert_eq!(
        assistant["tool_calls"][1]["function"]["arguments"],
        raw_second
    );
    assert_eq!(messages[3]["tool_call_id"], "call_1");
    assert_eq!(messages[4]["tool_call_id"], "call_2");
}

#[test]
fn deepseek_context_precedes_normalized_fields_and_foreign_context_is_ignored() {
    let request = ModelRequest {
        system_prompt: "system".to_owned(),
        messages: vec![ConversationMessage::assistant_with_provider_context(
            "normalized text",
            vec![normalized_call(
                "normalized_id",
                "normalized_name",
                json!({"normalized": true}),
            )],
            vec![
                ProviderConversationItem::KimiAssistantMessage {
                    content: Some("foreign content".to_owned()),
                    reasoning_content: Some("foreign reasoning".to_owned()),
                    tool_calls: Vec::new(),
                },
                ProviderConversationItem::DeepSeekAssistantMessage {
                    content: "raw text".to_owned(),
                    reasoning_content: Some("raw reasoning".to_owned()),
                    tool_calls: vec![DeepSeekToolCallContext {
                        id: "raw_id".to_owned(),
                        name: "raw_name".to_owned(),
                        arguments: "{ \"raw\": true }".to_owned(),
                    }],
                },
            ],
        )],
        tools: Vec::new(),
        response_schema: None,
    };
    let body = request_json(&request);
    let assistant = &body["messages"][1];

    assert_eq!(assistant["content"], "raw text");
    assert_eq!(assistant["reasoning_content"], "raw reasoning");
    assert_eq!(assistant["tool_calls"][0]["id"], "raw_id");
    assert_eq!(
        assistant["tool_calls"][0]["function"]["arguments"],
        "{ \"raw\": true }"
    );
    assert!(!body.to_string().contains("foreign"));
}

fn request_json(request: &ModelRequest) -> Value {
    serde_json::to_value(
        build_request(DEEPSEEK_V4_PRO, ThinkingLevel::High, request)
            .expect("plain request should build"),
    )
    .expect("DeepSeek continuation should serialize")
}

fn raw_call(id: &str, name: &str, arguments: &str) -> Value {
    json!({
        "id": id,
        "type": "function",
        "function": {
            "name": name,
            "arguments": arguments
        }
    })
}

fn normalized_call(id: &str, name: &str, input: Value) -> ToolCall {
    ToolCall {
        id: id.to_owned(),
        name: name.to_owned(),
        input,
        operation_id: None,
    }
}
