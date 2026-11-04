//! MiniMax catalog contract tests.

use ai_interface::{
    CostTier, Effort, IntelligenceScore, ModelFeature, ModelSpec, ProviderId, SpeedTier,
};
use serde_json::json;

use super::{PROVIDER_ID, known_models};

#[test]
fn catalog_matches_supported_minimax_models() {
    assert_eq!(PROVIDER_ID.as_str(), "minimax");
    assert_eq!(PROVIDER_ID.to_string(), "minimax");
    assert_eq!(serde_json::to_value(PROVIDER_ID).unwrap(), json!("minimax"));
    assert_eq!(
        serde_json::from_value::<ProviderId>(json!("minimax")).unwrap(),
        PROVIDER_ID
    );
    let alias = serde_json::from_value::<ProviderId>(json!("mini_max")).unwrap();
    assert_ne!(alias, PROVIDER_ID);
    assert_eq!(alias.as_str(), "mini_max");

    let models = known_models();

    assert_eq!(models.len(), 3);
    assert_model(
        &models,
        ExpectedModel {
            provider_model_id: "MiniMax-M3",
            context_window_tokens: 1_000_000,
            intelligence_score: IntelligenceScore::Nine,
            speed: SpeedTier::Medium,
            cost: CostTier::Low,
            default_effort: Some(Effort::Medium),
            features: &[
                ModelFeature::ToolCalling,
                ModelFeature::StructuredOutput,
                ModelFeature::Vision,
                ModelFeature::VideoInput,
                ModelFeature::LongContext,
                ModelFeature::Reasoning,
            ],
        },
    );
    assert_model(
        &models,
        ExpectedModel {
            provider_model_id: "MiniMax-M2.7",
            context_window_tokens: 204_800,
            intelligence_score: IntelligenceScore::Eight,
            speed: SpeedTier::Medium,
            cost: CostTier::Low,
            default_effort: Some(Effort::Medium),
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
            provider_model_id: "MiniMax-M2.7-highspeed",
            context_window_tokens: 204_800,
            intelligence_score: IntelligenceScore::Eight,
            speed: SpeedTier::Fast,
            cost: CostTier::Medium,
            default_effort: Some(Effort::Medium),
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
        let should_have_vision = model.provider_model_id == "MiniMax-M3";
        assert_eq!(
            model.has_feature(ModelFeature::Vision),
            should_have_vision,
            "unexpected vision capability for {}",
            model.provider_model_id
        );
    }
}

#[test]
fn video_input_is_advertised_only_by_m3_variants() {
    let models = known_models();
    for model in models {
        let should_have_video = model.provider_model_id == "MiniMax-M3";
        assert_eq!(
            model.has_feature(ModelFeature::VideoInput),
            should_have_video,
            "unexpected video capability for {}",
            model.provider_model_id
        );
    }
}

struct ExpectedModel {
    provider_model_id: &'static str,
    context_window_tokens: u32,
    intelligence_score: IntelligenceScore,
    speed: SpeedTier,
    cost: CostTier,
    default_effort: Option<Effort>,
    features: &'static [ModelFeature],
}

fn assert_model(models: &[ModelSpec], expected: ExpectedModel) {
    let model = models
        .iter()
        .find(|model| model.provider_model_id == expected.provider_model_id)
        .expect("catalog model should exist");

    assert_eq!(model.provider, PROVIDER_ID);
    assert_eq!(model.provider_model_id, expected.provider_model_id);
    assert_eq!(model.context_window_tokens, expected.context_window_tokens);
    assert_eq!(
        model.routing.expect("routing tiers").intelligence,
        expected.intelligence_score
    );
    assert_eq!(model.routing.expect("routing tiers").speed, expected.speed);
    assert_eq!(model.routing.expect("routing tiers").cost, expected.cost);
    assert_eq!(model.default_effort, expected.default_effort);
    assert_eq!(model.features.as_ref(), expected.features);
}
