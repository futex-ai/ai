//! Ordered portable conversation parts and their provider state.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::state::ProviderState;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
/// One ordered part of a model conversation.
pub enum ConversationPart {
    /// Portable text content.
    Text {
        /// Exact text content.
        text: String,
        /// Replay data produced with this part.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        provider_state: Option<ProviderState>,
    },
    /// Base64-encoded image content.
    Image {
        /// Image MIME type, validated by the adapter.
        mime_type: String,
        /// Base64-encoded image bytes.
        data_base64: String,
        /// Replay data produced with this part.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        provider_state: Option<ProviderState>,
    },
    /// Base64-encoded video content.
    Video {
        /// Video MIME type, validated by the adapter.
        mime_type: String,
        /// Base64-encoded video bytes.
        data_base64: String,
        /// Replay data produced with this part.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        provider_state: Option<ProviderState>,
    },
    /// One assistant request to call a tool.
    ToolCall {
        /// Identifier used by the matching tool result.
        call_id: String,
        /// Name of the requested tool.
        tool_name: String,
        /// One JSON argument value.
        arguments: Value,
        /// Idempotency key assigned by the agent loop.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        operation_id: Option<String>,
        /// Replay data produced with this part.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        provider_state: Option<ProviderState>,
    },
    /// One text result that answers a tool call.
    ToolResult {
        /// Identifier of the answered tool call.
        call_id: String,
        /// Name of the tool that produced the result.
        tool_name: String,
        /// Model-visible text of the result.
        content: String,
        /// Replay data produced with this part.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        provider_state: Option<ProviderState>,
    },
    /// Reasoning exposed by the provider, including state-only parts.
    Reasoning {
        /// Reasoning text, empty when only replay data is available.
        text: String,
        /// Replay data produced with this part.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        provider_state: Option<ProviderState>,
    },
}
