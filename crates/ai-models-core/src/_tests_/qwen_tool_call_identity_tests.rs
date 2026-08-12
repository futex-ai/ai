use ai_interface::{
    ConversationMessage, ModelRequest, ProviderConversationItem, QwenToolCallContext,
};

use crate::synthetic_tool_call_scope;

#[test]
fn synthetic_scope_hashes_every_qwen_replay_field() {
    let baseline = qwen_request(
        Some("visible"),
        Some("reasoning"),
        "call_1",
        "memory_read",
        "{\"path\":\"one\"}",
    );
    let baseline_scope = synthetic_tool_call_scope(&baseline);
    let variants = [
        qwen_request(
            Some("changed"),
            Some("reasoning"),
            "call_1",
            "memory_read",
            "{\"path\":\"one\"}",
        ),
        qwen_request(
            Some("visible"),
            Some("changed"),
            "call_1",
            "memory_read",
            "{\"path\":\"one\"}",
        ),
        qwen_request(
            Some("visible"),
            Some("reasoning"),
            "call_2",
            "memory_read",
            "{\"path\":\"one\"}",
        ),
        qwen_request(
            Some("visible"),
            Some("reasoning"),
            "call_1",
            "memory_write",
            "{\"path\":\"one\"}",
        ),
        qwen_request(
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

fn qwen_request(
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
            vec![ProviderConversationItem::QwenAssistantMessage {
                content: content.map(str::to_owned),
                reasoning_content: reasoning_content.map(str::to_owned),
                tool_calls: vec![QwenToolCallContext {
                    id: id.to_owned(),
                    name: name.to_owned(),
                    arguments: arguments.to_owned(),
                }],
            }],
        )],
        tools: Vec::new(),
        response_schema: None,
        controls: Default::default(),
    }
}
