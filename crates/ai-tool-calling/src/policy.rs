//! Tool output windowing and storage policy.

use thiserror::Error;

/// Default maximum inline model-visible output bytes.
pub const DEFAULT_INLINE_LIMIT_BYTES: usize = 20_000;
/// Default maximum bytes returned by one output read.
pub const DEFAULT_READ_LIMIT_BYTES: usize = 20_000;
/// Default maximum serialized bytes retained for one output.
pub const DEFAULT_PER_OUTPUT_LIMIT_BYTES: usize = 1_048_576;
/// Default maximum serialized bytes retained by one runtime.
pub const DEFAULT_AGGREGATE_LIMIT_BYTES: usize = 16_777_216;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
/// Validated policy for model-visible tool output envelopes and storage.
///
/// Only outputs larger than `inline_limit_bytes` are written to a
/// `ToolOutputStore`; inline outputs are already complete in model context and
/// never consume aggregate store budget.
pub struct ToolOutputPolicy {
    inline_limit_bytes: usize,
    read_limit_bytes: usize,
    per_output_limit_bytes: usize,
    aggregate_limit_bytes: usize,
}

impl ToolOutputPolicy {
    /// Builds a validated output policy.
    pub fn new(
        inline_limit_bytes: usize,
        read_limit_bytes: usize,
        per_output_limit_bytes: usize,
        aggregate_limit_bytes: usize,
    ) -> ToolOutputPolicyResult<Self> {
        Self::from_limits(ToolOutputPolicyLimits {
            inline_limit_bytes,
            read_limit_bytes,
            per_output_limit_bytes,
            aggregate_limit_bytes,
        })
    }

    /// Builds a validated output policy from raw limit values.
    pub fn from_limits(limits: ToolOutputPolicyLimits) -> ToolOutputPolicyResult<Self> {
        let inline_limit_bytes = limits.inline_limit_bytes;
        let read_limit_bytes = limits.read_limit_bytes;
        let per_output_limit_bytes = limits.per_output_limit_bytes;
        let aggregate_limit_bytes = limits.aggregate_limit_bytes;
        if inline_limit_bytes == 0 {
            return Err(ToolOutputPolicyError::ZeroInlineLimit);
        }
        if read_limit_bytes == 0 {
            return Err(ToolOutputPolicyError::ZeroReadLimit);
        }
        if per_output_limit_bytes == 0 {
            return Err(ToolOutputPolicyError::ZeroPerOutputLimit);
        }
        if aggregate_limit_bytes == 0 {
            return Err(ToolOutputPolicyError::ZeroAggregateLimit);
        }
        if inline_limit_bytes > per_output_limit_bytes {
            return Err(ToolOutputPolicyError::InlineLimitExceedsPerOutput {
                inline_limit_bytes,
                per_output_limit_bytes,
            });
        }
        if read_limit_bytes > per_output_limit_bytes {
            return Err(ToolOutputPolicyError::ReadLimitExceedsPerOutput {
                read_limit_bytes,
                per_output_limit_bytes,
            });
        }
        if per_output_limit_bytes > aggregate_limit_bytes {
            return Err(ToolOutputPolicyError::PerOutputLimitExceedsAggregate {
                per_output_limit_bytes,
                aggregate_limit_bytes,
            });
        }
        Ok(Self {
            inline_limit_bytes,
            read_limit_bytes,
            per_output_limit_bytes,
            aggregate_limit_bytes,
        })
    }

    /// Returns the maximum bytes kept inline in model-visible output.
    pub fn inline_limit_bytes(&self) -> usize {
        self.inline_limit_bytes
    }

    /// Returns the maximum bytes returned by one intrinsic read.
    pub fn read_limit_bytes(&self) -> usize {
        self.read_limit_bytes
    }

    /// Returns the maximum serialized bytes stored for one output.
    pub fn per_output_limit_bytes(&self) -> usize {
        self.per_output_limit_bytes
    }

