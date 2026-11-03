//! Serializable completion results with a stable, secret-free error surface.

use ai_interface::{ModelControl, ModelError, ModelResponse};
use json_http::test_support::Redactor;
use serde::{Deserialize, Serialize};

/// One scenario's ordered step results.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct ScenarioOutcome {
    /// Completed steps followed by the first failure, if any.
    pub steps: Vec<StepOutcome>,
    /// Exchanges left when an adapter rejected a request before transport.
    pub remaining_exchanges: usize,
}

/// A model call's response or its error fields.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum StepOutcome {
    /// Successful completion.
    Response {
        /// Full normalized model result, including provider context and usage.
        response: Box<ModelResponse>,
    },
    /// Failed completion.
    Error {
        /// Model error variant and its sanitized fields.
        error: ErrorOutcome,
    },
    /// The tool loop's first response contained no tool call.
    MissingToolCall,
}

/// Serializable counterpart of today's `ModelError`.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "variant")]
pub enum ErrorOutcome {
    /// HTTP rate limiting.
    RateLimited {
        /// Provider identity.
        provider: String,
        /// Concrete provider model id.
        model_id: String,
        /// Sanitized diagnostic.
        message: String,
    },
    /// Retryable provider failure before progress.
    TransientProvider {
        /// Provider identity.
        provider: String,
        /// Concrete provider model id.
        model_id: String,
        /// Sanitized diagnostic.
        message: String,
    },
    /// Stream failure after generation starts.
    Interrupted {
        /// Provider identity.
        provider: String,
        /// Concrete provider model id.
        model_id: String,
        /// Sanitized diagnostic.
        message: String,
    },
    /// Non-retryable provider failure.
    Provider {
        /// Provider identity.
        provider: String,
        /// Concrete provider model id.
        model_id: String,
        /// Sanitized diagnostic.
        message: String,
    },
    /// Input exceeded the context window.
    ContextLimitExceeded {
        /// Provider identity.
        provider: String,
        /// Concrete provider model id.
        model_id: String,
        /// Sanitized diagnostic.
        message: String,
    },
    /// A portable control is unsupported.
    UnsupportedControl {
        /// Provider identity.
        provider: String,
        /// Concrete provider model id.
        model_id: String,
        /// Rejected control.
        control: ModelControl,
    },
    /// Unhandled model failure.
    Internal {
        /// Sanitized source diagnostic.
        source: String,
    },
}

impl ErrorOutcome {
    pub(crate) fn capture(error: ModelError, redactor: &Redactor) -> Self {
        match error {
            ModelError::RateLimited {
                provider,
                model_id,
                message,
            } => Self::RateLimited {
                provider: redactor.text(&provider),
                model_id: redactor.text(&model_id),
                message: redactor.text(&message),
            },
            ModelError::TransientProvider {
                provider,
                model_id,
                message,
            } => Self::TransientProvider {
                provider: redactor.text(&provider),
                model_id: redactor.text(&model_id),
                message: redactor.text(&message),
            },
            ModelError::Interrupted {
                provider,
                model_id,
                message,
            } => Self::Interrupted {
                provider: redactor.text(&provider),
                model_id: redactor.text(&model_id),
                message: redactor.text(&message),
            },
            ModelError::Provider {
                provider,
                model_id,
                message,
            } => Self::Provider {
                provider: redactor.text(&provider),
                model_id: redactor.text(&model_id),
                message: redactor.text(&message),
            },
            ModelError::ContextLimitExceeded {
                provider,
                model_id,
                message,
            } => Self::ContextLimitExceeded {
                provider: redactor.text(&provider),
                model_id: redactor.text(&model_id),
                message: redactor.text(&message),
            },
            ModelError::UnsupportedControl {
                provider,
                model_id,
                control,
            } => Self::UnsupportedControl {
                provider: redactor.text(&provider),
                model_id: redactor.text(&model_id),
                control,
            },
            ModelError::Internal { source } => Self::Internal {
                source: redactor.text(&source.to_string()),
            },
        }
    }
}
