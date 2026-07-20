//! Error types for the generic tool-calling runtime.

use std::error::Error as StdError;

use thiserror::Error;

use ai_interface::{LoggerError, ModelError, ToolError, ToolOutputEnvelopeError};

use crate::{ToolOutputPolicyError, ToolOutputStoreError};

#[derive(Debug, Error)]
/// Errors returned by the generic tool-calling runtime.
pub enum Error {
    /// Model execution failed.
    #[error("[ai_tool_calling/runtime] model error: {0}")]
    Model(#[from] ModelError),
    /// Tool dispatch or tool execution failed.
    #[error("[ai_tool_calling/runtime] tool error: {0}")]
    Tool(#[from] ToolError),
    /// Logger persistence or callback logic failed.
    #[error("[ai_tool_calling/runtime] logger error: {0}")]
    Logger(#[from] LoggerError),
    /// A caller-provided turn checkpoint failed.
    #[error("[ai_tool_calling/runtime] checkpoint error: {source}")]
    Checkpoint {
        /// Underlying checkpoint failure.
        #[source]
        source: Box<dyn StdError + Send + Sync>,
    },
    /// Two injected tools exposed the same model-visible name.
    #[error("[ai_tool_calling/runtime] duplicate tool definition `{name}`")]
    DuplicateToolDefinition {
        /// Conflicting model-visible tool name.
        name: String,
    },
    /// A caller-provided tool attempted to use a runtime-reserved name.
    #[error("[ai_tool_calling/runtime] reserved tool definition `{name}`")]
    ReservedToolDefinition {
        /// Reserved model-visible tool name.
        name: String,
    },
    /// Output policy validation failed during runtime construction.
    #[error("[ai_tool_calling/runtime] invalid output policy: {source}")]
    OutputPolicy {
        /// Underlying policy validation error.
        source: ToolOutputPolicyError,
    },
    /// The runtime could not serialize a raw tool output.
    #[error("[ai_tool_calling/runtime] failed to serialize output for `{tool_name}`: {source}")]
    OutputSerialization {
        /// Tool name whose output failed to serialize.
        tool_name: String,
        /// Underlying JSON serialization failure.
        source: serde_json::Error,
    },
    /// The runtime could not serialize a model-visible output envelope.
    #[error("[ai_tool_calling/runtime] failed to serialize envelope for `{tool_name}`: {source}")]
    EnvelopeSerialization {
        /// Tool name whose envelope failed to serialize.
        tool_name: String,
        /// Underlying JSON serialization failure.
        source: serde_json::Error,
    },
    /// The runtime could not construct a valid model-visible output envelope.
    #[error("[ai_tool_calling/runtime] invalid output envelope: {source}")]
    OutputEnvelope {
        /// Underlying envelope construction error.
        source: ToolOutputEnvelopeError,
    },
    /// A store returned a read-only error while writing output.
    #[error("[ai_tool_calling/runtime] unexpected output store write failure: {source}")]
    UnexpectedOutputStoreWrite {
        /// Underlying store error.
        source: ToolOutputStoreError,
    },
}

impl Error {
    /// Builds a checkpoint error from an embedding runtime failure.
    pub fn checkpoint(source: impl StdError + Send + Sync + 'static) -> Self {
        Self::Checkpoint {
            source: Box::new(source),
        }
    }
}

/// Result alias for runtime operations.
pub type Result<T> = std::result::Result<T, Error>;
