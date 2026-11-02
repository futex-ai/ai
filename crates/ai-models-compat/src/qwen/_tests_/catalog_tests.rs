//! Qwen catalog contract tests.

use ai_models_core::{
    CostTier, IntelligenceScore, ModelFeature, ProviderKind, SpeedTier, ThinkingLevel,
};

use super::{
    QWEN_3_7_FLASH, QWEN_3_7_FLASH_THINKING_DISABLED, QWEN_3_7_MAX, QWEN_3_7_MAX_THINKING_DISABLED,
    QWEN_3_7_PLUS, QWEN_3_7_PLUS_THINKING_DISABLED, known_models,
};

#[test]
fn catalog_exposes_exact_stable_qwen_variants() {
    let models = known_models();

    assert_eq!(
        models.iter().map(|model| model.id).collect::<Vec<_>>(),
        vec![
            QWEN_3_7_MAX,
            QWEN_3_7_MAX_THINKING_DISABLED,
            QWEN_3_7_PLUS,
            QWEN_3_7_PLUS_THINKING_DISABLED,
            QWEN_3_7_FLASH,
            QWEN_3_7_FLASH_THINKING_DISABLED,
        ]
    );
    assert_eq!(
        models
            .iter()
            .map(|model| model.thinking_level)
            .collect::<Vec<_>>(),
        vec![
            ThinkingLevel::High,
            ThinkingLevel::Disabled,
            ThinkingLevel::High,
            ThinkingLevel::Disabled,
            ThinkingLevel::High,
            ThinkingLevel::Disabled,
        ]
    );
}

#[test]
fn catalog_matches_qwen_context_and_capability_contract() {
    let models = known_models();

    for model in &models {
        assert_eq!(model.provider, ProviderKind::Qwen);
        assert_eq!(model.context_window_tokens, 1_000_000);
        assert!(model.has_feature(ModelFeature::ToolCalling));
        assert!(model.has_feature(ModelFeature::StructuredOutput));
        assert!(model.has_feature(ModelFeature::LongContext));
        assert_eq!(
            model.has_feature(ModelFeature::Reasoning),
            model.thinking_level == ThinkingLevel::High
        );
    }
    for model in &models[..2] {
        assert!(!model.has_feature(ModelFeature::Vision));
    }
    for model in &models[2..] {
        assert!(model.has_feature(ModelFeature::Vision));
    }
}

#[test]
fn catalog_matches_routing_metadata_and_provider_ids() {
    let models = known_models();

    for model in &models[..2] {
        assert_eq!(model.provider_model_id, QWEN_3_7_MAX);
        assert_eq!(model.intelligence_score, IntelligenceScore::Ten);
        assert_eq!(model.cost, CostTier::Premium);
    }
    assert_eq!(models[0].speed, SpeedTier::Slow);
    assert_eq!(models[1].speed, SpeedTier::Medium);

    for model in &models[2..4] {
        assert_eq!(model.provider_model_id, QWEN_3_7_PLUS);
        assert_eq!(model.intelligence_score, IntelligenceScore::Nine);
        assert_eq!(model.cost, CostTier::Medium);
    }
    assert_eq!(models[2].speed, SpeedTier::Medium);
    assert_eq!(models[3].speed, SpeedTier::Fast);

    for model in &models[4..] {
        assert_eq!(model.provider_model_id, QWEN_3_7_FLASH);
        assert_eq!(model.intelligence_score, IntelligenceScore::Eight);
        assert_eq!(model.cost, CostTier::Low);
    }
    assert_eq!(models[4].speed, SpeedTier::Fast);
    assert_eq!(models[5].speed, SpeedTier::VeryFast);
}
