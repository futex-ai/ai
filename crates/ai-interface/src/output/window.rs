//! Windowed tool output envelope DTO.

use serde::de::Error as DeError;
use serde::ser::SerializeStruct;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use super::{
    ToolOutputEnvelopeError, ToolOutputEnvelopeResult, ToolOutputId,
    ToolOutputRemainderUnavailableReason, WINDOW_TYPE,
};

#[derive(Clone, Debug, PartialEq)]
/// Model-visible UTF-8 byte window for a stored or degraded tool output.
pub struct ToolOutputWindowEnvelope {
    output_id: Option<ToolOutputId>,
    tool_name: String,
    offset: usize,
    content: String,
    returned_bytes: usize,
    total_bytes: usize,
    truncated: bool,
    next_offset: Option<usize>,
    remainder_unavailable: Option<ToolOutputRemainderUnavailableReason>,
}

impl ToolOutputWindowEnvelope {
    /// Builds a readable window for a stored output.
    pub fn readable(
        tool_name: impl Into<String>,
        output_id: ToolOutputId,
        offset: usize,
        content: impl Into<String>,
        returned_bytes: usize,
        total_bytes: usize,
        next_offset: Option<usize>,
    ) -> ToolOutputEnvelopeResult<Self> {
        let envelope = Self {
            output_id: Some(output_id),
            tool_name: tool_name.into(),
            offset,
            content: content.into(),
            returned_bytes,
            total_bytes,
            truncated: next_offset.is_some(),
            next_offset,
            remainder_unavailable: None,
        };
        envelope.validate()?;
        Ok(envelope)
    }

    /// Builds a degraded first window whose unread remainder cannot be fetched.
    pub fn degraded(
        tool_name: impl Into<String>,
        content: impl Into<String>,
        returned_bytes: usize,
        total_bytes: usize,
        reason: ToolOutputRemainderUnavailableReason,
    ) -> Self {
        Self {
            output_id: None,
            tool_name: tool_name.into(),
            offset: 0,
            content: content.into(),
            returned_bytes,
            total_bytes,
            truncated: true,
            next_offset: None,
            remainder_unavailable: Some(reason),
        }
    }

    /// Returns the readable output id, when the remainder is available.
    pub fn output_id(&self) -> Option<&ToolOutputId> {
        self.output_id.as_ref()
    }

    /// Returns the tool name that produced the original output.
    pub fn tool_name(&self) -> &str {
        &self.tool_name
    }

    /// Returns the byte offset represented by this window.
    pub fn offset(&self) -> usize {
        self.offset
    }

    /// Returns the UTF-8 content window.
    pub fn content(&self) -> &str {
        &self.content
    }

    /// Returns the number of bytes included in `content`.
    pub fn returned_bytes(&self) -> usize {
        self.returned_bytes
    }

    /// Returns the total serialized output byte count.
    pub fn total_bytes(&self) -> usize {
        self.total_bytes
    }

    /// Returns whether bytes remain after this window.
    pub fn truncated(&self) -> bool {
        self.truncated
    }

    /// Returns the next readable byte offset, when more bytes are available.
    pub fn next_offset(&self) -> Option<usize> {
        self.next_offset
    }

    /// Returns why the unread remainder cannot be fetched.
    pub fn remainder_unavailable(&self) -> Option<&ToolOutputRemainderUnavailableReason> {
        self.remainder_unavailable.as_ref()
    }

    fn validate(&self) -> ToolOutputEnvelopeResult<()> {
        if self.remainder_unavailable.is_some() && self.output_id.is_some() {
            return Err(ToolOutputEnvelopeError::OutputIdWithUnavailableRemainder);
        }
        if self.remainder_unavailable.is_none() && self.output_id.is_none() {
            return Err(ToolOutputEnvelopeError::MissingOutputId);
        }
        match (
            self.truncated,
            self.next_offset.is_some(),
            self.remainder_unavailable.is_some(),
        ) {
            (true, true, true) => Err(ToolOutputEnvelopeError::TruncatedWindowRemainderConflict),
            (true, false, false) => Err(ToolOutputEnvelopeError::TruncatedWindowRemainderMissing),
            (false, true, _) | (false, _, true) => {
                Err(ToolOutputEnvelopeError::CompleteWindowRemainder)
            }
            _ => Ok(()),
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WindowWire {
    #[serde(rename = "type")]
    kind: String,
    #[serde(default)]
    output_id: Option<ToolOutputId>,
    tool_name: String,
    offset: usize,
    content: String,
    returned_bytes: usize,
    total_bytes: usize,
    truncated: bool,
    #[serde(default)]
    next_offset: Option<usize>,
    #[serde(default)]
    remainder_unavailable: Option<ToolOutputRemainderUnavailableReason>,
}

impl TryFrom<WindowWire> for ToolOutputWindowEnvelope {
    type Error = ToolOutputEnvelopeError;

    fn try_from(value: WindowWire) -> ToolOutputEnvelopeResult<Self> {
        if value.kind != WINDOW_TYPE {
            return Err(ToolOutputEnvelopeError::InvalidType {
                expected: WINDOW_TYPE,
            });
        }
        let envelope = Self {
            output_id: value.output_id,
            tool_name: value.tool_name,
            offset: value.offset,
            content: value.content,
            returned_bytes: value.returned_bytes,
            total_bytes: value.total_bytes,
            truncated: value.truncated,
            next_offset: value.next_offset,
            remainder_unavailable: value.remainder_unavailable,
        };
        envelope.validate()?;
        Ok(envelope)
    }
}

impl Serialize for ToolOutputWindowEnvelope {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut field_count = 7;
        if self.output_id.is_some() {
            field_count += 1;
        }
        if self.next_offset.is_some() {
            field_count += 1;
        }
        if self.remainder_unavailable.is_some() {
            field_count += 1;
        }
        let mut state = serializer.serialize_struct("ToolOutputWindowEnvelope", field_count)?;
        state.serialize_field("type", WINDOW_TYPE)?;
        if let Some(output_id) = &self.output_id {
            state.serialize_field("output_id", output_id)?;
        }
        state.serialize_field("tool_name", &self.tool_name)?;
        state.serialize_field("offset", &self.offset)?;
        state.serialize_field("content", &self.content)?;
        state.serialize_field("returned_bytes", &self.returned_bytes)?;
        state.serialize_field("total_bytes", &self.total_bytes)?;
        state.serialize_field("truncated", &self.truncated)?;
        if let Some(next_offset) = self.next_offset {
            state.serialize_field("next_offset", &next_offset)?;
        }
        if let Some(reason) = &self.remainder_unavailable {
            state.serialize_field("remainder_unavailable", reason)?;
        }
        state.end()
    }
}

impl<'de> Deserialize<'de> for ToolOutputWindowEnvelope {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = WindowWire::deserialize(deserializer)?;
        match Self::try_from(wire) {
            Ok(envelope) => Ok(envelope),
            Err(error) => Err(D::Error::custom(error)),
        }
    }
}
