//! Conversation content, participant names, and role mapping.

use ai_interface::{
    ConversationContentPart, ConversationMessage, ConversationRole, ModelError, ModelRequest,
    ModelResult,
};

use super::{
    dialect::{Dialect, InputParts, ReplayRule},
    request_replay::replay_message,
    request_types::{
        ChatCompletionsContent, ChatCompletionsContentPart, ChatCompletionsImageUrl,
        ChatCompletionsMessage,
    },
    tools::tool_call,
};

pub(super) fn system_message(content: String) -> ChatCompletionsMessage {
    ChatCompletionsMessage {
        role: "system",
        content: Some(ChatCompletionsContent::Text(content)),
        name: None,
        tool_call_id: None,
        reasoning_content: None,
        tool_calls: Vec::new(),
    }
}

pub(super) fn validate_content_parts(
    dialect: &Dialect,
    model_id: &str,
    request: &ModelRequest,
) -> ModelResult<()> {
    if matches!(dialect.input, InputParts::PlainText)
        && request
            .messages
            .iter()
            .any(|message| !message.content_parts.is_empty())
    {
        return Err(ModelError::provider(
            dialect.provider,
            model_id,
            format!("{} accepts plain text messages only", dialect.label),
        ));
    }
    if let InputParts::UserTextImage { plain_text_model } = dialect.input {
        for message in &request.messages {
            if message.content_parts.is_empty() {
                continue;
            }
            if model_id == plain_text_model {
                return Err(ModelError::provider(
                    dialect.provider,
                    model_id,
                    format!("{model_id} accepts plain text messages only"),
                ));
            }
            if message.role != ConversationRole::User {
                return Err(ModelError::provider(
                    dialect.provider,
                    model_id,
                    format!(
                        "{} accepts typed content parts only on user messages",
                        dialect.label
                    ),
                ));
            }
        }
    }
    Ok(())
}

pub(super) fn message(
    dialect: &Dialect,
    model_id: &str,
    message: &ConversationMessage,
) -> ModelResult<ChatCompletionsMessage> {
    if message.role == ConversationRole::Assistant
        && let Some(replay) = replay_message(dialect.replay, message)
    {
        return Ok(replay);
    }
    Ok(ChatCompletionsMessage {
        role: match message.role {
            ConversationRole::User => "user",
            ConversationRole::Assistant => "assistant",
            ConversationRole::Tool => "tool",
        },
        content: message_content(dialect, model_id, message)?,
        name: if dialect.names && message.role != ConversationRole::Tool {
            message.name.clone()
        } else {
            None
        },
        tool_call_id: if message.role == ConversationRole::Tool
            || dialect.replay == ReplayRule::Kimi
        {
            message.tool_call_id.clone()
        } else {
            None
        },
        reasoning_content: None,
        tool_calls: if message.role == ConversationRole::Assistant {
            message.tool_calls.iter().map(tool_call).collect()
        } else {
            Vec::new()
        },
    })
}

fn message_content(
    dialect: &Dialect,
    model_id: &str,
    message: &ConversationMessage,
) -> ModelResult<Option<ChatCompletionsContent>> {
    if !message.content_parts.is_empty() {
        return Ok(Some(ChatCompletionsContent::Parts(
            message
                .content_parts
                .iter()
                .map(|part| content_part(dialect, model_id, part))
                .collect::<ModelResult<Vec<_>>>()?,
        )));
    }
    if message.role == ConversationRole::Assistant
        && message.content.is_empty()
        && (dialect.replay == ReplayRule::Kimi
            || dialect.replay == ReplayRule::Qwen && !message.tool_calls.is_empty())
    {
        Ok(None)
    } else {
        Ok(Some(ChatCompletionsContent::Text(message.content.clone())))
    }
}

fn content_part(
    dialect: &Dialect,
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
            dialect.provider,
            model_id,
            format!(
                "{} accepts text and image content parts only",
                dialect.label
            ),
        )),
    }
}
