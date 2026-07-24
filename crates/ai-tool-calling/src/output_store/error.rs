//! Tool output store errors.

use std::error::Error as StdError;

use ai_interface::ToolOutputId;
use thiserror::Error;

use super::ToolOutputStoreWindow;

#[derive(Debug, Error)]
/// Errors returned by the tool output store boundary.
pub enum ToolOutputStoreError {
    /// The serialized output exceeded the configured per-output limit.
    #[error(
        "[ai_tool_calling/output_store] output size {requested_bytes} exceeds per-output limit {limit_bytes}"
    )]
    PerOutputOverflow {
        /// Serialized output byte count.
        requested_bytes: usize,
        /// Configured per-output byte limit.
        limit_bytes: usize,
        /// Store-computed degraded first window.
        window: ToolOutputStoreWindow,
    },
    /// The write would exceed the configured aggregate storage budget.
    #[error(
        "[ai_tool_calling/output_store] output size {requested_bytes} exceeds remaining aggregate budget {available_bytes} of {limit_bytes}"
    )]
    AggregateExhausted {
        /// Serialized output byte count.
        requested_bytes: usize,
        /// Remaining aggregate byte budget at rejection time.
        available_bytes: usize,
        /// Configured aggregate byte limit.
        limit_bytes: usize,
        /// Store-computed degraded first window.
        window: ToolOutputStoreWindow,
    },
    /// The store failed after reserving aggregate budget.
    #[error("[ai_tool_calling/output_store] output store write failed: {source}")]
    WriteFailure {
        /// Store-computed degraded first window.
        window: ToolOutputStoreWindow,
        /// Underlying storage failure.
        source: Box<dyn StdError + Send + Sync>,
    },
    /// The requested output id is not available in this store scope.
    #[error(
        "[ai_tool_calling/output_store] output is no longer available; the original tool call itself succeeded. Re-run the original tool only if it is read-only or otherwise safe to repeat, and confirm with the user before repeating a side-effecting call"
    )]
    UnavailableOutput {
        /// Opaque id that could not be resolved.
        output_id: ToolOutputId,
    },
    /// The requested offset is beyond the output or not a UTF-8 boundary.
    #[error(
        "[ai_tool_calling/output_store] invalid offset {offset} for output of {total_bytes} bytes"
    )]
    InvalidOffset {
        /// Opaque id being read.
        output_id: ToolOutputId,
        /// Requested byte offset.
        offset: usize,
        /// Total serialized output byte count.
        total_bytes: usize,
    },
    /// The requested read length was invalid.
    #[error("[ai_tool_calling/output_store] invalid read length {length}; minimum is 1")]
    InvalidLength {
        /// Requested byte length.
        length: usize,
    },
    /// No full UTF-8 character fits in the requested length.
    #[error(
        "[ai_tool_calling/output_store] no complete UTF-8 character fits in length {length}; minimum usable length is {minimum_usable_length}"
    )]
    NoCompleteCharacterFits {
        /// Requested byte offset.
        offset: usize,
        /// Effective requested byte length.
        length: usize,
        /// Minimum length that would fit the next complete character.
        minimum_usable_length: usize,
    },
}

impl ToolOutputStoreError {
    /// Builds a write-failure error while preserving the degraded first window.
    pub fn write_failure(
        window: ToolOutputStoreWindow,
        source: impl StdError + Send + Sync + 'static,
    ) -> Self {
        Self::WriteFailure {
            window,
            source: Box::new(source),
        }
    }
}

/// Result alias for tool output store operations.
pub type ToolOutputStoreResult<T> = std::result::Result<T, ToolOutputStoreError>;
