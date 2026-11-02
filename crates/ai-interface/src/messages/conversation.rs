//! Conversation roles, ordered messages, and portable access.

use serde::{Deserialize, Serialize};

use crate::{ModelError, ModelResult, ToolCall};

use super::part::ConversationPart;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
/// Model-visible actor of a conversation message.
pub enum ConversationRole {
    /// Caller input containing text, images, or videos.
    User,
    /// Generated text, reasoning, or tool calls.
    Assistant,
    /// Exactly one tool result.
    Tool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
/// One retained message with ordered parts and an optional participant name.
pub struct ConversationMessage {
    /// Actor that produced this message.
    pub role: ConversationRole,
    /// Parts in their original order.
    pub parts: Vec<ConversationPart>,
    /// Optional name of the user or assistant participant.
    pub name: Option<String>,
}

impl ConversationMessage {
    /// Builds a message from its role and ordered parts.
    pub fn new(role: ConversationRole, parts: Vec<ConversationPart>) -> Self {
        Self {
            role,
            parts,
            name: None,
        }
    }

    /// Builds a caller message with one text part.
    pub fn user(text: impl Into<String>) -> Self {
        Self::new(ConversationRole::User, vec![ConversationPart::text(text)])
    }

    /// Builds a caller message from ordered media and text parts.
    pub fn user_with_parts(parts: Vec<ConversationPart>) -> Self {
        Self::new(ConversationRole::User, parts)
    }

    /// Builds assistant text followed by its requested tool calls.
    pub fn assistant(text: impl Into<String>, tool_calls: Vec<ToolCall>) -> Self {
        let text = text.into();
        let mut parts = Vec::new();
        if !text.is_empty() {
            parts.push(ConversationPart::text(text));
        }
        parts.extend(tool_calls.into_iter().map(ConversationPart::from_tool_call));
        Self::new(ConversationRole::Assistant, parts)
    }

    /// Builds the one result part of a tool message.
    pub fn tool(
        content: impl Into<String>,
        name: impl Into<String>,
        call_id: impl Into<String>,
    ) -> Self {
        Self::new(
            ConversationRole::Tool,
            vec![ConversationPart::tool_result(content, name, call_id)],
        )
    }

    /// Concatenates portable text and tool-result content in part order.
    pub fn text(&self) -> String {
        self.parts
            .iter()
            .filter_map(|part| match part {
                ConversationPart::Text { text, .. } => Some(text.as_str()),
                ConversationPart::ToolResult { content, .. } => Some(content.as_str()),
                _ => None,
            })
            .collect()
    }

    /// Returns portable tool calls in part order.
    pub fn tool_calls(&self) -> Vec<ToolCall> {
        self.parts
            .iter()
            .filter_map(ConversationPart::as_tool_call)
            .collect()
    }

    /// Checks the role table before an adapter opens transport.
    pub fn validate(&self, provider: &str, model_id: &str) -> ModelResult<()> {
        let valid = match self.role {
            ConversationRole::User => self.parts.iter().all(|part| {
                matches!(
                    part,
                    ConversationPart::Text { .. }
                        | ConversationPart::Image { .. }
                        | ConversationPart::Video { .. }
                )
            }),
            ConversationRole::Assistant => self.parts.iter().all(|part| {
                matches!(
                    part,
                    ConversationPart::Text { .. }
                        | ConversationPart::Reasoning { .. }
                        | ConversationPart::ToolCall { .. }
                )
            }),
            ConversationRole::Tool => {
                matches!(self.parts.as_slice(), [ConversationPart::ToolResult { .. }])
            }
        };
        if valid {
            Ok(())
        } else {
            Err(ModelError::provider(
                provider,
                model_id,
                "conversation parts do not match the message role",
            ))
        }
    }

    /// Keeps each part while removing replay state and reasoning text.
    pub fn log_copy(&self) -> Self {
        Self {
            role: self.role,
            parts: self.parts.iter().map(ConversationPart::log_copy).collect(),
            name: self.name.clone(),
        }
    }
}
