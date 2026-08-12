//! xAI catalog tests.

use ai_models_core::{ModelFeature, ProviderKind, ThinkingLevel};

use super::{
    GROK_4_5, GROK_4_5_THINKING_LOW, GROK_4_5_THINKING_MEDIUM, GROK_4_20, GROK_4_20_REASONING,
    known_models,
};

#[test]
fn grok_4_5_is_the_first_catalog_model() {
    let models = known_models();

    assert_eq!(models.first().map(|model| model.id), Some(GROK_4_5));
    for legacy_id in [GROK_4_20_REASONING, GROK_4_20] {
        assert!(models.iter().any(|model| model.id == legacy_id));
    }
}

#[test]
fn catalog_excludes_unavailable_grok_4_20_aliases() {
    let model_ids = known_models()
        .into_iter()
        .map(|model| model.id)
        .collect::<Vec<_>>();

    assert!(!model_ids.contains(&"grok-4.20-mini"));
    assert!(!model_ids.contains(&"grok-4.20-thinking-high"));
}

#[test]
fn grok_4_5_variants_have_current_metadata() {
    let models = known_models();

    for (model_id, thinking_level) in [
        (GROK_4_5, ThinkingLevel::High),
        (GROK_4_5_THINKING_LOW, ThinkingLevel::Low),
        (GROK_4_5_THINKING_MEDIUM, ThinkingLevel::Medium),
    ] {
        let model = models
            .iter()
            .find(|model| model.id == model_id)
            .expect("Grok 4.5 model should exist");
        assert_eq!(model.provider, ProviderKind::Xai);
        assert_eq!(model.provider_model_id, GROK_4_5);
        assert_eq!(model.context_window_tokens, 500_000);
        assert_eq!(model.thinking_level, thinking_level);
        assert!(model.has_feature(ModelFeature::Reasoning));
        assert!(model.has_feature(ModelFeature::Vision));
    }
}

#[test]
fn grok_primary_models_advertise_vision() {
    let models = known_models();
    for model_id in [GROK_4_20_REASONING, GROK_4_20] {
        let model = models
            .iter()
            .find(|model| model.id == model_id)
            .expect("model should exist");
        assert!(model.has_feature(ModelFeature::Vision));
        assert_eq!(model.context_window_tokens, 1_000_000);
    }
}
