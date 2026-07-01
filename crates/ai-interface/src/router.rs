//! Model routing request DTOs and router trait.

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::DynModel;

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
/// Model provider identifiers understood by workspace model routing.
pub enum ProviderKind {
    /// Development and test mock provider.
    Mock,
    /// OpenAI model provider.
    #[serde(rename = "openai")]
    OpenAi,
    /// Anthropic model provider.
    Anthropic,
    /// Google Gemini model provider.
    Google,
    /// xAI/Grok model provider.
    Xai,
}

impl ProviderKind {
    /// Parses a deployment-config provider identifier.
    pub fn from_config_str(value: &str) -> Option<Self> {
        match value {
            "mock" => Some(Self::Mock),
            "openai" => Some(Self::OpenAi),
            "anthropic" => Some(Self::Anthropic),
            "google" => Some(Self::Google),
            "xai" => Some(Self::Xai),
            _ => None,
        }
    }

    /// Returns the stable deployment-config provider identifier.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Mock => "mock",
            Self::OpenAi => "openai",
            Self::Anthropic => "anthropic",
            Self::Google => "google",
            Self::Xai => "xai",
        }
    }
}

impl std::fmt::Display for ProviderKind {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
/// Typed model capabilities used by route requirements and preferences.
pub enum ModelFeature {
    /// Supports model-visible tool calls.
    ToolCalling,
    /// Supports structured JSON response requests.
    StructuredOutput,
    /// Supports image or screenshot inputs.
    Vision,
    /// Advertises a notably large context window.
    LongContext,
    /// Advertises stronger reasoning behavior.
    Reasoning,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
/// Hard model route requirement.
pub enum ModelRequirement {
    /// Require a configured catalog model id.
    ModelId(String),
    /// Require a provider family.
    Provider(ProviderKind),
    /// Require a model capability.
    Feature(ModelFeature),
    /// Require at least this advertised total context window.
    MinContextTokens(u32),
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
/// Model route ordering preference.
pub enum ModelPreference {
    /// Sort by deployment-configured priority, lower values first.
    DeploymentPriority,
    /// Sort by internal intelligence score, higher values first.
    Intelligence,
    /// Sort by speed tier, faster values first.
    Speed,
    /// Sort by cost tier, cheaper values first.
    LowCost,
    /// Sort by advertised context window, larger values first.
    LargeContext,
    /// Prefer models that advertise the capability.
    Feature(ModelFeature),
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Request used to resolve an ordered model route.
pub struct ModelRouteRequest {
    /// Hard requirements applied before ranking candidates.
    pub requirements: Vec<ModelRequirement>,
    /// Ordered preferences applied after hard requirements.
    pub preferences: Vec<ModelPreference>,
}

impl Default for ModelRouteRequest {
    fn default() -> Self {
        Self {
            requirements: Vec::new(),
            preferences: vec![ModelPreference::DeploymentPriority],
        }
    }
}

impl ModelRouteRequest {
    /// Builds a model route request incrementally.
    pub fn builder() -> ModelRouteRequestBuilder {
        ModelRouteRequestBuilder::default()
    }
}

#[derive(Clone, Debug, Default)]
/// Builder for [`ModelRouteRequest`].
pub struct ModelRouteRequestBuilder {
    requirements: Vec<ModelRequirement>,
    preferences: Vec<ModelPreference>,
}

impl ModelRouteRequestBuilder {
    /// Adds a hard requirement.
    pub fn require(mut self, requirement: ModelRequirement) -> Self {
        self.requirements.push(requirement);
        self
    }

    /// Adds a required model capability.
    pub fn require_feature(self, feature: ModelFeature) -> Self {
        self.require(ModelRequirement::Feature(feature))
    }

    /// Adds an ordered preference.
    pub fn prefer(mut self, preference: ModelPreference) -> Self {
        self.preferences.push(preference);
        self
    }

    /// Finishes the route request.
    pub fn build(self) -> ModelRouteRequest {
        ModelRouteRequest {
            requirements: self.requirements,
            preferences: self.preferences,
        }
    }
}

#[derive(Debug, Error)]
/// Errors returned by model routers.
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
        source: Box<dyn std::error::Error + Send + Sync>,
    },
}

impl ModelRouterError {
    /// Wraps an internal router failure.
    pub fn internal(source: impl std::error::Error + Send + Sync + 'static) -> Self {
        Self::Internal {
            source: Box::new(source),
        }
    }
}

/// Result alias for model-router operations.
pub type ModelRouterResult<T> = std::result::Result<T, ModelRouterError>;

#[cfg_attr(
    any(test, doctest, feature = "test-support"),
    unimock::unimock(api = ModelRouterMock)
)]
/// Provider-agnostic model routing boundary.
pub trait ModelRouter: Send + Sync {
    /// Resolves an ordered model implementation for a route request.
    fn resolve(&self, request: &ModelRouteRequest) -> ModelRouterResult<DynModel>;
}

/// Shared dynamic model-router alias.
pub type DynModelRouter = Arc<dyn ModelRouter>;
