//! Lossless replay of each dialect's own assistant state.

use ai_interface::{ConversationMessage, ProviderConversationItem};

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
            _ => return None,
        };
        Some(ChatCompletionsMessage {
            role: "assistant",
            content: content.map(ChatCompletionsContent::Text),
            name: None,
            tool_call_id: None,
            reasoning_content,
            tool_calls,
        })
    })
}
