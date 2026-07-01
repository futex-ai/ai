//! Tool DTOs and shared generic tool trait.

use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Model-visible tool call emitted by an assistant response.
pub struct ToolCall {
    /// Provider-generated tool call identifier.
    pub id: String,
    /// Canonical tool name to dispatch.
    pub name: String,
    /// Raw JSON arguments supplied by the model.
    pub input: Value,
    /// Runtime operation id used as an idempotency key for execution.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub operation_id: Option<String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
/// Canonical tool definition exposed to the model for a turn.
pub struct ToolDefinition {
    /// Stable tool name used in tool calls.
    pub name: String,
    /// Human-readable tool description shown to the model.
    pub description: String,
    /// JSON schema describing accepted tool arguments.
    pub input_schema: Value,
    /// Public one-word activity label used by UI status surfaces.
    #[serde(default, skip)]
    pub activity_verb: Option<String>,
}

#[derive(Debug, Error)]
/// Errors returned by the tool boundary.
pub enum ToolError {
    /// The model requested a tool name that is not registered.
    #[error("[ai_interface/tool] unknown tool `{name}`")]
    UnknownTool {
        /// Unknown model-visible tool name.
        name: String,
    },
    /// The tool adapter could not decode raw JSON arguments into its typed DTO.
    #[error("[ai_interface/tool] invalid arguments for `{tool_name}`: {source}")]
    InvalidArguments {
        /// Tool name whose arguments failed to decode.
        tool_name: String,
        /// Underlying JSON decode failure.
        source: serde_json::Error,
    },
    /// The typed tool logic failed after successful argument decoding.
    #[error("[ai_interface/tool] execution failed for `{tool_name}`: {source}")]
    Execution {
        /// Tool name whose execution failed.
        tool_name: String,
        /// Underlying typed tool failure.
        source: Box<dyn std::error::Error + Send + Sync>,
    },
}

impl ToolError {
    /// Builds an invalid-arguments error for one tool call.
    pub fn invalid_arguments(tool_name: impl Into<String>, source: serde_json::Error) -> Self {
        Self::InvalidArguments {
            tool_name: tool_name.into(),
            source,
        }
    }

    /// Builds an execution error for one tool call.
    pub fn execution(
        tool_name: impl Into<String>,
        source: impl std::error::Error + Send + Sync + 'static,
    ) -> Self {
        Self::Execution {
            tool_name: tool_name.into(),
            source: Box::new(source),
        }
    }
}

/// Result alias for tool operations.
pub type ToolResult<T> = std::result::Result<T, ToolError>;

#[derive(Clone, Debug, PartialEq)]
/// Context for one tool invocation at the generic tool boundary.
pub struct ToolInvocation {
    /// Canonical model-visible tool name.
    pub tool_name: String,
    /// Raw JSON arguments supplied by the model.
    pub input: Value,
    /// Runtime operation id to use as an idempotency key.
    pub operation_id: String,
}

#[cfg_attr(
    any(test, doctest, feature = "test-support"),
    unimock::unimock(api = ToolMock)
)]
#[async_trait]
/// Shared generic tool boundary.
pub trait Tool: Send + Sync {
    /// Returns the model-visible tool definitions owned by this tool adapter.
    fn definitions(&self) -> Vec<ToolDefinition>;

    /// Dispatches one model-visible tool call by name with raw JSON arguments.
    async fn call(&self, tool_name: &str, input: Value) -> ToolResult<Value>;

    /// Dispatches one model-visible tool call with durable invocation context.
    async fn call_with_invocation(&self, invocation: ToolInvocation) -> ToolResult<Value> {
        self.call(&invocation.tool_name, invocation.input).await
    }

    /// Returns the selector/runtime group that exposed `tool_name`, when known.
    fn group_for_tool(&self, tool_name: &str) -> Option<&'static str> {
        let _ = tool_name;
        None
    }
}

/// Shared dynamic tool alias.
pub type DynTool = Arc<dyn Tool>;
