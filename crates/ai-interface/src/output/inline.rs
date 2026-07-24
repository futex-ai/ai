//! Inline tool output envelope DTO.

use serde::de::Error as DeError;
use serde::ser::SerializeStruct;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value;

use super::{INLINE_TYPE, ToolOutputEnvelopeError, ToolOutputEnvelopeResult};

#[derive(Clone, Debug, PartialEq)]
/// Complete model-visible envelope for an inline-sized successful tool output.
pub struct ToolOutputInlineEnvelope {
    tool_name: String,
    output: Value,
    total_bytes: usize,
}

impl ToolOutputInlineEnvelope {
    /// Builds a complete inline tool output envelope.
    pub fn new(tool_name: impl Into<String>, output: Value, total_bytes: usize) -> Self {
        Self {
            tool_name: tool_name.into(),
            output,
            total_bytes,
        }
    }

    /// Returns the tool name that produced the output.
    pub fn tool_name(&self) -> &str {
        &self.tool_name
    }

    /// Returns the complete raw JSON output.
    pub fn output(&self) -> &Value {
        &self.output
    }

    /// Returns the compact serialized output byte count.
    pub fn total_bytes(&self) -> usize {
        self.total_bytes
    }

    /// Returns whether the inline output is truncated.
    pub fn truncated(&self) -> bool {
        false
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct InlineWire {
    #[serde(rename = "type")]
    kind: String,
    tool_name: String,
    output: Value,
    total_bytes: usize,
    truncated: bool,
}

impl TryFrom<InlineWire> for ToolOutputInlineEnvelope {
    type Error = ToolOutputEnvelopeError;

    fn try_from(value: InlineWire) -> ToolOutputEnvelopeResult<Self> {
        if value.kind != INLINE_TYPE {
            return Err(ToolOutputEnvelopeError::InvalidType {
                expected: INLINE_TYPE,
            });
        }
        if value.truncated {
            return Err(ToolOutputEnvelopeError::InlineTruncated);
        }
        Ok(Self::new(value.tool_name, value.output, value.total_bytes))
    }
}

impl Serialize for ToolOutputInlineEnvelope {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("ToolOutputInlineEnvelope", 5)?;
        state.serialize_field("type", INLINE_TYPE)?;
        state.serialize_field("tool_name", &self.tool_name)?;
        state.serialize_field("output", &self.output)?;
        state.serialize_field("total_bytes", &self.total_bytes)?;
        state.serialize_field("truncated", &false)?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for ToolOutputInlineEnvelope {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = InlineWire::deserialize(deserializer)?;
        match Self::try_from(wire) {
            Ok(envelope) => Ok(envelope),
            Err(error) => Err(D::Error::custom(error)),
        }
    }
}
