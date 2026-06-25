//! Conversation message DTOs.

use serde::{Deserialize, Serialize};

use crate::ToolCall;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
/// Model-visible role assigned to a conversation message.
pub enum ConversationRole {
    /// Human or synthetic caller input provided to the model.
    User,
    /// Assistant text produced by the model.
    Assistant,
    /// Tool output injected back into the conversation.
    Tool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// One message in the retained conversation state.
pub struct ConversationMessage {
    /// Actor role associated with this message.
    pub role: ConversationRole,
    /// Plain-text message content.
    pub content: String,
    #[serde(default)]
    /// Typed multimodal content parts. Empty means use `content` as plain text.
    pub content_parts: Vec<ConversationContentPart>,
    #[serde(default)]
    /// Optional participant or tool name.
    pub name: Option<String>,
    #[serde(default)]
    /// Optional provider-generated tool call identifier.
    pub tool_call_id: Option<String>,
    #[serde(default)]
    /// Optional tool calls attached to an assistant message.
    pub tool_calls: Vec<ToolCall>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
/// Typed model-visible content part.
pub enum ConversationContentPart {
    /// Plain text content.
    Text {
        /// Text body.
        text: String,
    },
    /// Image bytes encoded as base64.
    Image {
        /// Image MIME content type.
        mime_type: String,
        /// Base64-encoded image bytes.
        data_base64: String,
    },
}

impl ConversationMessage {
    /// Builds a caller/user message.
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: ConversationRole::User,
            content: content.into(),
            content_parts: Vec::new(),
            name: None,
            tool_call_id: None,
            tool_calls: Vec::new(),
        }
    }

    /// Builds an assistant message with optional tool calls.
    pub fn assistant(content: impl Into<String>, tool_calls: Vec<ToolCall>) -> Self {
        Self {
            role: ConversationRole::Assistant,
            content: content.into(),
            content_parts: Vec::new(),
            name: None,
            tool_call_id: None,
            tool_calls,
        }
    }

    /// Builds a tool message.
    pub fn tool(
        content: impl Into<String>,
        name: impl Into<String>,
        tool_call_id: impl Into<String>,
    ) -> Self {
        Self {
            role: ConversationRole::Tool,
            content: content.into(),
            content_parts: Vec::new(),
            name: Some(name.into()),
            tool_call_id: Some(tool_call_id.into()),
            tool_calls: Vec::new(),
        }
    }

    /// Builds a caller/user message from typed content parts and a text fallback.
    pub fn user_with_parts(
        content: impl Into<String>,
        content_parts: Vec<ConversationContentPart>,
    ) -> Self {
        Self {
            role: ConversationRole::User,
            content: content.into(),
            content_parts,
            name: None,
            tool_call_id: None,
            tool_calls: Vec::new(),
        }
    }
}
