//! Tool output store request and result DTOs.

use ai_interface::ToolOutputId;

use crate::ToolOutputPolicy;

#[derive(Clone, Debug, Eq, PartialEq)]
/// Request to store one serialized tool output.
pub struct ToolOutputWriteRequest {
    /// Model-visible tool name that produced the output.
    pub tool_name: String,
    /// Compact UTF-8 JSON serialization of the tool output.
    pub content: String,
    /// Validated output policy governing the write.
    pub policy: ToolOutputPolicy,
    /// Maximum bytes returned in the first model-visible window.
    pub first_window_length: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
/// Successful stored-output write result.
pub struct ToolOutputWriteResult {
    /// Opaque id assigned to the stored bytes.
    pub output_id: ToolOutputId,
    /// First UTF-8-safe window for the stored bytes.
    pub first_window: ToolOutputStoreWindow,
}

#[derive(Clone, Debug, Eq, PartialEq)]
/// Request to read a UTF-8 byte window from a stored output.
pub struct ToolOutputStoreReadRequest {
    /// Opaque id returned by a prior stored output.
    pub output_id: ToolOutputId,
    /// Optional byte offset; omitted means zero.
    pub offset: Option<usize>,
    /// Optional byte length; omitted means the policy read limit.
    pub length: Option<usize>,
    /// Validated output policy governing the read.
    pub policy: ToolOutputPolicy,
}

#[derive(Clone, Debug, Eq, PartialEq)]
/// UTF-8-safe byte window returned by an output store.
pub struct ToolOutputStoreWindow {
    /// Model-visible tool name that produced the original output.
    pub tool_name: String,
    /// Byte offset represented by this window.
    pub offset: usize,
    /// UTF-8 substring of the serialized tool output.
    pub content: String,
    /// Number of bytes returned in `content`.
    pub returned_bytes: usize,
    /// Total serialized tool output byte count.
    pub total_bytes: usize,
    /// Whether unread bytes remain after this window.
    pub truncated: bool,
    /// Next readable byte offset, when `truncated` is true.
    pub next_offset: Option<usize>,
}
