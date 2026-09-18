//! TypeSafe catalog contract tests.

use ai_models_core::{
    CostTier, IntelligenceScore, ModelFeature, ProviderKind, SpeedTier, ThinkingLevel,
};

use super::{JEV_1_13_0, JEV_LATEST, known_models};

#[test]
fn catalog_exposes_exact_jev_models() {
    let models = known_models();

    assert_eq!(
        models.iter().map(|model| model.id).collect::<Vec<_>>(),
        vec![JEV_LATEST, JEV_1_13_0]
    );
    assert_eq!(
        models
            .iter()
            .map(|model| model.provider_model_id)
            .collect::<Vec<_>>(),
        vec![JEV_LATEST, JEV_1_13_0]
    );
}

#[test]
fn catalog_entries_have_the_typesafe_judgment_contract() {
    for model in known_models() {
        assert_eq!(model.provider, ProviderKind::TypeSafe);
        assert_eq!(model.context_window_tokens, 64_000);
        assert_eq!(model.intelligence_score, IntelligenceScore::Five);
        assert_eq!(model.speed, SpeedTier::VeryFast);
        assert_eq!(model.cost, CostTier::Low);
        assert_eq!(model.thinking_level, ThinkingLevel::Disabled);
        assert_eq!(model.features, [ModelFeature::Judgment]);
    }
}
