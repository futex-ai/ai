//! Serializable records of failed fallback attempts.

use serde::{Deserialize, Serialize, Serializer};

use crate::{ModelControl, ModelError};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// A failed attempt discarded before a later model completed the response.
pub struct ModelAttempt {
    /// Serializable failure details from the discarded attempt.
    pub error: ModelAttemptError,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[non_exhaustive]
/// Snapshot of an existing model error without an internal error source object.
pub enum ModelAttemptError {
    /// The attempt reached a provider rate limit.
    RateLimited {
        /// Provider that rejected the attempt.
        provider: String,
        /// Requested provider model identifier.
        model_id: String,
        /// Provider failure details.
        message: String,
    },
    /// The provider returned a transient failure before progress.
    TransientProvider {
        /// Provider that failed the attempt.
        provider: String,
        /// Requested provider model identifier.
        model_id: String,
        /// Provider failure details.
        message: String,
    },
    /// The provider failed after stream progress.
    Interrupted {
        /// Provider that failed the attempt.
        provider: String,
        /// Requested provider model identifier.
        model_id: String,
        /// Stream failure details.
        message: String,
    },
    /// The provider returned a non-retryable failure.
    Provider {
        /// Provider that failed the attempt.
        provider: String,
        /// Requested provider model identifier.
        model_id: String,
        /// Provider failure details.
        message: String,
    },
    /// The attempt exceeded the provider context window.
    ContextLimitExceeded {
        /// Provider that rejected the attempt.
        provider: String,
        /// Requested provider model identifier.
        model_id: String,
        /// Provider failure details.
        message: String,
    },
    /// The selected model could not honor a requested control.
    UnsupportedControl {
        /// Provider selected for the attempt.
        provider: String,
        /// Requested provider model identifier.
        model_id: String,
        /// Control that the model could not honor.
        control: ModelControl,
    },
    /// An internal failure whose source remains on the original model error.
    Internal {
        /// Display text of the original internal source.
        message: String,
    },
}

impl From<&ModelError> for ModelAttempt {
    fn from(error: &ModelError) -> Self {
        let error = match error {
            ModelError::RateLimited {
                provider,
                model_id,
                message,
            } => ModelAttemptError::RateLimited {
                provider: provider.clone(),
                model_id: model_id.clone(),
                message: message.clone(),
            },
            ModelError::TransientProvider {
                provider,
                model_id,
                message,
            } => ModelAttemptError::TransientProvider {
                provider: provider.clone(),
                model_id: model_id.clone(),
                message: message.clone(),
            },
            ModelError::Interrupted {
                provider,
                model_id,
                message,
            } => ModelAttemptError::Interrupted {
                provider: provider.clone(),
                model_id: model_id.clone(),
                message: message.clone(),
            },
            ModelError::Provider {
                provider,
                model_id,
                message,
            } => ModelAttemptError::Provider {
                provider: provider.clone(),
                model_id: model_id.clone(),
                message: message.clone(),
            },
            ModelError::ContextLimitExceeded {
                provider,
                model_id,
                message,
            } => ModelAttemptError::ContextLimitExceeded {
                provider: provider.clone(),
                model_id: model_id.clone(),
                message: message.clone(),
            },
            ModelError::UnsupportedControl {
                provider,
                model_id,
                control,
            } => ModelAttemptError::UnsupportedControl {
                provider: provider.clone(),
                model_id: model_id.clone(),
                control: *control,
            },
            ModelError::Internal { source } => ModelAttemptError::Internal {
                message: source.to_string(),
            },
        };
        Self { error }
    }
}

pub(crate) fn serialize_model_error<S>(error: &ModelError, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    ModelAttempt::from(error).error.serialize(serializer)
}
