//! Error types for the generic tool-calling runtime.

use std::error::Error as StdError;

use thiserror::Error;

use ai_interface::{LoggerError, ModelError, ToolError};

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
