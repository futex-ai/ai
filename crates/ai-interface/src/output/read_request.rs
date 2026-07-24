//! Intrinsic tool output read request DTO.

use serde::{Deserialize, Serialize};

use super::ToolOutputId;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
/// Arguments accepted by the intrinsic `tool_output_read` tool.
pub struct ToolOutputReadRequest {
    /// Opaque id returned by a prior windowed tool output.
    pub output_id: ToolOutputId,
    /// Optional byte offset to read from; omitted means zero.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub offset: Option<usize>,
    /// Optional byte length to read; omitted means the runtime read default.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub length: Option<usize>,
}
