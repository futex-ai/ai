//! Tagged tool output envelope enum.

use serde::de::Error as DeError;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value;

use super::{
    INLINE_TYPE, ToolOutputEnvelopeResult, ToolOutputId, ToolOutputInlineEnvelope,
    ToolOutputRemainderUnavailableReason, ToolOutputWindowEnvelope, WINDOW_TYPE,
};

#[derive(Clone, Debug, PartialEq)]
/// Model-visible envelope appended for a successful tool output.
pub enum ToolOutputEnvelope {
    /// Complete inline-sized tool output.
    Inline(ToolOutputInlineEnvelope),
    /// UTF-8 byte window for a larger or degraded tool output.
    Window(ToolOutputWindowEnvelope),
}

impl ToolOutputEnvelope {
    /// Builds a complete inline tool output envelope.
    pub fn inline(tool_name: impl Into<String>, output: Value, total_bytes: usize) -> Self {
        Self::Inline(ToolOutputInlineEnvelope::new(
            tool_name,
            output,
            total_bytes,
        ))
    }

    /// Builds a readable window envelope for a stored output.
    pub fn readable_window(
        tool_name: impl Into<String>,
        output_id: ToolOutputId,
        offset: usize,
        content: impl Into<String>,
        returned_bytes: usize,
        total_bytes: usize,
        next_offset: Option<usize>,
    ) -> ToolOutputEnvelopeResult<Self> {
        Ok(Self::Window(ToolOutputWindowEnvelope::readable(
            tool_name,
            output_id,
            offset,
            content,
            returned_bytes,
            total_bytes,
            next_offset,
        )?))
    }

    /// Builds a degraded first-window envelope for an unstored output.
    pub fn degraded_window(
        tool_name: impl Into<String>,
        content: impl Into<String>,
        returned_bytes: usize,
        total_bytes: usize,
        reason: ToolOutputRemainderUnavailableReason,
    ) -> Self {
        Self::Window(ToolOutputWindowEnvelope::degraded(
            tool_name,
            content,
            returned_bytes,
            total_bytes,
            reason,
        ))
    }

    /// Returns the output id when the envelope references stored bytes.
    pub fn output_id(&self) -> Option<&ToolOutputId> {
        match self {
            Self::Inline(_) => None,
            Self::Window(window) => window.output_id(),
        }
    }
}

impl Serialize for ToolOutputEnvelope {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Self::Inline(envelope) => envelope.serialize(serializer),
            Self::Window(envelope) => envelope.serialize(serializer),
        }
    }
}

impl<'de> Deserialize<'de> for ToolOutputEnvelope {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = Value::deserialize(deserializer)?;
        let kind = match value.get("type").and_then(Value::as_str) {
            Some(kind) => kind,
            None => return Err(D::Error::custom("missing tool output envelope type")),
        };
        match kind {
            INLINE_TYPE => match serde_json::from_value::<ToolOutputInlineEnvelope>(value) {
                Ok(envelope) => Ok(Self::Inline(envelope)),
                Err(error) => Err(D::Error::custom(error)),
            },
            WINDOW_TYPE => match serde_json::from_value::<ToolOutputWindowEnvelope>(value) {
                Ok(envelope) => Ok(Self::Window(envelope)),
                Err(error) => Err(D::Error::custom(error)),
            },
            _ => Err(D::Error::custom("unknown tool output envelope type")),
        }
    }
}
