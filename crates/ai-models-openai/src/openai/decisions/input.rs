//! Decisions wire forms for judgment content and portable conversations.

use std::borrow::Cow;

use ai_interface::{
    ConversationMessage, ConversationPart, ConversationRole, JudgmentContent, JudgmentInput,
    JudgmentJson, JudgmentPart,
};
use serde::Serialize;
use serde_json::{Result, Value};

#[derive(Serialize)]
#[serde(untagged)]
pub(super) enum TextOrArray<'a, T> {
    Text(&'a str),
    Array(Vec<T>),
}

#[derive(Serialize)]
#[serde(untagged)]
pub(super) enum InputItem<'a> {
    Message(Message<'a>),
    Function(FunctionItem<'a>),
}

#[derive(Serialize)]
#[serde(tag = "role", rename_all = "snake_case")]
pub(super) enum Message<'a> {
    Developer { content: &'a str },
    User { content: Vec<InputPart<'a>> },
    Assistant { content: &'a str },
}

#[derive(Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub(super) enum FunctionItem<'a> {
    FunctionCall {
        call_id: &'a str,
        name: &'a str,
        arguments: String,
    },
    FunctionCallOutput {
        call_id: &'a str,
        output: &'a str,
    },
}

#[derive(Serialize)]
#[serde(tag = "type")]
pub(super) enum InputPart<'a> {
    #[serde(rename = "input_text")]
    Text { text: Cow<'a, str> },
    #[serde(rename = "input_image")]
    Image { image_url: String },
    #[serde(rename = "input_json")]
    Json { data: &'a JudgmentJson },
}

pub(super) fn input(input: &JudgmentInput) -> Result<TextOrArray<'_, InputItem<'_>>> {
    match input {
        JudgmentInput::Content(content) => match content.parts.as_slice() {
            [JudgmentPart::Text { text }] => Ok(TextOrArray::Text(text)),
            _ => Ok(TextOrArray::Array(vec![InputItem::Message(
                Message::User {
                    content: content_parts(content)?,
                },
            )])),
        },
        JudgmentInput::Conversation { system, messages } => Ok(TextOrArray::Array(conversation(
            system.as_deref(),
            messages,
        ))),
    }
}

pub(super) fn instructions(content: &JudgmentContent) -> Result<TextOrArray<'_, InputPart<'_>>> {
    match content.parts.as_slice() {
        [JudgmentPart::Text { text }] => Ok(TextOrArray::Text(text)),
        _ => Ok(TextOrArray::Array(content_parts(content)?)),
    }
}

fn content_parts(content: &JudgmentContent) -> Result<Vec<InputPart<'_>>> {
    content
        .parts
        .iter()
        .map(|part| {
            Ok(match part {
                JudgmentPart::Text { text } => InputPart::Text {
                    text: Cow::Borrowed(text),
                },
                JudgmentPart::Image {
                    mime_type,
                    data_base64,
                } => image(mime_type, data_base64),
                JudgmentPart::Json {
                    value: JudgmentJson::Array(values),
                } if !values.iter().all(Value::is_object) => InputPart::Text {
                    text: Cow::Owned(serde_json::to_string(values)?),
                },
                JudgmentPart::Json { value } => InputPart::Json { data: value },
            })
        })
        .collect()
}

fn conversation<'a>(
    system: Option<&'a str>,
    messages: &'a [ConversationMessage],
) -> Vec<InputItem<'a>> {
    let mut items = Vec::new();
    if let Some(content) = system.filter(|text| !text.trim().is_empty()) {
        items.push(InputItem::Message(Message::Developer { content }));
    }
    for message in messages {
        match message.role {
            ConversationRole::User => items.push(InputItem::Message(Message::User {
                content: message.parts.iter().filter_map(user_part).collect(),
            })),
            ConversationRole::Assistant => {
                items.extend(message.parts.iter().filter_map(assistant_part))
            }
            ConversationRole::Tool => {
                if let [
                    ConversationPart::ToolResult {
                        call_id, content, ..
                    },
                ] = message.parts.as_slice()
                {
                    items.push(InputItem::Function(FunctionItem::FunctionCallOutput {
                        call_id,
                        output: content,
                    }));
                }
            }
        }
    }
    items
}

fn user_part(part: &ConversationPart) -> Option<InputPart<'_>> {
    match part {
        ConversationPart::Text { text, .. } => Some(InputPart::Text {
            text: Cow::Borrowed(text),
        }),
        ConversationPart::Image {
            mime_type,
            data_base64,
            ..
        } => Some(image(mime_type, data_base64)),
        _ => None,
    }
}

fn assistant_part(part: &ConversationPart) -> Option<InputItem<'_>> {
    match part {
        ConversationPart::Text { text, .. } => {
            Some(InputItem::Message(Message::Assistant { content: text }))
        }
        ConversationPart::ToolCall {
            call_id,
            tool_name,
            arguments,
            ..
        } => Some(InputItem::Function(FunctionItem::FunctionCall {
            call_id,
            name: tool_name,
            arguments: arguments.to_string(),
        })),
        _ => None,
    }
}

fn image<'a>(mime_type: &str, data_base64: &str) -> InputPart<'a> {
    InputPart::Image {
        image_url: format!("data:{mime_type};base64,{data_base64}"),
    }
}
