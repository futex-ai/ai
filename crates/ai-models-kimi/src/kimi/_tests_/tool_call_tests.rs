//! Kimi tool request and response tests.

use ai_interface::{
    ConversationMessage, FinishReason, KimiToolCallContext, ModelError, ProviderConversationItem,
    ToolCall, ToolDefinition,
};
use ai_models_core::ThinkingLevel;
use serde_json::{Value, json};

use crate::KIMI_K3;

use super::{
    client::KimiReasoningEffort, request::build_request, response::parse_response,
    test_support::simple_request,
};

#[test]
fn serializes_custom_tools_auto_choice_and_normalized_assistant_calls() {
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
            tool_call("call_1", "memory_read", json!({"path": "one"})),
            tool_call("call_2", "memory_read", json!({"path": "two"})),
        ],
    ));
    request.messages.push(ConversationMessage::tool(
        "first result",
        "memory_read",
        "call_1",
    ));
    let body = build_request(KIMI_K3, KimiReasoningEffort::Max, &request)
        .expect("Kimi request should build");
    let body = serde_json::to_value(body).expect("Kimi request should serialize");
    let tool = &body["tools"][0];
    let assistant = &body["messages"][2];
    let result = &body["messages"][3];

    assert_eq!(body["tool_choice"], "auto");
    assert_eq!(tool["type"], "function");
    assert_eq!(tool["function"]["name"], "memory_read");
    assert_eq!(tool["function"]["description"], "Read retained memory");
    assert_eq!(
        tool["function"]["parameters"],
        request.tools[0].input_schema
    );
    assert!(tool["function"].get("strict").is_none());
    assert_eq!(assistant["tool_calls"][0]["id"], "call_1");
    assert_eq!(assistant["tool_calls"][1]["id"], "call_2");
    assert_eq!(result["role"], "tool");
    assert_eq!(result["tool_call_id"], "call_1");
    assert!(result.get("name").is_none());
}

#[test]
fn parses_ordered_parallel_calls_and_preserves_raw_arguments() {
    let response = parse(tool_response(
        Some("tool_calls"),
        vec![
            raw_call("call_1", "memory_read", "{ \"path\": \"one\" }"),
            raw_call(
                "call_2",
                "memory_write",
                "{\n  \"path\": \"two\",\n  \"value\": 2\n}",
            ),
        ],
    ))
    .expect("parallel Kimi calls should parse");

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
        vec![ProviderConversationItem::KimiAssistantMessage {
            content: None,
            reasoning_content: Some("Retain this reasoning.".to_owned()),
            tool_calls: vec![
                KimiToolCallContext {
                    id: "call_1".to_owned(),
                    name: "memory_read".to_owned(),
                    arguments: "{ \"path\": \"one\" }".to_owned(),
                },
                KimiToolCallContext {
                    id: "call_2".to_owned(),
                    name: "memory_write".to_owned(),
                    arguments: "{\n  \"path\": \"two\",\n  \"value\": 2\n}".to_owned(),
                },
            ],
        }]
    );
}

#[test]
fn invalid_dispatchable_tool_arguments_fail_as_provider_error() {
    let result = parse(tool_response(
        Some("tool_calls"),
        vec![raw_call("call_1", "memory_read", "{\"path\":")],
    ));

    assert!(matches!(result, Err(ModelError::Provider { .. })));
}

#[test]
fn structurally_partial_dispatchable_tool_calls_fail_as_provider_error() {
    let result = parse(json!({
        "choices": [{
            "finish_reason": "tool_calls",
            "message": {
                "content": null,
                "tool_calls": [{
                    "id": "call_1",
                    "type": "function",
                    "function": {
                        "name": "memory_read"
                    }
                }]
            }
        }]
    }));

    assert!(matches!(result, Err(ModelError::Provider { .. })));
}

#[test]
fn empty_dispatchable_tool_call_payloads_fail_as_provider_error() {
    for message in [
        json!({"content": null}),
        json!({"content": null, "tool_calls": null}),
        json!({"content": null, "tool_calls": []}),
    ] {
        let result = parse(json!({
            "choices": [{
                "finish_reason": "tool_calls",
                "message": message
            }]
        }));

        assert!(matches!(result, Err(ModelError::Provider { .. })));
    }
}

#[test]
fn non_tool_finishes_suppress_and_do_not_replay_tool_payloads() {
    for finish_reason in [
        Some("stop"),
        Some("length"),
        Some("content_filter"),
        Some("custom"),
        None,
    ] {
        let response = parse(tool_response(
            finish_reason,
            vec![raw_call("call_1", "memory_read", "{\"path\":")],
        ))
        .expect("terminal tool payload should not be parsed");

        assert!(response.tool_calls.is_empty());
        let ProviderConversationItem::KimiAssistantMessage { tool_calls, .. } =
            &response.provider_context[0]
        else {
            unreachable!("Kimi response should retain Kimi context")
        };
        assert!(tool_calls.is_empty());
    }
}

#[test]
fn non_tool_finishes_ignore_structurally_partial_tool_calls() {
    for finish_reason in [
        Some("stop"),
        Some("length"),
        Some("content_filter"),
        Some("custom"),
        None,
    ] {
        let response = parse(json!({
            "choices": [{
                "finish_reason": finish_reason,
                "message": {
                    "content": "Partial response",
                    "reasoning_content": "Retain this reasoning.",
                    "tool_calls": [{
                        "id": "call_1",
                        "type": "function",
                        "function": {
                            "name": "memory_read"
                        }
                    }]
                }
            }]
        }))
        .expect("terminal partial tool payload should be ignored");

        assert!(response.tool_calls.is_empty());
        let ProviderConversationItem::KimiAssistantMessage { tool_calls, .. } =
            &response.provider_context[0]
        else {
            unreachable!("Kimi response should retain Kimi context")
        };
        assert!(tool_calls.is_empty());
    }
}

fn parse(body: Value) -> Result<ai_interface::ModelResponse, ModelError> {
    parse_response(KIMI_K3, KIMI_K3, ThinkingLevel::Max, body, None)
}

fn tool_response(finish_reason: Option<&str>, tool_calls: Vec<Value>) -> Value {
    json!({
        "choices": [{
            "finish_reason": finish_reason,
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

fn tool_call(id: &str, name: &str, input: Value) -> ToolCall {
    ToolCall {
        id: id.to_owned(),
        name: name.to_owned(),
        input,
        operation_id: None,
    }
}
