//! Existing model error contract and constructors.

use thiserror::Error;

use crate::ModelControl;

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
    /// A streaming response failed after the provider had begun generation.
    #[error(
        "[ai_interface/model] interrupted provider response for `{provider}` model `{model_id}`: {message}"
    )]
    Interrupted {
        /// Provider whose response stream was interrupted.
        provider: String,
        /// Model identifier requested from the provider.
        model_id: String,
        /// Failure details reported while consuming the stream.
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
    /// The selected provider or model cannot honor an explicit call control.
    #[error(
        "[ai_interface/model] unsupported {control:?} control for `{provider}` model `{model_id}`"
    )]
    UnsupportedControl {
        /// Provider selected for the call.
        provider: String,
        /// Model identifier selected for the call.
        model_id: String,
        /// Portable control that cannot be honored.
        control: ModelControl,
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

    /// Builds an interrupted streaming-response error.
    pub fn interrupted(
        provider: impl Into<String>,
        model_id: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self::Interrupted {
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

    /// Builds an unsupported-control model error.
    pub fn unsupported_control(
        provider: impl Into<String>,
        model_id: impl Into<String>,
        control: ModelControl,
    ) -> Self {
        Self::UnsupportedControl {
            provider: provider.into(),
            model_id: model_id.into(),
            control,
        }
    }

    /// Wraps an internal model-boundary error.
    pub fn internal(source: impl std::error::Error + Send + Sync + 'static) -> Self {
        Self::Internal {
            source: Box::new(source),
        }
    }
}
