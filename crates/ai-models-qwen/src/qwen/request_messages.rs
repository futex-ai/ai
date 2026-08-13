//! Conversation-to-Qwen chat message mapping.

use ai_interface::{
    ConversationContentPart, ConversationMessage, ConversationRole, ModelError, ModelResult,
    ProviderConversationItem, QwenToolCallContext, ToolCall,
};

use super::request_types::{
    ChatCompletionsContent, ChatCompletionsContentPart, ChatCompletionsImageUrl,
    ChatCompletionsMessage, ChatCompletionsToolCall, ChatCompletionsToolFunction,
};

const PROVIDER: &str = "qwen";

pub(super) fn message(
    model_id: &str,
    message: &ConversationMessage,
) -> ModelResult<ChatCompletionsMessage> {
    if message.role == ConversationRole::Assistant
        && let Some(replay) = qwen_replay(message)
    {
        return Ok(replay_message(replay));
    }
    Ok(ChatCompletionsMessage {
        role: role(message.role).to_owned(),
        content: message_content(model_id, message)?,
        tool_call_id: (message.role == ConversationRole::Tool)
            .then(|| message.tool_call_id.clone())
            .flatten(),
        reasoning_content: None,
        tool_calls: if message.role == ConversationRole::Assistant {
            message.tool_calls.iter().map(tool_call).collect()
        } else {
            Vec::new()
        },
    })
}

fn qwen_replay(message: &ConversationMessage) -> Option<&ProviderConversationItem> {
    message
        .provider_context
        .iter()
        .find(|item| matches!(item, ProviderConversationItem::QwenAssistantMessage { .. }))
}

fn replay_message(item: &ProviderConversationItem) -> ChatCompletionsMessage {
    let ProviderConversationItem::QwenAssistantMessage {
        content,
        reasoning_content,
        tool_calls,
    } = item
    else {
        return empty_assistant_message();
    };
    ChatCompletionsMessage {
        role: "assistant".to_owned(),
        content: match content {
            Some(content) => Some(ChatCompletionsContent::Text(content.clone())),
            None if tool_calls.is_empty() => Some(ChatCompletionsContent::Text(String::new())),
            None => None,
        },
        tool_call_id: None,
        reasoning_content: reasoning_content.clone(),
        tool_calls: tool_calls.iter().map(raw_tool_call).collect(),
    }
}

fn empty_assistant_message() -> ChatCompletionsMessage {
    ChatCompletionsMessage {
        role: "assistant".to_owned(),
        content: None,
        tool_call_id: None,
        reasoning_content: None,
        tool_calls: Vec::new(),
    }
}

fn role(role: ConversationRole) -> &'static str {
    match role {
        ConversationRole::User => "user",
        ConversationRole::Assistant => "assistant",
        ConversationRole::Tool => "tool",
    }
}

fn message_content(
    model_id: &str,
    message: &ConversationMessage,
) -> ModelResult<Option<ChatCompletionsContent>> {
    if !message.content_parts.is_empty() {
        return Ok(Some(ChatCompletionsContent::Parts(
            message
                .content_parts
                .iter()
                .map(|part| content_part(model_id, part))
                .collect::<ModelResult<Vec<_>>>()?,
        )));
    }
    if message.role == ConversationRole::Assistant
        && message.content.is_empty()
        && !message.tool_calls.is_empty()
    {
        Ok(None)
    } else {
        Ok(Some(ChatCompletionsContent::Text(message.content.clone())))
    }
}

fn content_part(
    model_id: &str,
    part: &ConversationContentPart,
) -> ModelResult<ChatCompletionsContentPart> {
    match part {
        ConversationContentPart::Text { text } => {
            Ok(ChatCompletionsContentPart::Text { text: text.clone() })
        }
        ConversationContentPart::Image {
            mime_type,
            data_base64,
        } => Ok(ChatCompletionsContentPart::ImageUrl {
            image_url: ChatCompletionsImageUrl {
                url: format!("data:{mime_type};base64,{data_base64}"),
            },
        }),
        ConversationContentPart::Video { .. } => Err(ModelError::provider(
            PROVIDER,
            model_id,
            "Qwen accepts text and image content parts only",
        )),
    }
}

fn tool_call(call: &ToolCall) -> ChatCompletionsToolCall {
    ChatCompletionsToolCall {
        id: call.id.clone(),
        kind: "function".to_owned(),
        function: ChatCompletionsToolFunction {
            name: call.name.clone(),
            arguments: call.input.to_string(),
        },
    }
}

fn raw_tool_call(call: &QwenToolCallContext) -> ChatCompletionsToolCall {
    ChatCompletionsToolCall {
        id: call.id.clone(),
        kind: "function".to_owned(),
        function: ChatCompletionsToolFunction {
            name: call.name.clone(),
            arguments: call.arguments.clone(),
        },
    }
}
