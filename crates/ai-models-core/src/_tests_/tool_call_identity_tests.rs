use ai_interface::{ConversationMessage, ModelRequest};

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

fn request_with_message(content: &str) -> ModelRequest {
    ModelRequest {
        system_prompt: "system".to_owned(),
        messages: vec![ConversationMessage::user(content)],
        tools: Vec::new(),
        response_schema: None,
    }
}
