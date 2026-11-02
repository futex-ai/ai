//! Legacy assistant-function and paired-result replay.

use ai_interface::{ConversationMessage, ConversationRole, ProviderConversationItem};

#[derive(Clone, Debug)]
pub(super) struct LegacyFunctionCall {
    pub(super) tool_call_id: String,
    pub(super) name: String,
    pub(super) arguments: String,
}

pub(super) fn legacy_function_call(message: &ConversationMessage) -> Option<LegacyFunctionCall> {
    if message.role != ConversationRole::Assistant {
        return None;
    }
    message.provider_context.iter().find_map(|item| match item {
        ProviderConversationItem::XaiLegacyFunctionCall {
            tool_call_id,
            name,
            arguments,
        } if message
            .tool_calls
            .iter()
            .any(|call| call.id == *tool_call_id) =>
        {
            Some(LegacyFunctionCall {
                tool_call_id: tool_call_id.clone(),
                name: name.clone(),
                arguments: arguments.clone(),
            })
        }
        ProviderConversationItem::AnthropicAssistantMessage { .. }
        | ProviderConversationItem::GoogleAssistantMessage { .. }
        | ProviderConversationItem::DeepSeekAssistantMessage { .. }
        | ProviderConversationItem::OpenAiMessage { .. }
        | ProviderConversationItem::OpenAiReasoning { .. }
        | ProviderConversationItem::OpenAiFunctionCall { .. }
        | ProviderConversationItem::KimiAssistantMessage { .. }
        | ProviderConversationItem::MiniMaxAssistant { .. }
        | ProviderConversationItem::QwenAssistantMessage { .. }
        | ProviderConversationItem::XaiLegacyFunctionCall { .. } => None,
    })
}

pub(super) fn legacy_tool_name<'a>(
    message: &ConversationMessage,
    legacy_calls: &'a [LegacyFunctionCall],
) -> Option<&'a str> {
    if message.role != ConversationRole::Tool {
        return None;
    }
    let tool_call_id = message.tool_call_id.as_deref()?;
    legacy_calls
        .iter()
        .rev()
        .find(|call| call.tool_call_id == tool_call_id)
        .map(|call| call.name.as_str())
}
