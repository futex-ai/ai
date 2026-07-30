//! DeepSeek catalog contract tests.

use ai_models_core::{
    CostTier, IntelligenceScore, ModelFeature, ProviderKind, SpeedTier, ThinkingLevel,
};

use super::{
    DEEPSEEK_V4_FLASH, DEEPSEEK_V4_FLASH_THINKING_DISABLED, DEEPSEEK_V4_FLASH_THINKING_MAX,
    DEEPSEEK_V4_PRO, DEEPSEEK_V4_PRO_THINKING_DISABLED, DEEPSEEK_V4_PRO_THINKING_MAX, known_models,
};

#[test]
fn catalog_exposes_exact_current_deepseek_variants() {
    let models = known_models();

    assert_eq!(
        models.iter().map(|model| model.id).collect::<Vec<_>>(),
        vec![
            DEEPSEEK_V4_PRO,
            DEEPSEEK_V4_PRO_THINKING_MAX,
            DEEPSEEK_V4_PRO_THINKING_DISABLED,
            DEEPSEEK_V4_FLASH,
            DEEPSEEK_V4_FLASH_THINKING_MAX,
            DEEPSEEK_V4_FLASH_THINKING_DISABLED,
        ]
    );
    assert_eq!(
        models
            .iter()
            .map(|model| model.thinking_level)
            .collect::<Vec<_>>(),
        vec![
            ThinkingLevel::High,
            ThinkingLevel::Max,
            ThinkingLevel::Disabled,
            ThinkingLevel::High,
            ThinkingLevel::Max,
            ThinkingLevel::Disabled,
        ]
    );
}

#[test]
fn catalog_variants_match_provider_metadata_and_features() {
    let enabled_features = [
        ModelFeature::ToolCalling,
        ModelFeature::StructuredOutput,
        ModelFeature::LongContext,
        ModelFeature::Reasoning,
    ];
    let disabled_features = [
        ModelFeature::ToolCalling,
        ModelFeature::StructuredOutput,
        ModelFeature::LongContext,
    ];

    for model in known_models() {
        assert_eq!(model.provider, ProviderKind::DeepSeek);
        assert_eq!(model.context_window_tokens, 1_000_000);
        assert_eq!(model.cost, CostTier::Low);
        assert!(!model.features.contains(&ModelFeature::Vision));
        if model.thinking_level == ThinkingLevel::Disabled {
            assert_eq!(model.features, disabled_features);
        } else {
            assert_eq!(model.features, enabled_features);
        }
    }
}

#[test]
fn catalog_variants_match_intelligence_speed_and_provider_ids() {
    let models = known_models();

    for model in &models[..3] {
        assert_eq!(model.provider_model_id, DEEPSEEK_V4_PRO);
        assert_eq!(model.intelligence_score, IntelligenceScore::Ten);
    }
    assert_eq!(models[0].speed, SpeedTier::Medium);
    assert_eq!(models[1].speed, SpeedTier::Slow);
    assert_eq!(models[2].speed, SpeedTier::Fast);

    for model in &models[3..] {
        assert_eq!(model.provider_model_id, DEEPSEEK_V4_FLASH);
        assert_eq!(model.intelligence_score, IntelligenceScore::Nine);
    }
    assert_eq!(models[3].speed, SpeedTier::Fast);
    assert_eq!(models[4].speed, SpeedTier::Medium);
    assert_eq!(models[5].speed, SpeedTier::VeryFast);
}
