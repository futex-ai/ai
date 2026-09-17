//! Errors returned by model routers.

use std::error;

use thiserror::Error;

use crate::ModelRequirement;

/// Errors returned by model routers.
#[derive(Debug, Error)]
pub enum ModelRouterError {
    /// No configured models were available.
    #[error("[ai_interface/model_router] no models configured")]
    NoModelsConfigured,
    /// A configured model id is not present in the provider-owned model catalog.
    #[error("[ai_interface/model_router] unknown configured model `{provider}` `{model_id}`")]
    UnknownConfiguredModel {
        /// Configured provider string.
        provider: String,
        /// Configured model id.
        model_id: String,
    },
    /// A configured provider string is not supported by routing.
    #[error(
        "[ai_interface/model_router] unsupported configured provider `{provider}` for model `{model_id}`"
    )]
    UnsupportedConfiguredProvider {
        /// Configured provider string.
        provider: String,
        /// Configured model id.
        model_id: String,
    },
    /// Route requirements removed every candidate model.
    #[error("[ai_interface/model_router] no models matched route requirements")]
    NoModelsMatched {
        /// Requirements that produced no candidates.
        requirements: Vec<ModelRequirement>,
    },
    /// A model could not be built because its API key env var was missing or blank.
    #[error(
        "[ai_interface/model_router] missing API key from env `{env_name}` for model `{model_id}`"
    )]
    MissingApiKeyEnv {
        /// Configured model id.
        model_id: String,
        /// Environment variable name used by the model config.
        env_name: String,
    },
    /// A model could not be built because its API key secret was missing.
    #[error(
        "[ai_interface/model_router] missing API key from secret `{secret_name}` for model `{model_id}`"
    )]
    MissingApiKeySecret {
        /// Configured model id.
        model_id: String,
        /// Secret name used by the model config.
        secret_name: String,
    },
    /// A model config had no API key source.
    #[error("[ai_interface/model_router] model `{model_id}` has no API key credential source")]
    MissingApiKeySource {
        /// Configured model id.
        model_id: String,
    },
    /// The router failed while building a model chain.
    #[error("[ai_interface/model_router] internal error: {source}")]
    Internal {
        /// Underlying router failure.
        source: Box<dyn error::Error + Send + Sync>,
    },
}

impl ModelRouterError {
    /// Wraps an internal router failure.
    pub fn internal(source: impl error::Error + Send + Sync + 'static) -> Self {
        Self::Internal {
            source: Box::new(source),
        }
    }
}

/// Result alias for model-router operations.
pub type ModelRouterResult<T> = std::result::Result<T, ModelRouterError>;
