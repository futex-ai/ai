//! Construction, portable access, and redaction of conversation parts.

use crate::ToolCall;

use super::{part::ConversationPart, state::ProviderState};

impl ConversationPart {
    /// Builds a text part without provider state.
    pub fn text(text: impl Into<String>) -> Self {
        Self::Text {
            text: text.into(),
            provider_state: None,
        }
    }

    /// Builds a reasoning part without provider state.
    pub fn reasoning(text: impl Into<String>) -> Self {
        Self::Reasoning {
            text: text.into(),
            provider_state: None,
        }
    }

    /// Builds an image part without provider state.
    pub fn image(mime_type: impl Into<String>, data_base64: impl Into<String>) -> Self {
        Self::Image {
            mime_type: mime_type.into(),
            data_base64: data_base64.into(),
            provider_state: None,
        }
    }

    /// Builds a video part without provider state.
    pub fn video(mime_type: impl Into<String>, data_base64: impl Into<String>) -> Self {
        Self::Video {
            mime_type: mime_type.into(),
            data_base64: data_base64.into(),
            provider_state: None,
        }
    }

    /// Builds a tool-call part from a portable invocation.
    pub fn from_tool_call(call: ToolCall) -> Self {
        Self::ToolCall {
            call_id: call.id,
            tool_name: call.name,
            arguments: call.input,
            operation_id: call.operation_id,
            provider_state: None,
        }
    }

    /// Builds a tool result without provider state.
    pub fn tool_result(
        content: impl Into<String>,
        tool_name: impl Into<String>,
        call_id: impl Into<String>,
    ) -> Self {
        Self::ToolResult {
            call_id: call_id.into(),
            tool_name: tool_name.into(),
            content: content.into(),
            provider_state: None,
        }
    }

    /// Returns the provider state attached to this part.
    pub fn provider_state(&self) -> Option<&ProviderState> {
        match self {
            Self::Text { provider_state, .. }
            | Self::Image { provider_state, .. }
            | Self::Video { provider_state, .. }
            | Self::ToolCall { provider_state, .. }
            | Self::ToolResult { provider_state, .. }
            | Self::Reasoning { provider_state, .. } => provider_state.as_ref(),
        }
    }

    /// Returns the slot that owns this part's provider state.
    pub fn provider_state_mut(&mut self) -> &mut Option<ProviderState> {
        match self {
            Self::Text { provider_state, .. }
            | Self::Image { provider_state, .. }
            | Self::Video { provider_state, .. }
            | Self::ToolCall { provider_state, .. }
            | Self::ToolResult { provider_state, .. }
            | Self::Reasoning { provider_state, .. } => provider_state,
        }
    }

    /// Attaches the state produced with this part.
    pub fn with_provider_state(mut self, state: ProviderState) -> Self {
        *self.provider_state_mut() = Some(state);
        self
    }

    /// Returns a portable tool call when this is a tool-call part.
    pub fn as_tool_call(&self) -> Option<ToolCall> {
        let Self::ToolCall {
            call_id,
            tool_name,
            arguments,
            operation_id,
            ..
        } = self
        else {
            return None;
        };
        Some(ToolCall {
            id: call_id.clone(),
            name: tool_name.clone(),
            input: arguments.clone(),
            operation_id: operation_id.clone(),
        })
    }

    /// Returns a copy without replay state or reasoning text.
    pub fn log_copy(&self) -> Self {
        let mut part = self.clone();
        *part.provider_state_mut() = None;
        if let Self::Reasoning { text, .. } = &mut part {
            text.clear();
        }
        part
    }
}
