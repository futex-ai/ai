//! Kimi raw assistant continuation replay tests.

use ai_interface::{
    ConversationMessage, KimiToolCallContext, ModelRequest, ProviderConversationItem, ToolCall,
};
use serde_json::{Value, json};

use crate::KIMI_K3;

use super::{client::KimiReasoningEffort, request::build_request};

#[test]
fn replays_exact_kimi_assistant_before_matching_tool_results() {
    let raw_first = "{ \"path\": \"one\" }";
    let raw_second = "{\n  \"path\": \"two\",\n  \"value\": 2\n}";
    let request = ModelRequest {
        system_prompt: "system".to_owned(),
        messages: vec![
            ConversationMessage::user("start"),
            ConversationMessage::assistant_with_provider_context(
                "",
                vec![
                    normalized_call("call_1", "memory_read", json!({"path": "one"})),
                    normalized_call("call_2", "memory_write", json!({"path": "two", "value": 2})),
                ],
                vec![ProviderConversationItem::KimiAssistantMessage {
                    content: None,
                    reasoning_content: Some("Need both results.".to_owned()),
                    tool_calls: vec![
                        KimiToolCallContext {
                            id: "call_1".to_owned(),
                            name: "memory_read".to_owned(),
                            arguments: raw_first.to_owned(),
                        },
                        KimiToolCallContext {
                            id: "call_2".to_owned(),
                            name: "memory_write".to_owned(),
                            arguments: raw_second.to_owned(),
                        },
                    ],
                }],
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

    assert_eq!(assistant["content"], Value::Null);
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
fn kimi_context_takes_precedence_over_normalized_assistant_fields() {
    let request = ModelRequest {
        system_prompt: "system".to_owned(),
        messages: vec![ConversationMessage::assistant_with_provider_context(
            "normalized text",
            vec![normalized_call(
                "normalized_id",
                "normalized_name",
                json!({"normalized": true}),
            )],
            vec![ProviderConversationItem::KimiAssistantMessage {
                content: Some("raw text".to_owned()),
                reasoning_content: Some("raw reasoning".to_owned()),
                tool_calls: vec![KimiToolCallContext {
                    id: "raw_id".to_owned(),
                    name: "raw_name".to_owned(),
                    arguments: "{ \"raw\": true }".to_owned(),
                }],
            }],
        )],
        tools: Vec::new(),
        response_schema: None,
    };
    let body = request_json(&request);
    let assistant = &body["messages"][1];

    assert_eq!(assistant["content"], "raw text");
    assert_eq!(assistant["tool_calls"][0]["id"], "raw_id");
    assert_eq!(
        assistant["tool_calls"][0]["function"]["arguments"],
        "{ \"raw\": true }"
    );
}

fn request_json(request: &ModelRequest) -> Value {
    serde_json::to_value(build_request(KIMI_K3, KimiReasoningEffort::Max, request))
        .expect("Kimi continuation should serialize")
}

fn normalized_call(id: &str, name: &str, input: Value) -> ToolCall {
    ToolCall {
        id: id.to_owned(),
        name: name.to_owned(),
        input,
        operation_id: None,
    }
}
