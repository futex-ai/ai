//! Text, image, video, and empty-content wire forms.

use ai_interface::{
    ConversationMessage, ConversationPart, ConversationRole, ErrorKind, ModelError, ModelResult,
};

use super::{
    dialect::{Dialect, InputParts, ReplayRule},
    request_types::{ChatCompletionsContent, ChatCompletionsContentPart, ChatCompletionsImageUrl},
};

pub(super) fn message_content(
    dialect: &Dialect,
    model_id: &str,
    message: &ConversationMessage,
) -> ModelResult<Option<ChatCompletionsContent>> {
    if message.role == ConversationRole::User {
        return match message.parts.as_slice() {
            [] => Ok(Some(ChatCompletionsContent::Text(String::new()))),
            [ConversationPart::Text { text, .. }] => {
                Ok(Some(ChatCompletionsContent::Text(text.clone())))
            }
            parts => Ok(Some(ChatCompletionsContent::Parts(
                parts
                    .iter()
                    .map(|part| content_part(dialect, model_id, part))
                    .collect::<ModelResult<_>>()?,
            ))),
        };
    }
    let text = message.text();
    if message.role == ConversationRole::Assistant
        && text.is_empty()
        && (matches!(
            dialect.replay,
            ReplayRule::Xai | ReplayRule::MiniMax | ReplayRule::Kimi
        ) || dialect.replay == ReplayRule::Qwen && !message.tool_calls().is_empty())
    {
        Ok(None)
    } else {
        Ok(Some(ChatCompletionsContent::Text(text)))
    }
}

fn content_part(
    dialect: &Dialect,
    model_id: &str,
    part: &ConversationPart,
) -> ModelResult<ChatCompletionsContentPart> {
    match part {
        ConversationPart::Text { text, .. } => {
            Ok(ChatCompletionsContentPart::Text { text: text.clone() })
        }
        ConversationPart::Image {
            mime_type,
            data_base64,
            ..
        } => Ok(ChatCompletionsContentPart::ImageUrl {
            image_url: ChatCompletionsImageUrl {
                url: format!("data:{mime_type};base64,{data_base64}"),
            },
        }),
        ConversationPart::Video {
            mime_type,
            data_base64,
            ..
        } if matches!(dialect.input, InputParts::TextImageVideo) => {
            Ok(ChatCompletionsContentPart::VideoUrl {
                video_url: ChatCompletionsImageUrl {
                    url: format!("data:{mime_type};base64,{data_base64}"),
                },
            })
        }
        _ => Err(ModelError::failure(
            ErrorKind::UnsupportedInput,
            dialect.provider,
            model_id,
            format!(
                "{} accepts text and image content parts only",
                dialect.label
            ),
        )),
    }
}
