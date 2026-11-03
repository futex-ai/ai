//! Catalog fixtures for judgment runner tests.

use ai_interface::ProviderKind;
use ai_models_core::{
    CostTier, IntelligenceScore, KnownModelSpec, ModelFeature, SpeedTier, ThinkingLevel,
};

const JUDGMENT_FEATURES: &[ModelFeature] = &[ModelFeature::Judgment];

pub(super) fn judgment_spec(
    provider: ProviderKind,
    id: &'static str,
    provider_model_id: &'static str,
) -> KnownModelSpec {
    KnownModelSpec {
        provider,
        id,
        provider_model_id,
        context_window_tokens: 0,
        max_output_tokens: 0,
        intelligence_score: IntelligenceScore::Five,
        speed: SpeedTier::VeryFast,
        cost: CostTier::Low,
        thinking_level: ThinkingLevel::Disabled,
        features: JUDGMENT_FEATURES,
    }
}

pub(super) fn mock_spec() -> KnownModelSpec {
    judgment_spec(ProviderKind::TypeSafe, "mock-judgment", "mock-judgment")
}
