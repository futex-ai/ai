//! Anthropic catalog tests.

use ai_models_core::{ModelFeature, ProviderKind, ThinkingLevel};

use super::{
    CLAUDE_FABLE_5, CLAUDE_HAIKU_4_5, CLAUDE_OPUS_4_7, CLAUDE_OPUS_4_7_THINKING_MAX, CLAUDE_OPUS_5,
    CLAUDE_OPUS_5_THINKING_MAX, CLAUDE_SONNET_4_6, CLAUDE_SONNET_5, known_models,
};

#[test]
fn claude_sonnet_5_is_the_first_catalog_model() {
    let models = known_models();

    assert_eq!(models.first().map(|model| model.id), Some(CLAUDE_SONNET_5));
    for legacy_id in [
        CLAUDE_SONNET_4_6,
        CLAUDE_OPUS_4_7,
        CLAUDE_OPUS_4_7_THINKING_MAX,
        CLAUDE_HAIKU_4_5,
    ] {
        assert!(models.iter().any(|model| model.id == legacy_id));
    }
}

#[test]
fn claude_5_family_has_current_metadata() {
    let models = known_models();

    for model_id in [CLAUDE_SONNET_5, CLAUDE_OPUS_5, CLAUDE_FABLE_5] {
        let model = models
            .iter()
            .find(|model| model.id == model_id)
            .expect("Claude 5 model should exist");
        assert_eq!(model.provider, ProviderKind::Anthropic);
        assert_eq!(model.context_window_tokens, 1_000_000);
        assert!(model.has_feature(ModelFeature::Reasoning));
        assert!(model.has_feature(ModelFeature::Vision));
    }

    let max = models
        .iter()
        .find(|model| model.id == CLAUDE_OPUS_5_THINKING_MAX)
        .expect("Claude Opus 5 max-thinking model should exist");
    assert_eq!(max.provider_model_id, CLAUDE_OPUS_5);
    assert_eq!(max.thinking_level, ThinkingLevel::Max);
}
