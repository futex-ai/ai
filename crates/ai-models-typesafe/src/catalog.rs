//! Known TypeSafe model metadata.

use ai_models_core::{
    CostTier, IntelligenceScore, KnownModelSpec, ModelFeature, ProviderKind, SpeedTier,
    ThinkingLevel,
};

/// TypeSafe alias for the newest stable Jev release.
pub const JEV_LATEST: &str = "jev-latest";

/// TypeSafe's pinned Jev 1.13.0 release.
pub const JEV_1_13_0: &str = "jev-1.13.0";

const JEV_FEATURES: &[ModelFeature] = &[ModelFeature::Judgment];

/// Returns TypeSafe models known to this provider crate.
pub fn known_models() -> Vec<KnownModelSpec> {
    vec![jev_spec(JEV_LATEST), jev_spec(JEV_1_13_0)]
}

fn jev_spec(id: &'static str) -> KnownModelSpec {
    KnownModelSpec {
        provider: ProviderKind::TypeSafe,
        id,
        provider_model_id: id,
        context_window_tokens: 64_000,
        intelligence_score: IntelligenceScore::Five,
        speed: SpeedTier::VeryFast,
        cost: CostTier::Low,
        thinking_level: ThinkingLevel::Disabled,
        features: JEV_FEATURES,
    }
}

#[cfg(test)]
#[path = "_tests_/catalog_tests.rs"]
mod catalog_tests;
