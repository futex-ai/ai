//! Model routing request DTOs and router trait.

use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::{DynModel, ModelRouterResult};

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
    /// DeepSeek model provider.
    #[serde(rename = "deepseek")]
    DeepSeek,
    /// Google Gemini model provider.
    Google,
    /// Moonshot AI Kimi model provider.
    Kimi,
    /// MiniMax model provider.
    #[serde(rename = "minimax")]
    MiniMax,
    /// Alibaba Qwen model provider.
    Qwen,
    /// TypeSafe System One judgment provider.
    #[serde(rename = "typesafe")]
    TypeSafe,
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
            "deepseek" => Some(Self::DeepSeek),
            "google" => Some(Self::Google),
            "kimi" => Some(Self::Kimi),
            "minimax" => Some(Self::MiniMax),
            "qwen" => Some(Self::Qwen),
            "typesafe" => Some(Self::TypeSafe),
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
            Self::DeepSeek => "deepseek",
            Self::Google => "google",
            Self::Kimi => "kimi",
            Self::MiniMax => "minimax",
            Self::Qwen => "qwen",
            Self::TypeSafe => "typesafe",
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
    /// Supports video inputs.
    VideoInput,
    /// Advertises a notably large context window.
    LongContext,
    /// Advertises stronger reasoning behavior.
    Reasoning,
    /// Supports generating or editing images.
    ImageGeneration,
    /// Supports typed judgment questions over shared state.
    Judgment,
    /// Supports generating videos.
    VideoGeneration,
}

impl ModelFeature {
    /// Returns the stable deployment-config capability identifier.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ToolCalling => "tool_calling",
            Self::StructuredOutput => "structured_output",
            Self::Vision => "vision",
            Self::VideoInput => "video_input",
            Self::LongContext => "long_context",
            Self::Reasoning => "reasoning",
            Self::ImageGeneration => "image_generation",
            Self::Judgment => "judgment",
            Self::VideoGeneration => "video_generation",
        }
    }
}

impl std::fmt::Display for ModelFeature {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
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
