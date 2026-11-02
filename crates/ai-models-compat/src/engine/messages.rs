//! Shared conversation and provider-state replay mapping.

use ai_interface::{ConversationMessage, ConversationRole, ProviderConversationItem};

use super::{
    request_types::ChatCompletionsMessage,
    tools::{raw_tool_call, tool_call},
};

pub(super) fn message(message: &ConversationMessage) -> ChatCompletionsMessage {
    if message.role == ConversationRole::Assistant
        && let Some(replay) = deepseek_replay(message)
    {
        return replay_message(replay);
    }
    ChatCompletionsMessage {
        role: role(message.role).to_owned(),
        content: message.content.clone(),
        name: match message.role {
            ConversationRole::User | ConversationRole::Assistant => message.name.clone(),
            ConversationRole::Tool => None,
        },
        tool_call_id: match message.role {
            ConversationRole::Tool => message.tool_call_id.clone(),
            ConversationRole::User | ConversationRole::Assistant => None,
        },
        reasoning_content: None,
        tool_calls: if message.role == ConversationRole::Assistant {
            message.tool_calls.iter().map(tool_call).collect()
        } else {
            Vec::new()
        },
    }
}

fn deepseek_replay(message: &ConversationMessage) -> Option<&ProviderConversationItem> {
    message.provider_context.iter().find(|item| {
        matches!(
            item,
            ProviderConversationItem::DeepSeekAssistantMessage { .. }
        )
    })
}

fn replay_message(item: &ProviderConversationItem) -> ChatCompletionsMessage {
    let ProviderConversationItem::DeepSeekAssistantMessage {
        content,
        reasoning_content,
        tool_calls,
    } = item
    else {
        return ChatCompletionsMessage {
            role: "assistant".to_owned(),
            content: String::new(),
            name: None,
            tool_call_id: None,
            reasoning_content: None,
            tool_calls: Vec::new(),
        };
    };
    ChatCompletionsMessage {
        role: "assistant".to_owned(),
        content: content.clone(),
        name: None,
        tool_call_id: None,
        reasoning_content: reasoning_content.clone(),
        tool_calls: tool_calls.iter().map(raw_tool_call).collect(),
    }
}

fn role(role: ConversationRole) -> &'static str {
    match role {
        ConversationRole::User => "user",
        ConversationRole::Assistant => "assistant",
        ConversationRole::Tool => "tool",
    }
}
