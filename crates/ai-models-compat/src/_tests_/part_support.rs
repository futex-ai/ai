//! Shared part fixtures and replay assertions for compatible adapters.

use ai_interface::{
    ConversationMessage, ConversationPart, ConversationRole, ModelResponse, ToolCall,
    read_stored_message,
};
use serde_json::{Value, json};

use crate::{deepseek, engine::replay_state::PartState, kimi, minimax, qwen, xai};

pub(crate) fn assistant_with_items(
    text: impl Into<String>,
    calls: Vec<ToolCall>,
    items: Vec<Value>,
) -> ConversationMessage {
    read_stored_message(
        json!({
            "role": "assistant", "content": text.into(), "tool_calls": calls,
            "provider_context": items,
        }),
        &[
            &deepseek::LegacyConverter,
            &kimi::LegacyConverter,
            &qwen::LegacyConverter,
            &minimax::LegacyConverter,
            &xai::LegacyConverter,
        ],
    )
    .expect("legacy assistant converts")
}

pub(crate) fn replay_items(response: &ModelResponse) -> Vec<Value> {
    let mut content = Value::Null;
    let mut reasoning = Value::Null;
    let mut calls = Vec::new();
    let mut details = Vec::new();
    let mut owns_assistant = false;
    let mut owns_reasoning = false;
    let mut legacy = Vec::new();
    for state in response
        .parts
        .iter()
        .filter_map(|part| part.provider_state())
    {
        let payload: PartState = serde_json::from_value(Value::Object(state.payload.clone()))
            .expect("part state decodes");
        match payload {
            PartState::Text { content: raw } => {
                content = json!(raw);
                owns_assistant = true;
            }
            PartState::Reasoning {
                reasoning_content,
                reasoning_details,
            } => {
                reasoning = json!(reasoning_content);
                details = reasoning_details;
                owns_reasoning = true;
            }
            PartState::ToolCall { call, legacy: true } => legacy.push(json!({
                "type": "xai_legacy_function_call", "tool_call_id": call.id,
                "name": call.name, "arguments": call.arguments,
            })),
            PartState::ToolCall {
                call,
                legacy: false,
            } => calls.push(json!(call)),
        }
    }
    match response.provider.as_str() {
        "minimax" if owns_reasoning => vec![json!({ "type": "minimax_assistant",
            "reasoning_content": reasoning, "reasoning_details": details })],
        "deepseek" | "kimi" | "qwen" if owns_assistant => vec![json!({
            "type": match response.provider.as_str() {
                "deepseek" => "deep_seek_assistant_message", "kimi" => "kimi_assistant_message",
                _ => "qwen_assistant_message",
            }, "content": content, "reasoning_content": reasoning, "tool_calls": calls,
        })],
        "xai" => legacy,
        _ => Vec::new(),
    }
}

pub(crate) fn message(
    role: ConversationRole,
    text: String,
    parts: Vec<ConversationPart>,
    name: Option<String>,
    call_id: Option<String>,
    calls: Vec<ToolCall>,
) -> ConversationMessage {
    let mut message = match role {
        ConversationRole::User if !parts.is_empty() => ConversationMessage::user_with_parts(parts),
        ConversationRole::User => ConversationMessage::user(text),
        ConversationRole::Assistant if !parts.is_empty() => ConversationMessage::new(role, parts),
        ConversationRole::Assistant => ConversationMessage::assistant(text, calls),
        ConversationRole::Tool => ConversationMessage::tool(
            text,
            name.clone().unwrap_or_default(),
            call_id.unwrap_or_default(),
        ),
    };
    message.name = name;
    message
}
