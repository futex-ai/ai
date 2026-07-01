//! Logger trait and log DTOs for tool-calling runtimes.

use std::sync::Arc;

use serde_json::Value;
use thiserror::Error;

use crate::{ModelRequest, ModelResponse, ToolCall};

#[derive(Clone, Debug, PartialEq)]
/// Result observed for one model call.
pub enum ModelCallLogResult {
    /// The model returned a normal response.
    Success {
        /// Response returned by the model.
        response: Box<ModelResponse>,
    },
    /// The model call failed before a response was produced.
    Error {
        /// User-facing error text returned by the model boundary.
        message: String,
        /// Full debug representation of the model-boundary failure.
        debug: String,
    },
}

#[derive(Clone, Debug, PartialEq)]
/// Combined model-call log payload.
pub struct ModelCallLogEntry {
    /// Request sent to the model.
    pub request: ModelRequest,
    /// Result returned by the model boundary.
    pub result: ModelCallLogResult,
    /// Wall-clock latency in milliseconds.
    pub latency_ms: u128,
}

#[derive(Clone, Debug, PartialEq)]
/// Tool-call log outcome.
pub enum ToolCallLogResult {
    /// The tool returned JSON successfully.
    Success {
        /// JSON payload returned by the tool.
        output: Value,
    },
    /// The tool failed and the runtime recorded an error message into conversation.
    Error {
        /// Error text recorded for the model to see.
        message: String,
        /// Full debug representation of the tool-boundary failure.
        debug: String,
    },
}

#[derive(Clone, Debug, PartialEq)]
/// Combined tool-call log payload.
pub struct ToolCallLogEntry {
    /// Tool call emitted by the model.
    pub call: ToolCall,
    /// Runtime group that exposed the tool, when known.
    pub tool_group: Option<String>,
    /// Runtime-observed tool result.
    pub result: ToolCallLogResult,
    /// Wall-clock latency in milliseconds.
    pub latency_ms: u128,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
/// Lifecycle phase for public tool activity reporting.
pub enum ToolActivityPhase {
    /// The tool is about to execute.
    Started,
    /// The tool finished executing.
    Completed,
}

#[derive(Clone, Debug, PartialEq)]
/// Public-safe tool activity payload.
pub struct ToolActivityLogEntry {
    /// Model-visible tool name.
    pub tool_name: String,
    /// Public one-word activity label, when the runtime can resolve one.
    pub activity_verb: Option<String>,
    /// Tool activity phase.
    pub phase: ToolActivityPhase,
}

#[derive(Clone, Debug, PartialEq)]
/// Terminal turn outcome log payload.
pub struct TurnOutcomeLogEntry {
    /// Most recent non-empty assistant response produced during the turn.
    pub assistant_message: String,
    /// Number of model rounds executed in the turn.
    pub steps_taken: usize,
    /// Whether the turn reached a terminal completion state.
    pub completed: bool,
    /// Maximum step budget when one was configured.
    pub max_steps: Option<usize>,
}

#[derive(Debug, Error)]
/// Errors returned by logger callbacks.
pub enum LoggerError {
    #[error("[ai_interface/logger] log callback failed: {source}")]
    /// Logger implementation failure.
    Write {
        /// Underlying logger failure.
        source: Box<dyn std::error::Error + Send + Sync>,
    },
}

impl LoggerError {
    /// Wraps a logger callback failure.
    pub fn write(source: impl std::error::Error + Send + Sync + 'static) -> Self {
        Self::Write {
            source: Box::new(source),
        }
    }
}

/// Result alias for logger operations.
pub type LoggerResult<T> = std::result::Result<T, LoggerError>;

#[cfg_attr(
    any(test, doctest, feature = "test-support"),
    unimock::unimock(api = LoggerMock)
)]
/// Logging hook boundary for runtime orchestration.
pub trait Logger: Send + Sync {
    /// Records one combined model request/response event.
    fn log_model_call(&self, entry: &ModelCallLogEntry) -> LoggerResult<()>;

    /// Records one tool call event.
    fn log_tool_call(&self, entry: &ToolCallLogEntry) -> LoggerResult<()>;

    /// Records a public-safe tool activity lifecycle event.
    fn log_tool_activity(&self, _entry: &ToolActivityLogEntry) -> LoggerResult<()> {
        Ok(())
    }

    /// Records one terminal turn outcome.
    fn log_turn_outcome(&self, entry: &TurnOutcomeLogEntry) -> LoggerResult<()>;
}

/// Shared dynamic logger alias.
pub type DynLogger = Arc<dyn Logger>;

/// Default logger that drops all events.
pub struct NoopLogger;

impl Logger for NoopLogger {
    fn log_model_call(&self, _entry: &ModelCallLogEntry) -> LoggerResult<()> {
        Ok(())
    }

    fn log_tool_call(&self, _entry: &ToolCallLogEntry) -> LoggerResult<()> {
        Ok(())
    }

    fn log_tool_activity(&self, _entry: &ToolActivityLogEntry) -> LoggerResult<()> {
        Ok(())
    }

    fn log_turn_outcome(&self, _entry: &TurnOutcomeLogEntry) -> LoggerResult<()> {
        Ok(())
    }
}
