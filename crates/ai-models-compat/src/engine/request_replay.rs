//! Lossless replay of each dialect's own assistant state.

use ai_interface::{
    ConversationMessage, ConversationRole, MiniMaxReasoningDetail, ProviderConversationItem,
};

use super::{
    dialect::ReplayRule,
    request_types::{ChatCompletionsContent, ChatCompletionsMessage},
    tools::raw_tool_call,
};

pub(super) fn replay_message(
    rule: ReplayRule,
    message: &ConversationMessage,
) -> Option<ChatCompletionsMessage> {
    message.provider_context.iter().find_map(|item| {
        let (content, reasoning_content, tool_calls) = match (rule, item) {
            (
                ReplayRule::DeepSeek,
                ProviderConversationItem::DeepSeekAssistantMessage {
                    content,
                    reasoning_content,
                    tool_calls,
                },
            ) => (
                Some(content.clone()),
                reasoning_content.clone(),
                tool_calls
                    .iter()
                    .map(|call| raw_tool_call(&call.id, &call.name, &call.arguments))
                    .collect(),
            ),
            (
                ReplayRule::Kimi,
                ProviderConversationItem::KimiAssistantMessage {
                    content,
                    reasoning_content,
                    tool_calls,
                },
            ) => (
                content.clone(),
                reasoning_content.clone(),
                tool_calls
                    .iter()
                    .map(|call| raw_tool_call(&call.id, &call.name, &call.arguments))
                    .collect(),
            ),
            (
                ReplayRule::Qwen,
                ProviderConversationItem::QwenAssistantMessage {
                    content,
                    reasoning_content,
                    tool_calls,
                },
            ) => (
                content
                    .clone()
                    .or_else(|| tool_calls.is_empty().then(String::new)),
                reasoning_content.clone(),
                tool_calls
                    .iter()
                    .map(|call| raw_tool_call(&call.id, &call.name, &call.arguments))
                    .collect(),
            ),
            _ => return None,
        };
        Some(ChatCompletionsMessage {
            role: "assistant",
            content: Some(content.map(ChatCompletionsContent::Text)),
            name: None,
            tool_call_id: None,
            reasoning_content,
            reasoning_details: Vec::new(),
            tool_calls,
        })
    })
}

pub(super) fn reasoning_context(
    rule: ReplayRule,
    message: &ConversationMessage,
) -> (Option<String>, Vec<MiniMaxReasoningDetail>) {
    if rule != ReplayRule::MiniMax || message.role != ConversationRole::Assistant {
        return (None, Vec::new());
    }
    message
        .provider_context
        .iter()
        .find_map(|item| match item {
            ProviderConversationItem::MiniMaxAssistant {
                reasoning_content,
                reasoning_details,
            } => Some((reasoning_content.clone(), reasoning_details.clone())),
            _ => None,
        })
        .unwrap_or_default()
}
