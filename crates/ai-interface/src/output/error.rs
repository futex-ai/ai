//! Tool output envelope construction errors.

use thiserror::Error;

#[derive(Clone, Debug, Eq, Error, PartialEq)]
/// Errors raised when constructing or decoding tool output envelopes.
pub enum ToolOutputEnvelopeError {
    /// The envelope tag did not match the expected DTO type.
    #[error("[ai_interface/output] expected envelope type `{expected}`")]
    InvalidType {
        /// Expected serde tag value.
        expected: &'static str,
    },
    /// Inline output envelopes must be complete.
    #[error("[ai_interface/output] inline envelope cannot be truncated")]
    InlineTruncated,
    /// A truncated window had both readable and unavailable remainder fields.
    #[error("[ai_interface/output] truncated window has conflicting remainder fields")]
    TruncatedWindowRemainderConflict,
    /// A truncated window had neither readable nor unavailable remainder fields.
    #[error("[ai_interface/output] truncated window is missing its remainder field")]
    TruncatedWindowRemainderMissing,
    /// A complete window carried a remainder field.
    #[error("[ai_interface/output] complete window cannot carry a remainder field")]
    CompleteWindowRemainder,
    /// An unavailable remainder carried an output id.
    #[error("[ai_interface/output] unavailable remainder cannot carry an output id")]
    OutputIdWithUnavailableRemainder,
    /// A readable window did not carry an output id.
    #[error("[ai_interface/output] readable window must carry an output id")]
    MissingOutputId,
}

/// Result alias for tool output envelope construction.
pub type ToolOutputEnvelopeResult<T> = std::result::Result<T, ToolOutputEnvelopeError>;
