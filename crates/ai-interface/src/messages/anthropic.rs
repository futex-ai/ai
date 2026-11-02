//! Typed Anthropic assistant blocks retained unchanged for continuation.

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
/// One original Anthropic assistant content block.
pub enum AnthropicAssistantBlock {
    /// Thinking text and the signature that preserves its full reasoning.
    Thinking {
        /// Provider thinking text, including empty text when display is omitted.
        thinking: String,
        /// Opaque provider signature, replayed unchanged.
        signature: String,
    },
    /// Encrypted thinking that has no readable summary.
    RedactedThinking {
        /// Opaque encrypted provider data.
        data: String,
    },
    /// Original assistant text, including whitespace.
    Text {
        /// Provider text.
        text: String,
    },
    /// Original tool use before normalization for runtime dispatch.
    ToolUse {
        /// Provider-issued call identifier.
        id: String,
        /// Provider tool name.
        name: String,
        /// Original tool input.
        input: Value,
    },
}