    /// Returns the aggregate serialized bytes retained by one runtime.
    pub fn aggregate_limit_bytes(&self) -> usize {
        self.aggregate_limit_bytes
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
/// Raw output policy limits that must be validated before runtime use.
pub struct ToolOutputPolicyLimits {
    /// Maximum bytes kept inline in model-visible output.
    pub inline_limit_bytes: usize,
    /// Maximum bytes returned by one intrinsic read.
    pub read_limit_bytes: usize,
    /// Maximum serialized bytes stored for one output.
    pub per_output_limit_bytes: usize,
    /// Aggregate serialized bytes retained by one runtime.
    pub aggregate_limit_bytes: usize,
}

impl Default for ToolOutputPolicyLimits {
    fn default() -> Self {
        Self {
            inline_limit_bytes: DEFAULT_INLINE_LIMIT_BYTES,
            read_limit_bytes: DEFAULT_READ_LIMIT_BYTES,
            per_output_limit_bytes: DEFAULT_PER_OUTPUT_LIMIT_BYTES,
            aggregate_limit_bytes: DEFAULT_AGGREGATE_LIMIT_BYTES,
        }
    }
}

impl Default for ToolOutputPolicy {
    fn default() -> Self {
        Self {
            inline_limit_bytes: DEFAULT_INLINE_LIMIT_BYTES,
            read_limit_bytes: DEFAULT_READ_LIMIT_BYTES,
            per_output_limit_bytes: DEFAULT_PER_OUTPUT_LIMIT_BYTES,
            aggregate_limit_bytes: DEFAULT_AGGREGATE_LIMIT_BYTES,
        }
    }
}

#[derive(Clone, Debug, Eq, Error, PartialEq)]
/// Errors returned while validating an output policy.
pub enum ToolOutputPolicyError {
    /// Inline limit was zero.
    #[error("[ai_tool_calling/policy] inline output limit must be greater than zero")]
    ZeroInlineLimit,
    /// Read limit was zero.
    #[error("[ai_tool_calling/policy] read output limit must be greater than zero")]
    ZeroReadLimit,
    /// Per-output limit was zero.
    #[error("[ai_tool_calling/policy] per-output store limit must be greater than zero")]
    ZeroPerOutputLimit,
    /// Aggregate limit was zero.
    #[error("[ai_tool_calling/policy] aggregate store limit must be greater than zero")]
    ZeroAggregateLimit,
    /// Inline limit exceeded the per-output storage limit.
    #[error(
        "[ai_tool_calling/policy] inline limit {inline_limit_bytes} exceeds per-output limit {per_output_limit_bytes}"
    )]
    InlineLimitExceedsPerOutput {
        /// Configured inline byte limit.
        inline_limit_bytes: usize,
        /// Configured per-output byte limit.
        per_output_limit_bytes: usize,
    },
    /// Read limit exceeded the per-output storage limit.
    #[error(
        "[ai_tool_calling/policy] read limit {read_limit_bytes} exceeds per-output limit {per_output_limit_bytes}"
    )]
    ReadLimitExceedsPerOutput {
        /// Configured read byte limit.
        read_limit_bytes: usize,
        /// Configured per-output byte limit.
        per_output_limit_bytes: usize,
    },
    /// Per-output storage limit exceeded the aggregate storage limit.
    #[error(
        "[ai_tool_calling/policy] per-output limit {per_output_limit_bytes} exceeds aggregate limit {aggregate_limit_bytes}"
    )]
    PerOutputLimitExceedsAggregate {
        /// Configured per-output byte limit.
        per_output_limit_bytes: usize,
        /// Configured aggregate byte limit.
        aggregate_limit_bytes: usize,
    },
}

/// Result alias for output policy validation.
pub type ToolOutputPolicyResult<T> = std::result::Result<T, ToolOutputPolicyError>;
