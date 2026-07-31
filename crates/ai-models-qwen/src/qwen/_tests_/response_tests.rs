//! Qwen response, tool-call, replay, usage, and schema tests.

use ai_interface::{
    FinishReason, ModelError, ProviderConversationItem, QwenToolCallContext, StructuredOutputSchema,
};
use ai_models_core::ThinkingLevel;
use serde_json::{Value, json};

use crate::QWEN_3_7_PLUS;

use super::response::parse_response;

#[test]
fn maps_text_metadata_finish_and_provider_replay() {
    let response = parse(
        ThinkingLevel::High,
        json!({
            "choices": [{
                "finish_reason": "stop",
                "message": {
                    "content": "Done",
                    "reasoning_content": "private reasoning"
                }
            }]
        }),
        None,
    )
    .expect("stopped response should parse");

    assert_eq!(response.provider, "qwen");
    assert_eq!(response.model_id, QWEN_3_7_PLUS);
    assert_eq!(response.catalog_model_id.as_deref(), Some("catalog-plus"));
    assert_eq!(response.thinking_level.as_deref(), Some("high"));
    assert_eq!(response.assistant_message, "Done");
    assert_eq!(response.finish_reason, FinishReason::Stop);
    assert!(response.tool_calls.is_empty());
    assert_eq!(
        response.provider_context,
        vec![ProviderConversationItem::QwenAssistantMessage {
            content: Some("Done".to_owned()),
            reasoning_content: Some("private reasoning".to_owned()),
            tool_calls: Vec::new(),
        }]
    );
}

#[test]
fn normalizes_nullable_content_and_every_finish_reason() {
    let cases = [
        (Some("stop"), FinishReason::Stop),
        (Some("tool_calls"), FinishReason::ToolCalls),
        (Some("length"), FinishReason::Truncated),
        (Some("content_filter"), FinishReason::Filtered),
        (Some("future"), FinishReason::Other("future".to_owned())),
        (None, FinishReason::Other("missing".to_owned())),
    ];

    for (raw, expected) in cases {
        let mut message = json!({"content": null, "reasoning_content": "private"});
        if raw == Some("tool_calls") {
            message["tool_calls"] = json!([raw_call("call_1", "memory_read", "{}")]);
        }
        let mut choice = json!({"message": message});
        if let Some(raw) = raw {
            choice["finish_reason"] = json!(raw);
        }
        let response = parse(ThinkingLevel::High, json!({"choices": [choice]}), None)
            .expect("recognized response shape should parse");

        assert_eq!(response.assistant_message, "");
        assert_eq!(response.finish_reason, expected);
    }
}

#[test]
fn parses_parallel_calls_and_preserves_raw_arguments_in_order() {
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
        None,
    )
    .expect("parallel Qwen calls should parse");

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
        vec![ProviderConversationItem::QwenAssistantMessage {
            content: None,
            reasoning_content: Some("Retain this reasoning.".to_owned()),
            tool_calls: vec![
                QwenToolCallContext {
                    id: "call_1".to_owned(),
                    name: "memory_read".to_owned(),
                    arguments: "{ \"path\": \"one\" }".to_owned(),
                },
                QwenToolCallContext {
                    id: "call_2".to_owned(),
                    name: "memory_write".to_owned(),
                    arguments: "{\n  \"path\": \"two\",\n  \"value\": 2\n}".to_owned(),
                },
            ],
        }]
    );
}

#[test]
fn enforces_dispatchable_tool_shape_and_thinking_reasoning() {
    let malformed_messages = [
        json!({"content": null, "reasoning_content": "private"}),
        json!({"content": null, "reasoning_content": "private", "tool_calls": []}),
        json!({
            "content": null,
            "reasoning_content": "private",
            "tool_calls": [{"id": "call_1", "function": {"name": "memory_read"}}]
        }),
        json!({
            "content": null,
            "reasoning_content": "private",
            "tool_calls": [raw_call("call_1", "memory_read", "{")]
        }),
        json!({
            "content": null,
            "reasoning_content": "private",
            "tool_calls": [raw_call("", "memory_read", "{}")]
        }),
        json!({
            "content": null,
            "reasoning_content": "private",
            "tool_calls": [raw_call(" ", "memory_read", "{}")]
        }),
        json!({
            "content": null,
            "reasoning_content": "private",
            "tool_calls": [raw_call("call_1", "", "{}")]
        }),
        json!({
            "content": null,
            "reasoning_content": "private",
            "tool_calls": [raw_call("call_1", " ", "{}")]
        }),
    ];

    for message in malformed_messages {
        let result = parse(
            ThinkingLevel::High,
            json!({"choices": [{"finish_reason": "tool_calls", "message": message}]}),
            None,
        );
        assert!(matches!(result, Err(ModelError::Provider { .. })));
    }

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
        parse(ThinkingLevel::High, missing_reasoning.clone(), None),
        Err(ModelError::Provider { .. })
    ));
    assert!(
        parse(ThinkingLevel::Disabled, missing_reasoning, None).is_ok(),
        "disabled thinking should not require reasoning content"
    );
}

#[test]
fn non_tool_finishes_suppress_malformed_tool_payloads() {
    let response = parse(
        ThinkingLevel::High,
        json!({
            "choices": [{
                "finish_reason": "stop",
                "message": {
                    "content": "Terminal",
                    "reasoning_content": "private",
                    "tool_calls": [{"invalid": true}]
                }
            }]
        }),
        None,
    )
    .expect("terminal tool payload should not be parsed");

    assert!(response.tool_calls.is_empty());
    assert_eq!(
        response.provider_context,
        vec![ProviderConversationItem::QwenAssistantMessage {
            content: Some("Terminal".to_owned()),
            reasoning_content: Some("private".to_owned()),
            tool_calls: Vec::new(),
        }]
    );
}

fn parse(
    thinking_level: ThinkingLevel,
    body: Value,
    schema: Option<&StructuredOutputSchema>,
) -> Result<ai_interface::ModelResponse, ModelError> {
    parse_response("catalog-plus", QWEN_3_7_PLUS, thinking_level, body, schema)
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
        "function": {"name": name, "arguments": arguments}
    })
}
