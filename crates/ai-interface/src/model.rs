//! Model DTOs, normalized finish reasons, and model trait.

use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    ConversationMessage, ConversationPart, DynModelEventStream, Effort, ModelAttempt,
    ModelCallControls, ModelError, ModelResponseFold, ToolDefinition, usage::ModelUsage,
};

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
/// Structured response contract the model should satisfy.
pub struct StructuredOutputSchema {
    /// Stable schema name used by providers that require one.
    pub name: String,
    /// JSON Schema describing the required response shape.
    pub schema: Value,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
/// Normalized request sent to a tool-calling model.
pub struct ModelRequest {
    /// System prompt rendered for the current turn.
    pub system_prompt: String,
    /// Ordered retained conversation history.
    pub messages: Vec<ConversationMessage>,
    /// Tool definitions currently available to the model.
    pub tools: Vec<ToolDefinition>,
    /// Optional JSON Schema the final assistant response must satisfy.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_schema: Option<StructuredOutputSchema>,
    /// Optional generation and execution intent for this call.
    #[serde(default, skip_serializing_if = "ModelCallControls::is_default")]
    pub controls: ModelCallControls,
}

impl ModelRequest {
    /// Returns the authored system prompt when it contains non-whitespace content.
    pub fn nonblank_system_prompt(&self) -> Option<&str> {
        (!self.system_prompt.trim().is_empty()).then_some(self.system_prompt.as_str())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Normalized reason a provider stopped generating a model response.
pub enum FinishReason {
    /// The model reached a natural stop point or caller-provided stop sequence.
    Stop,
    /// The model requested one or more tool calls.
    ToolCalls,
    /// The response was truncated by a token, output, or context limit.
    Truncated,
    /// The provider filtered or refused the response for policy reasons.
    Filtered,
    /// Provider-specific finish reason not yet normalized by this contract.
    Other(String),
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
/// Normalized response returned by a tool-calling model.
pub struct ModelResponse {
    /// Provider that served the response.
    pub provider: String,
    /// Concrete provider model identifier used for the call.
    pub model_id: String,
    /// Provider/model text id that selected this call, when available.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub catalog_model_id: Option<String>,
    /// Resolved portable effort selected for this call, when available.
    #[serde(rename = "effort", skip_serializing_if = "Option::is_none")]
    pub thinking_level: Option<Effort>,
    /// Assistant parts in their generated order.
    pub parts: Vec<ConversationPart>,
    /// Normalized provider finish reason for this response.
    pub finish_reason: FinishReason,
    /// Parsed structured output when the request required a schema.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub structured_output: Option<Value>,
    /// Usage and estimated cost information for the call.
    pub usage: ModelUsage,
    /// Failed fallback attempts in the order they occurred.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub attempts: Vec<ModelAttempt>,
}

impl ModelResponse {
    /// Returns the portable effort resolved for this response.
    pub fn effort(&self) -> Option<Effort> {
        self.thinking_level
    }
}

/// Result alias for model operations.
pub type ModelResult<T> = std::result::Result<T, ModelError>;

#[cfg_attr(
    any(test, doctest, feature = "test-support"),
    unimock::unimock(api = ModelMock)
)]
#[async_trait]
/// Provider-agnostic tool-calling model boundary.
pub trait Model: Send + Sync {
    /// Opens the ordered event stream for the next assistant response.
    async fn stream(&self, request: &ModelRequest) -> ModelResult<DynModelEventStream>;

    /// Produces a response by folding the events returned by `stream`.
    async fn complete(&self, request: &ModelRequest) -> ModelResult<ModelResponse> {
        let mut stream = self.stream(request).await?;
        let mut fold = ModelResponseFold::new();
        while let Some(event) = stream.next().await? {
            fold.push(event)?;
        }
        fold.finish()
    }
}

/// Shared dynamic model alias.
pub type DynModel = Arc<dyn Model>;
