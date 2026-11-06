//! Portable user content mapped to Responses text and image inputs.

use ai_interface::{ConversationPart, ErrorKind, ModelResult};
use ai_models_core::ErrorContext;

use super::request_types::{ResponsesContentPart, ResponsesMessageContent};

pub(super) fn message_content(
    context: &ErrorContext,
    parts: &[ConversationPart],
) -> ModelResult<ResponsesMessageContent> {
    match parts {
        [] => return Ok(ResponsesMessageContent::Text(String::new())),
        [ConversationPart::Text { text, .. }] => {
            return Ok(ResponsesMessageContent::Text(text.clone()));
        }
        _ => {}
    }
    let mut content = Vec::new();
    for part in parts {
        content.push(match part {
            ConversationPart::Text { text, .. } => {
                ResponsesContentPart::InputText { text: text.clone() }
            }
            ConversationPart::Image {
                mime_type,
                data_base64,
                ..
            } => ResponsesContentPart::InputImage {
                image_url: format!("data:{mime_type};base64,{data_base64}"),
            },
            _ => {
                return Err(context.model_error(
                    ErrorKind::UnsupportedInput,
                    "OpenAI accepts text and image content parts only",
                ));
            }
        });
    }
    Ok(ResponsesMessageContent::Parts(content))
}
