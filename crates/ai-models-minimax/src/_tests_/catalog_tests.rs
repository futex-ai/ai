//! MiniMax catalog contract tests.

use ai_models_core::{
    CostTier, IntelligenceScore, ModelFeature, ProviderKind, SpeedTier, ThinkingLevel,
};

use super::{
    MINIMAX_M2_7, MINIMAX_M2_7_HIGHSPEED, MINIMAX_M3, MINIMAX_M3_THINKING_DISABLED, known_models,
};

#[test]
fn catalog_matches_supported_minimax_models() {
    let models = known_models();

    assert_eq!(models.len(), 4);
    assert_model(
        &models,
        ExpectedModel {
            id: MINIMAX_M3,
            provider_model_id: "MiniMax-M3",
            context_window_tokens: 1_000_000,
            intelligence_score: IntelligenceScore::Nine,
            speed: SpeedTier::Medium,
            cost: CostTier::Low,
            thinking_level: ThinkingLevel::Medium,
            features: &[
                ModelFeature::ToolCalling,
                ModelFeature::StructuredOutput,
                ModelFeature::Vision,
                ModelFeature::LongContext,
                ModelFeature::Reasoning,
            ],
        },
    );
    assert_model(
        &models,
        ExpectedModel {
            id: MINIMAX_M3_THINKING_DISABLED,
            provider_model_id: "MiniMax-M3",
            context_window_tokens: 1_000_000,
            intelligence_score: IntelligenceScore::Nine,
            speed: SpeedTier::Fast,
            cost: CostTier::Low,
            thinking_level: ThinkingLevel::Disabled,
            features: &[
                ModelFeature::ToolCalling,
                ModelFeature::StructuredOutput,
                ModelFeature::Vision,
                ModelFeature::LongContext,
            ],
        },
    );
    assert_model(
        &models,
        ExpectedModel {
            id: MINIMAX_M2_7,
            provider_model_id: "MiniMax-M2.7",
            context_window_tokens: 204_800,
            intelligence_score: IntelligenceScore::Eight,
            speed: SpeedTier::Medium,
            cost: CostTier::Low,
            thinking_level: ThinkingLevel::Medium,
            features: &[
                ModelFeature::ToolCalling,
                ModelFeature::StructuredOutput,
                ModelFeature::LongContext,
                ModelFeature::Reasoning,
            ],
        },
    );
    assert_model(
        &models,
        ExpectedModel {
            id: MINIMAX_M2_7_HIGHSPEED,
            provider_model_id: "MiniMax-M2.7-highspeed",
            context_window_tokens: 204_800,
            intelligence_score: IntelligenceScore::Eight,
            speed: SpeedTier::Fast,
            cost: CostTier::Medium,
            thinking_level: ThinkingLevel::Medium,
            features: &[
                ModelFeature::ToolCalling,
                ModelFeature::StructuredOutput,
                ModelFeature::LongContext,
                ModelFeature::Reasoning,
            ],
        },
    );
}

#[test]
fn vision_is_advertised_only_by_m3_variants() {
    let models = known_models();
    for model in models {
        let should_have_vision = matches!(model.id, MINIMAX_M3 | MINIMAX_M3_THINKING_DISABLED);
        assert_eq!(
            model.has_feature(ModelFeature::Vision),
            should_have_vision,
            "unexpected vision capability for {}",
            model.id
        );
    }
}

struct ExpectedModel {
    id: &'static str,
    provider_model_id: &'static str,
    context_window_tokens: u32,
    intelligence_score: IntelligenceScore,
    speed: SpeedTier,
    cost: CostTier,
    thinking_level: ThinkingLevel,
    features: &'static [ModelFeature],
}

fn assert_model(models: &[ai_models_core::KnownModelSpec], expected: ExpectedModel) {
    let model = models
        .iter()
        .find(|model| model.id == expected.id)
        .expect("catalog model should exist");

    assert_eq!(model.provider, ProviderKind::MiniMax);
    assert_eq!(model.provider_model_id, expected.provider_model_id);
    assert_eq!(model.context_window_tokens, expected.context_window_tokens);
    assert_eq!(model.intelligence_score, expected.intelligence_score);
    assert_eq!(model.speed, expected.speed);
    assert_eq!(model.cost, expected.cost);
    assert_eq!(model.thinking_level, expected.thinking_level);
    assert_eq!(model.features, expected.features);
}
