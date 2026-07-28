use ai_interface::{
    ConversationMessage, KimiToolCallContext, ModelRequest, ProviderConversationItem,
};

use crate::{synthetic_tool_call_id, synthetic_tool_call_scope};

#[test]
fn synthetic_scope_changes_with_request_history() {
    let first = synthetic_tool_call_scope(&request_with_message("first request"));
    let second = synthetic_tool_call_scope(&request_with_message("second request"));

    assert_ne!(first, second);
}

#[test]
fn synthetic_tool_call_id_changes_with_arguments() {
    let scope = synthetic_tool_call_scope(&request_with_message("same request"));
    let first = synthetic_tool_call_id(
        "provider_tool_call:",
        &scope,
        0,
        "memory_read",
        "{\"path\":\"one\"}",
    );
    let second = synthetic_tool_call_id(
        "provider_tool_call:",
        &scope,
        0,
        "memory_read",
        "{\"path\":\"two\"}",
    );

    assert_ne!(first, second);
}

#[test]
fn synthetic_scope_hashes_every_kimi_replay_field() {
    let baseline = kimi_request(
        Some("visible"),
        Some("reasoning"),
        "call_1",
        "memory_read",
        "{\"path\":\"one\"}",
    );
    let baseline_scope = synthetic_tool_call_scope(&baseline);
    let variants = [
        kimi_request(
            Some("changed"),
            Some("reasoning"),
            "call_1",
            "memory_read",
            "{\"path\":\"one\"}",
        ),
        kimi_request(
            Some("visible"),
            Some("changed"),
            "call_1",
            "memory_read",
            "{\"path\":\"one\"}",
        ),
        kimi_request(
            Some("visible"),
            Some("reasoning"),
            "call_2",
            "memory_read",
            "{\"path\":\"one\"}",
        ),
        kimi_request(
            Some("visible"),
            Some("reasoning"),
            "call_1",
            "memory_write",
            "{\"path\":\"one\"}",
        ),
        kimi_request(
            Some("visible"),
            Some("reasoning"),
            "call_1",
            "memory_read",
            "{\"path\":\"two\"}",
        ),
    ];

    for variant in variants {
        assert_ne!(synthetic_tool_call_scope(&variant), baseline_scope);
    }
}

fn request_with_message(content: &str) -> ModelRequest {
    ModelRequest {
        system_prompt: "system".to_owned(),
        messages: vec![ConversationMessage::user(content)],
        tools: Vec::new(),
        response_schema: None,
    }
}

fn kimi_request(
    content: Option<&str>,
    reasoning_content: Option<&str>,
    id: &str,
    name: &str,
    arguments: &str,
) -> ModelRequest {
    ModelRequest {
        system_prompt: "system".to_owned(),
        messages: vec![ConversationMessage::assistant_with_provider_context(
            content.unwrap_or_default(),
            Vec::new(),
            vec![ProviderConversationItem::KimiAssistantMessage {
                content: content.map(str::to_owned),
                reasoning_content: reasoning_content.map(str::to_owned),
                tool_calls: vec![KimiToolCallContext {
                    id: id.to_owned(),
                    name: name.to_owned(),
                    arguments: arguments.to_owned(),
                }],
            }],
        )],
        tools: Vec::new(),
        response_schema: None,
    }
}
