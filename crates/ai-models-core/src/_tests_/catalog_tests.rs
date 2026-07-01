use ai_interface::{ModelFeature, ProviderKind};

use crate::{
    IntelligenceScore, KnownModelCatalog, KnownModelSpec, ThinkingLevel, known_mock_models,
};

#[test]
fn mock_catalog_contains_valid_development_model() {
    let catalog = KnownModelCatalog::new().with_models(known_mock_models());

    let model = catalog
        .find(ProviderKind::Mock, "mock")
        .expect("mock model should be present");

    assert_eq!(model.intelligence_score.value(), 5);
    assert_eq!(model.provider_model_id, "mock");
    assert_eq!(model.thinking_level, ThinkingLevel::Disabled);
    assert!(model.context_window_tokens > 0);
    assert!(model.has_feature(ModelFeature::ToolCalling));
}

#[test]
fn intelligence_scores_are_strongly_typed_one_to_ten() {
    assert_eq!(IntelligenceScore::One.value(), 1);
    assert_eq!(IntelligenceScore::Ten.value(), 10);
}

#[test]
fn catalog_allows_distinct_variants_for_one_provider_model() {
    let catalog = KnownModelCatalog::new().with_models([
        KnownModelSpec {
            provider: ProviderKind::OpenAi,
            id: "gpt-5.5-thinking-low",
            provider_model_id: "gpt-5.5",
            context_window_tokens: 400_000,
            intelligence_score: IntelligenceScore::Ten,
            speed: crate::SpeedTier::Medium,
            cost: crate::CostTier::Premium,
            thinking_level: ThinkingLevel::Low,
            features: &[ModelFeature::Reasoning],
        },
        KnownModelSpec {
            provider: ProviderKind::OpenAi,
            id: "gpt-5.5-thinking-extra-high",
            provider_model_id: "gpt-5.5",
            context_window_tokens: 400_000,
            intelligence_score: IntelligenceScore::Ten,
            speed: crate::SpeedTier::Slow,
            cost: crate::CostTier::Premium,
            thinking_level: ThinkingLevel::ExtraHigh,
            features: &[ModelFeature::Reasoning],
        },
    ]);

    let low = catalog
        .find(ProviderKind::OpenAi, "gpt-5.5-thinking-low")
        .expect("low thinking variant should exist");
    let extra_high = catalog
        .find(ProviderKind::OpenAi, "gpt-5.5-thinking-extra-high")
        .expect("extra-high thinking variant should exist");

    assert_eq!(low.provider_model_id, extra_high.provider_model_id);
    assert_ne!(low.id, extra_high.id);
    assert_ne!(low.thinking_level, extra_high.thinking_level);
}

#[test]
fn thinking_levels_have_stable_log_values() {
    assert_eq!(ThinkingLevel::Disabled.as_str(), "disabled");
    assert_eq!(ThinkingLevel::Low.as_str(), "low");
    assert_eq!(ThinkingLevel::Medium.as_str(), "medium");
    assert_eq!(ThinkingLevel::High.as_str(), "high");
    assert_eq!(ThinkingLevel::ExtraHigh.as_str(), "extra_high");
    assert_eq!(ThinkingLevel::Max.as_str(), "max");
}
