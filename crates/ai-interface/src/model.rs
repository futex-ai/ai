//! Model DTOs, normalized finish reasons, and model trait.

use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

use crate::{
    ConversationMessage, ProviderConversationItem, ToolCall, ToolDefinition, usage::ModelUsage,
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
    /// Catalog model id that selected this provider call, when available.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub catalog_model_id: Option<String>,
    /// Normalized thinking level selected for this call, when available.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking_level: Option<String>,
    /// Assistant text returned by the model.
    pub assistant_message: String,
    /// Tool calls requested by the model.
    pub tool_calls: Vec<ToolCall>,
    /// Normalized provider finish reason for this response.
    pub finish_reason: FinishReason,
    /// Parsed structured output when the request required a schema.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub structured_output: Option<Value>,
    /// Provider-specific items that must be retained for future turns.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub provider_context: Vec<ProviderConversationItem>,
    /// Usage and estimated cost information for the call.
    pub usage: ModelUsage,
}

#[derive(Debug, Error)]
/// Errors returned by the model boundary.
pub enum ModelError {
    /// The upstream provider rejected the request due to a rate limit.
    #[error(
        "[ai_interface/model] provider rate limit for `{provider}` model `{model_id}`: {message}"
    )]
    RateLimited {
        /// Provider that returned the rate-limit response.
        provider: String,
        /// Model identifier requested from the provider.
        model_id: String,
        /// Provider-supplied failure details.
        message: String,
    },
    /// The upstream provider returned a transient failure that may succeed later.
    #[error(
        "[ai_interface/model] transient provider failure for `{provider}` model `{model_id}`: {message}"
    )]
    TransientProvider {
        /// Provider that returned the transient failure.
        provider: String,
        /// Model identifier requested from the provider.
        model_id: String,
        /// Provider-supplied failure details.
        message: String,
    },
    /// The upstream provider returned a non-retryable failure.
    #[error("[ai_interface/model] provider failure for `{provider}` model `{model_id}`: {message}")]
    Provider {
        /// Provider that returned the failure.
        provider: String,
        /// Model identifier requested from the provider.
        model_id: String,
        /// Provider-supplied failure details.
        message: String,
    },
    /// The upstream provider rejected the request because the input exceeded
    /// the model context window.
    #[error(
        "[ai_interface/model] context limit exceeded for `{provider}` model `{model_id}`: {message}"
    )]
    ContextLimitExceeded {
        /// Provider that returned the failure.
        provider: String,
        /// Model identifier requested from the provider.
        model_id: String,
        /// Provider-supplied failure details.
        message: String,
    },
    /// Unhandled model-boundary failure.
    #[error("[ai_interface/model] internal error: {source}")]
    Internal {
        /// Underlying model failure.
        source: Box<dyn std::error::Error + Send + Sync>,
    },
}

impl ModelError {
    /// Builds a rate-limited model error.
    pub fn rate_limited(
        provider: impl Into<String>,
        model_id: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self::RateLimited {
            provider: provider.into(),
            model_id: model_id.into(),
            message: message.into(),
        }
    }

    /// Builds a transient provider error.
    pub fn transient_provider(
        provider: impl Into<String>,
        model_id: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self::TransientProvider {
            provider: provider.into(),
            model_id: model_id.into(),
            message: message.into(),
        }
    }

    /// Builds a non-retryable provider error.
    pub fn provider(
        provider: impl Into<String>,
        model_id: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self::Provider {
            provider: provider.into(),
            model_id: model_id.into(),
            message: message.into(),
        }
    }

    /// Builds a context-window overflow error.
    pub fn context_limit_exceeded(
        provider: impl Into<String>,
        model_id: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self::ContextLimitExceeded {
            provider: provider.into(),
            model_id: model_id.into(),
            message: message.into(),
        }
    }

    /// Wraps an internal model-boundary error.
    pub fn internal(source: impl std::error::Error + Send + Sync + 'static) -> Self {
        Self::Internal {
            source: Box::new(source),
        }
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
    /// Produces the next assistant response for the current conversation state.
    async fn complete(&self, request: &ModelRequest) -> ModelResult<ModelResponse>;
}

/// Shared dynamic model alias.
pub type DynModel = Arc<dyn Model>;
