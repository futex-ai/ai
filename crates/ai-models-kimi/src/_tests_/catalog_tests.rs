//! Kimi catalog contract tests.

use ai_models_core::{
    CostTier, IntelligenceScore, ModelFeature, ProviderKind, SpeedTier, ThinkingLevel,
};

use super::{KIMI_K3, KIMI_K3_THINKING_HIGH, KIMI_K3_THINKING_LOW, known_models};

#[test]
fn catalog_exposes_exact_kimi_k3_variants() {
    let models = known_models();

    assert_eq!(
        models.iter().map(|model| model.id).collect::<Vec<_>>(),
        vec![KIMI_K3, KIMI_K3_THINKING_HIGH, KIMI_K3_THINKING_LOW]
    );
    assert_eq!(
        models
            .iter()
            .map(|model| model.thinking_level)
            .collect::<Vec<_>>(),
        vec![ThinkingLevel::Max, ThinkingLevel::High, ThinkingLevel::Low]
    );
}

#[test]
fn catalog_variants_share_k3_provider_contract() {
    let expected_features = [
        ModelFeature::ToolCalling,
        ModelFeature::StructuredOutput,
        ModelFeature::Vision,
        ModelFeature::LongContext,
        ModelFeature::Reasoning,
    ];

    for model in known_models() {
        assert_eq!(model.provider, ProviderKind::Kimi);
        assert_eq!(model.provider_model_id, KIMI_K3);
        assert_eq!(model.context_window_tokens, 1_000_000);
        assert_eq!(model.intelligence_score, IntelligenceScore::Ten);
        assert_eq!(model.features, expected_features);
    }
}

#[test]
fn catalog_variants_have_specified_speed_and_cost_tiers() {
    let models = known_models();

    assert_eq!(models[0].speed, SpeedTier::Slow);
    assert_eq!(models[0].cost, CostTier::Premium);
    assert_eq!(models[1].speed, SpeedTier::Medium);
    assert_eq!(models[1].cost, CostTier::Premium);
    assert_eq!(models[2].speed, SpeedTier::Fast);
    assert_eq!(models[2].cost, CostTier::High);
}
