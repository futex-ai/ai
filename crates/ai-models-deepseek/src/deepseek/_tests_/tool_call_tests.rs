//! DeepSeek tool request and response tests.

use ai_interface::{
    ConversationMessage, DeepSeekToolCallContext, FinishReason, ModelError,
    ProviderConversationItem, ToolCall, ToolDefinition,
};
use ai_models_core::ThinkingLevel;
use serde_json::{Value, json};

use crate::DEEPSEEK_V4_PRO;

use super::{request::build_request, response::parse_response, test_support::simple_request};

#[test]
fn serializes_tools_normalized_calls_and_matching_tool_results() {
    let mut request = simple_request();
    request.tools = vec![ToolDefinition {
        name: "memory_read".to_owned(),
        description: "Read retained memory".to_owned(),
        input_schema: json!({
            "type": "object",
            "properties": {"path": {"type": "string"}},
            "required": ["path"]
        }),
        activity_verb: Some("Remembering".to_owned()),
    }];
    request.messages.push(ConversationMessage::assistant(
        "",
        vec![
            normalized_call("call_1", "memory_read", json!({"path": "one"})),
            normalized_call("call_2", "memory_read", json!({"path": "two"})),
        ],
    ));
    request.messages.push(ConversationMessage::tool(
        "first result",
        "memory_read",
        "call_1",
    ));
    let body = request_json(ThinkingLevel::Disabled, &request);
    let object = body.as_object().expect("request object");
    let tool = &body["tools"][0];
    let assistant = &body["messages"][2];
    let result = &body["messages"][3];

    assert!(!object.contains_key("tool_choice"));
    assert_eq!(tool["type"], "function");
    assert_eq!(tool["function"]["name"], "memory_read");
    assert_eq!(tool["function"]["description"], "Read retained memory");
    assert_eq!(
        tool["function"]["parameters"],
        request.tools[0].input_schema
    );
    assert!(tool["function"].get("strict").is_none());
    assert_eq!(assistant["content"], "");
    assert_eq!(assistant["tool_calls"][0]["id"], "call_1");
    assert_eq!(assistant["tool_calls"][1]["id"], "call_2");
    assert_eq!(result["role"], "tool");
    assert_eq!(result["content"], "first result");
    assert_eq!(result["tool_call_id"], "call_1");
}

#[test]
fn parses_parallel_calls_and_preserves_ids_order_and_raw_arguments() {
    let response = parse(
        ThinkingLevel::High,
        tool_response(vec![
            raw_call("call_1", "memory_read", "{ \"path\": \"one\" }"),
            raw_call(
                "call_2",
                "memory_write",
                "{\n  \"path\": \"two\",\n  \"value\": 2\n}",
            ),
        ]),
    )
    .expect("parallel DeepSeek calls should parse");

    assert_eq!(response.finish_reason, FinishReason::ToolCalls);
    assert_eq!(
        response
            .tool_calls
            .iter()
            .map(|call| (call.id.as_str(), call.name.as_str()))
            .collect::<Vec<_>>(),
        vec![("call_1", "memory_read"), ("call_2", "memory_write")]
    );
    assert_eq!(response.tool_calls[0].input, json!({"path": "one"}));
    assert_eq!(
        response.provider_context,
        vec![ProviderConversationItem::DeepSeekAssistantMessage {
            content: String::new(),
            reasoning_content: Some("Retain this reasoning.".to_owned()),
            tool_calls: vec![
                DeepSeekToolCallContext {
                    id: "call_1".to_owned(),
                    name: "memory_read".to_owned(),
                    arguments: "{ \"path\": \"one\" }".to_owned(),
                },
                DeepSeekToolCallContext {
                    id: "call_2".to_owned(),
                    name: "memory_write".to_owned(),
                    arguments: "{\n  \"path\": \"two\",\n  \"value\": 2\n}".to_owned(),
                },
            ],
        }]
    );
}

#[test]
fn rejects_absent_null_empty_partial_and_malformed_dispatchable_calls() {
    let messages = [
        json!({"content": null, "reasoning_content": "reasoning"}),
        json!({"content": null, "reasoning_content": "reasoning", "tool_calls": null}),
        json!({"content": null, "reasoning_content": "reasoning", "tool_calls": []}),
        json!({
            "content": null,
            "reasoning_content": "reasoning",
            "tool_calls": [{
                "id": "call_1",
                "type": "function",
                "function": {"name": "memory_read"}
            }]
        }),
        json!({
            "content": null,
            "reasoning_content": "reasoning",
            "tool_calls": [raw_call("call_1", "memory_read", "{\"path\":")]
        }),
    ];

    for message in messages {
        let result = parse(
            ThinkingLevel::High,
            json!({
                "choices": [{
                    "finish_reason": "tool_calls",
                    "message": message
                }]
            }),
        );

        assert!(matches!(result, Err(ModelError::Provider { .. })));
    }
}

#[test]
fn non_tool_finishes_ignore_and_suppress_malformed_tool_payloads() {
    for finish_reason in [
        Some("stop"),
        Some("length"),
        Some("content_filter"),
        Some("custom"),
        None,
    ] {
        let response = parse(
            ThinkingLevel::High,
            json!({
                "choices": [{
                    "finish_reason": finish_reason,
                    "message": {
                        "content": "Terminal",
                        "reasoning_content": "private",
                        "tool_calls": [{
                            "id": "call_1",
                            "type": "function",
                            "function": {"name": "memory_read"}
                        }]
                    }
                }]
            }),
        )
        .expect("terminal tool payload should not be parsed");

        assert!(response.tool_calls.is_empty());
        assert!(response.provider_context.is_empty());
    }
}

#[test]
fn enabled_thinking_requires_reasoning_but_disabled_thinking_does_not() {
    let missing_reasoning = json!({
        "choices": [{
            "finish_reason": "tool_calls",
            "message": {
                "content": null,
                "tool_calls": [raw_call("call_1", "memory_read", "{}")]
            }
        }]
    });

    assert!(matches!(
        parse(ThinkingLevel::High, missing_reasoning.clone()),
        Err(ModelError::Provider { .. })
    ));
    let response = parse(ThinkingLevel::Disabled, missing_reasoning)
        .expect("disabled-thinking tool call should not require reasoning");
    assert_eq!(response.finish_reason, FinishReason::ToolCalls);
}

fn parse(
    thinking_level: ThinkingLevel,
    body: Value,
) -> Result<ai_interface::ModelResponse, ModelError> {
    parse_response(DEEPSEEK_V4_PRO, DEEPSEEK_V4_PRO, thinking_level, body, None)
}

fn request_json(thinking_level: ThinkingLevel, request: &ai_interface::ModelRequest) -> Value {
    serde_json::to_value(
        build_request(DEEPSEEK_V4_PRO, thinking_level, request)
            .expect("plain request should build"),
    )
    .expect("DeepSeek request should serialize")
}

fn tool_response(tool_calls: Vec<Value>) -> Value {
    json!({
        "choices": [{
            "finish_reason": "tool_calls",
            "message": {
                "content": null,
                "reasoning_content": "Retain this reasoning.",
                "tool_calls": tool_calls
            }
        }]
    })
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
