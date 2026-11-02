use ai_interface::{
    ConversationMessage, ConversationPart, ConversationRole, ModelRequest, ProviderId,
    ProviderState,
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
        messages: vec![ConversationMessage::new(
            ConversationRole::Assistant,
            vec![
                ConversationPart::reasoning("").with_provider_state(ProviderState {
                    provider: ProviderId::new("qwen"),
                    model: String::new(),
                    version: 1,
                    payload: serde_json::json!({
                        "content": content, "reasoning_content": reasoning_content,
                        "tool_calls": [{"id": id, "name": name, "arguments": arguments}],
                    })
                    .as_object()
                    .unwrap()
                    .clone(),
                }),
            ],
        )],
        tools: Vec::new(),
        response_schema: None,
        controls: Default::default(),
    }
}
