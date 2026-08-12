//! Anthropic prompt-cache message-marker tests.

use ai_interface::{ConversationContentPart, ConversationMessage, ModelRequest, ToolCall};
use ai_models_core::ThinkingLevel;
use serde_json::{Value, json};

use super::{
    cache::{AnthropicCacheTtl, AnthropicPromptCache},
    request::build_request,
};

#[test]
fn single_turn_marks_only_the_final_message_block() {
    let body = serialized_request(&ModelRequest {
        system_prompt: String::new(),
        messages: vec![ConversationMessage::user_with_parts(
            "visual",
            vec![
                ConversationContentPart::Text {
                    text: "inspect".to_owned(),
                },
                ConversationContentPart::Image {
                    mime_type: "image/png".to_owned(),
                    data_base64: "abc123".to_owned(),
                },
            ],
        )],
        tools: Vec::new(),
        response_schema: None,
    });
    let blocks = message_blocks(&body);

    assert_eq!(blocks.len(), 2);
    assert!(blocks[0].get("cache_control").is_none());
    assert_eq!(blocks[1]["cache_control"], json!({"type": "ephemeral"}));
}

#[test]
fn multi_turn_tool_loop_marks_the_final_block() {
    let body = serialized_request(&ModelRequest {
        system_prompt: "system".to_owned(),
        messages: vec![
            ConversationMessage::user("inspect"),
            ConversationMessage::assistant(
                "checking",
                vec![tool_call("call-1"), tool_call("call-2")],
            ),
            ConversationMessage::tool("first", "inspect", "call-1"),
            ConversationMessage::tool("second", "inspect", "call-2"),
        ],
        tools: Vec::new(),
        response_schema: None,
    });
    let blocks = message_blocks(&body);
    let final_block = blocks.last().expect("message block should exist");

    assert_eq!(final_block["type"], "tool_result");
    assert_eq!(final_block["cache_control"], json!({"type": "ephemeral"}));
}

#[test]
fn long_history_uses_tail_stride_and_four_marker_budget() {
    let body = serialized_request(&ModelRequest {
        system_prompt: "system".to_owned(),
        messages: (0..65)
            .map(|index| ConversationMessage::user(format!("block {index}")))
            .collect(),
        tools: Vec::new(),
        response_schema: None,
    });
    let blocks = message_blocks(&body);
    let marked_indices = blocks
        .iter()
        .enumerate()
        .filter_map(|(index, block)| block.get("cache_control").map(|_| index))
        .collect::<Vec<_>>();

    assert_eq!(marked_indices, vec![24, 44, 64]);
    assert!(blocks[4].get("cache_control").is_none());
    assert_eq!(cache_control_count(&body), 4);
}

#[test]
fn markers_use_only_supported_content_block_types() {
    let requests = [
        (
            ModelRequest {
                system_prompt: String::new(),
                messages: vec![ConversationMessage::user("text")],
                tools: Vec::new(),
                response_schema: None,
            },
            "text",
        ),
        (
            ModelRequest {
                system_prompt: String::new(),
                messages: vec![ConversationMessage::user_with_parts(
                    "image",
                    vec![ConversationContentPart::Image {
                        mime_type: "image/png".to_owned(),
                        data_base64: "abc123".to_owned(),
                    }],
                )],
                tools: Vec::new(),
                response_schema: None,
            },
            "image",
        ),
        (
            ModelRequest {
                system_prompt: String::new(),
                messages: vec![ConversationMessage::assistant(
                    String::new(),
                    vec![tool_call("call-1")],
                )],
                tools: Vec::new(),
                response_schema: None,
            },
            "tool_use",
        ),
        (
            ModelRequest {
                system_prompt: String::new(),
                messages: vec![ConversationMessage::tool("done", "inspect", "call-1")],
                tools: Vec::new(),
                response_schema: None,
            },
            "tool_result",
        ),
    ];

    for (request, expected_type) in requests {
        let body = serialized_request(&request);
        let blocks = message_blocks(&body);
        let block = blocks.last().expect("message block should exist");
        assert_eq!(block["type"], expected_type);
        assert!(block.get("cache_control").is_some());
    }
}

fn serialized_request(request: &ModelRequest) -> Value {
    serde_json::to_value(build_request(
        "claude-sonnet-4-6",
        ThinkingLevel::Disabled,
        request,
        AnthropicPromptCache::Enabled {
            ttl: AnthropicCacheTtl::FiveMinutes,
        },
    ))
    .expect("request should serialize")
}

fn message_blocks(body: &Value) -> Vec<&Value> {
    body["messages"]
        .as_array()
        .expect("messages should be an array")
        .iter()
        .flat_map(|message| {
            message["content"]
                .as_array()
                .expect("content should be an array")
        })
        .collect()
}

fn tool_call(id: &str) -> ToolCall {
    ToolCall {
        id: id.to_owned(),
        name: "inspect".to_owned(),
        input: json!({"path": id}),
        operation_id: None,
    }
}

fn cache_control_count(value: &Value) -> usize {
    match value {
        Value::Array(values) => values.iter().map(cache_control_count).sum(),
        Value::Object(values) => {
            usize::from(values.contains_key("cache_control"))
                + values.values().map(cache_control_count).sum::<usize>()
        }
        _ => 0,
    }
}
