use ai_interface::{ModelFeature, ProviderKind};

use crate::{
    IntelligenceScore, KnownModelCatalog, KnownModelSpec, ThinkingLevel, known_mock_models,
    resolve_catalog_thinking_level,
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

#[test]
fn resolves_to_the_highest_catalog_thinking_level_not_above_the_request() {
    let models = [
        known_model("provider-a-low", "provider-a", ThinkingLevel::Low),
        known_model("provider-a-high", "provider-a", ThinkingLevel::High),
        known_model(
            "provider-a-extra-high",
            "provider-a",
            ThinkingLevel::ExtraHigh,
        ),
        known_model("provider-b-high", "provider-b", ThinkingLevel::High),
    ];

    assert_eq!(
        resolve_catalog_thinking_level(
            &models,
            ProviderKind::OpenAi,
            "provider-a",
            ThinkingLevel::Max,
        ),
        Some(ThinkingLevel::ExtraHigh)
    );
    assert_eq!(
        resolve_catalog_thinking_level(
            &models,
            ProviderKind::OpenAi,
            "provider-a",
            ThinkingLevel::High,
        ),
        Some(ThinkingLevel::High)
    );
    assert_eq!(
        resolve_catalog_thinking_level(
            &models,
            ProviderKind::OpenAi,
            "provider-a",
            ThinkingLevel::Medium,
        ),
        Some(ThinkingLevel::Low)
    );
    assert_eq!(
        resolve_catalog_thinking_level(
            &models,
            ProviderKind::OpenAi,
            "provider-a",
            ThinkingLevel::Disabled,
        ),
        None
    );
    assert_eq!(
        resolve_catalog_thinking_level(
            &models,
            ProviderKind::OpenAi,
            "unknown",
            ThinkingLevel::Max,
        ),
        None
    );
    assert_eq!(
        resolve_catalog_thinking_level(
            &models,
            ProviderKind::Anthropic,
            "provider-a",
            ThinkingLevel::Max,
        ),
        None
    );
}

#[test]
fn only_entries_without_specialized_features_support_chat() {
    let specialized: [&'static [ModelFeature]; 4] = [
        &[ModelFeature::Judgment],
        &[ModelFeature::ImageGeneration, ModelFeature::Vision],
        &[ModelFeature::VideoGeneration, ModelFeature::Vision],
        &[ModelFeature::ToolCalling, ModelFeature::Judgment],
    ];
    for features in specialized {
        let model = known_model_with_features("specialized", "id", ThinkingLevel::Low, features);
        assert!(!model.supports_chat(), "{features:?}");
    }

    let chat: [&'static [ModelFeature]; 3] = [
        &[],
        &[ModelFeature::Vision],
        &[
            ModelFeature::ToolCalling,
            ModelFeature::StructuredOutput,
            ModelFeature::Vision,
            ModelFeature::VideoInput,
            ModelFeature::LongContext,
            ModelFeature::Reasoning,
        ],
    ];
    for features in chat {
        let model = known_model_with_features("chat", "id", ThinkingLevel::Low, features);
        assert!(model.supports_chat(), "{features:?}");
    }
    assert!(
        known_mock_models()
            .iter()
            .all(KnownModelSpec::supports_chat)
    );
}

#[test]
fn resolver_ignores_a_provider_model_id_that_only_specialized_entries_use() {
    let models = [known_model_with_features(
        "provider-a-judgment",
        "provider-a",
        ThinkingLevel::Disabled,
        &[ModelFeature::Judgment],
    )];

    for requested in [
        ThinkingLevel::Disabled,
        ThinkingLevel::High,
        ThinkingLevel::Max,
    ] {
        assert_eq!(
            resolve_catalog_thinking_level(&models, ProviderKind::OpenAi, "provider-a", requested),
            None,
            "requested {requested:?}"
        );
    }
}

#[test]
fn resolver_uses_only_chat_entries_when_specialized_entries_share_the_id() {
    let models = [
        known_model_with_features(
            "provider-a-judgment",
            "provider-a",
            ThinkingLevel::Disabled,
            &[ModelFeature::Judgment],
        ),
        known_model_with_features(
            "provider-a-image",
            "provider-a",
            ThinkingLevel::Low,
            &[ModelFeature::ImageGeneration, ModelFeature::Vision],
        ),
        known_model_with_features(
            "provider-a-video",
            "provider-a",
            ThinkingLevel::Low,
            &[ModelFeature::VideoGeneration, ModelFeature::Vision],
        ),
        known_model("provider-a-medium", "provider-a", ThinkingLevel::Medium),
        known_model("provider-a-high", "provider-a", ThinkingLevel::High),
    ];
    let resolve = |requested| {
        resolve_catalog_thinking_level(&models, ProviderKind::OpenAi, "provider-a", requested)
    };

    assert_eq!(resolve(ThinkingLevel::Max), Some(ThinkingLevel::High));
    assert_eq!(resolve(ThinkingLevel::High), Some(ThinkingLevel::High));
    assert_eq!(resolve(ThinkingLevel::Medium), Some(ThinkingLevel::Medium));
    assert_eq!(resolve(ThinkingLevel::Low), None);
    assert_eq!(resolve(ThinkingLevel::Disabled), None);
}

fn known_model(
    id: &'static str,
    provider_model_id: &'static str,
    thinking_level: ThinkingLevel,
) -> KnownModelSpec {
    known_model_with_features(
        id,
        provider_model_id,
        thinking_level,
        &[ModelFeature::Reasoning],
    )
}

fn known_model_with_features(
    id: &'static str,
    provider_model_id: &'static str,
    thinking_level: ThinkingLevel,
    features: &'static [ModelFeature],
) -> KnownModelSpec {
    KnownModelSpec {
        provider: ProviderKind::OpenAi,
        id,
        provider_model_id,
        context_window_tokens: 128_000,
        intelligence_score: IntelligenceScore::Five,
        speed: crate::SpeedTier::Medium,
        cost: crate::CostTier::Medium,
        thinking_level,
        features,
    }
}
