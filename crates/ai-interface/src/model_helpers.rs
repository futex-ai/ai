//! Portable response access and safe copies for model observers.

use crate::{
    ConversationMessage, ConversationPart, ConversationRole, ModelRequest, ModelResponse,
    ModelResult, ToolCall,
};

impl ModelRequest {
    /// Checks every message against its role before adapter transport.
    pub fn validate_messages(&self, provider: &str, model_id: &str) -> ModelResult<()> {
        for message in &self.messages {
            message.validate(provider, model_id)?;
        }
        Ok(())
    }

    /// Keeps the request while removing all replay state and reasoning text.
    pub fn log_copy(&self) -> Self {
        Self {
            messages: self
                .messages
                .iter()
                .map(ConversationMessage::log_copy)
                .collect(),
            ..self.clone()
        }
    }
}

impl ModelResponse {
    /// Concatenates assistant text parts in generated order.
    pub fn text(&self) -> String {
        self.parts
            .iter()
            .filter_map(|part| match part {
                ConversationPart::Text { text, .. } => Some(text.as_str()),
                _ => None,
            })
            .collect()
    }

    /// Returns portable tool calls in generated order.
    pub fn tool_calls(&self) -> Vec<ToolCall> {
        self.parts
            .iter()
            .filter_map(ConversationPart::as_tool_call)
            .collect()
    }

    /// Copies every generated part into a retained assistant message.
    pub fn to_assistant_message(&self) -> ConversationMessage {
        ConversationMessage::new(ConversationRole::Assistant, self.parts.clone())
    }

    /// Keeps the response while removing all replay state and reasoning text.
    pub fn log_copy(&self) -> Self {
        Self {
            parts: self.parts.iter().map(ConversationPart::log_copy).collect(),
            ..self.clone()
        }
    }
}
