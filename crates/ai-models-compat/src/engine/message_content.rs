//! Text, image, video, and empty-content wire forms.

use super::{
    dialect::{Dialect, InputParts, ReplayRule},
    request_types::{ChatCompletionsContent, ChatCompletionsContentPart, ChatCompletionsImageUrl},
};
use ai_interface::{
    ConversationContentPart, ConversationMessage, ConversationRole, ModelError, ModelResult,
};

pub(super) fn message_content(
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
    if dialect.replay == ReplayRule::Xai && message.content.is_empty() {
        return Ok(None);
    }
    if dialect.replay == ReplayRule::MiniMax
        && message.role != ConversationRole::Tool
        && message.content.is_empty()
    {
        return Ok(None);
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
        ConversationContentPart::Video {
            mime_type,
            data_base64,
        } if matches!(dialect.input, InputParts::TextImageVideo) => {
            Ok(ChatCompletionsContentPart::VideoUrl {
                video_url: ChatCompletionsImageUrl {
                    url: format!("data:{mime_type};base64,{data_base64}"),
                },
            })
        }
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
