//! Typed known-model metadata used by routing composition roots.

pub use ai_interface::{CostTier, IntelligenceScore, ModelFeature, ProviderKind, SpeedTier};

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
/// Normalized thinking budget level for a catalog model variant.
pub enum ThinkingLevel {
    /// Do not send explicit provider thinking controls.
    Disabled,
    /// Lowest non-disabled provider thinking setting.
    Low,
    /// Balanced provider thinking setting.
    Medium,
    /// High provider thinking setting.
    High,
    /// Stronger-than-high provider thinking setting when supported.
    ExtraHigh,
    /// Maximum provider thinking setting when supported.
    Max,
}

impl ThinkingLevel {
    /// Returns the stable snake-case value used in logs and docs.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Disabled => "disabled",
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
            Self::ExtraHigh => "extra_high",
            Self::Max => "max",
        }
    }

    /// Returns true when a provider should receive explicit thinking controls.
    pub fn is_enabled(self) -> bool {
        self != Self::Disabled
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
/// Provider-owned model metadata consumed by model routers.
pub struct KnownModelSpec {
    /// Provider that owns this model id.
    pub provider: ProviderKind,
    /// Unique workspace catalog identifier for this deployable model variant.
    pub id: &'static str,
    /// Concrete provider model identifier sent to the upstream API.
    pub provider_model_id: &'static str,
    /// Provider-advertised total context window in tokens.
    pub context_window_tokens: u32,
    /// Provider-advertised maximum generated text tokens; zero for non-chat models.
    pub max_output_tokens: u32,
    /// Internal intelligence ranking from one to ten.
    pub intelligence_score: IntelligenceScore,
    /// Coarse speed tier.
    pub speed: SpeedTier,
    /// Coarse cost tier.
    pub cost: CostTier,
    /// Explicit thinking level for this catalog variant.
    pub thinking_level: ThinkingLevel,
    /// Capabilities advertised for routing requirements and preferences.
    pub features: &'static [ModelFeature],
}

impl KnownModelSpec {
    /// Returns true when this model advertises the supplied feature.
    pub fn has_feature(&self, feature: ModelFeature) -> bool {
        self.features.contains(&feature)
    }
}

/// Resolves a requested thinking level to the highest catalog level for the
/// provider model that does not exceed the request.
///
/// Returns `None` when the catalog has no level at or below the request. This
/// lets adapters preserve explicit custom mappings or return a typed
/// configuration error when no safe downgrade exists.
pub fn resolve_catalog_thinking_level(
    models: &[KnownModelSpec],
    provider: ProviderKind,
    provider_model_id: &str,
    requested: ThinkingLevel,
) -> Option<ThinkingLevel> {
    models
        .iter()
        .filter(|model| {
            model.provider == provider
                && model.provider_model_id == provider_model_id
                && model.thinking_level <= requested
        })
        .map(|model| model.thinking_level)
        .max()
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
/// Registry of strongly typed known provider models.
pub struct KnownModelCatalog {
    models: Vec<KnownModelSpec>,
}

impl KnownModelCatalog {
    /// Builds an empty known-model catalog.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a provider's known models to the catalog.
    pub fn with_models(mut self, models: impl IntoIterator<Item = KnownModelSpec>) -> Self {
        self.models.extend(models);
        self
    }

    /// Returns all known model specs in insertion order.
    pub fn all(&self) -> &[KnownModelSpec] {
        &self.models
    }

    /// Finds one known provider/catalog-model pair.
    pub fn find(&self, provider: ProviderKind, model_id: &str) -> Option<&KnownModelSpec> {
        self.models
            .iter()
            .find(|model| model.provider == provider && model.id == model_id)
    }

    /// Returns true when the catalog contains the provider/catalog-model pair.
    pub fn contains(&self, provider: ProviderKind, model_id: &str) -> bool {
        self.find(provider, model_id).is_some()
    }
}

const MOCK_FEATURES: &[ModelFeature] = &[
    ModelFeature::ToolCalling,
    ModelFeature::StructuredOutput,
    ModelFeature::Reasoning,
];

/// Returns the known model metadata used by local development and tests.
pub fn known_mock_models() -> Vec<KnownModelSpec> {
    vec![KnownModelSpec {
        provider: ProviderKind::Mock,
        id: "mock",
        provider_model_id: "mock",
        context_window_tokens: 128_000,
        max_output_tokens: 128_000,
        intelligence_score: IntelligenceScore::Five,
        speed: SpeedTier::VeryFast,
        cost: CostTier::Low,
        thinking_level: ThinkingLevel::Disabled,
        features: MOCK_FEATURES,
    }]
}
