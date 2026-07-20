//! Stored tool output identifier.

use std::fmt;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
/// Opaque identifier for a stored tool output.
pub struct ToolOutputId(String);

impl ToolOutputId {
    /// Builds an opaque output id from a caller-provided string.
    pub fn from_opaque(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// Returns the opaque output id string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ToolOutputId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}
