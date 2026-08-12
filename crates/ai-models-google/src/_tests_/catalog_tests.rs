//! Google catalog tests.

use ai_models_core::{ModelFeature, ProviderKind, ThinkingLevel};

use super::{
    GEMINI_2_5_FLASH, GEMINI_2_5_FLASH_LITE, GEMINI_2_5_PRO, GEMINI_2_5_PRO_THINKING_HIGH,
    GEMINI_2_5_PRO_THINKING_MAX, GEMINI_3_1_FLASH_IMAGE, GEMINI_3_5_FLASH_LITE, GEMINI_3_6_FLASH,
    GEMINI_3_6_FLASH_THINKING_HIGH, known_models,
};

#[test]
fn gemini_3_6_flash_is_the_first_catalog_model() {
    let models = known_models();

    assert_eq!(models.first().map(|model| model.id), Some(GEMINI_3_6_FLASH));
    for legacy_id in [
        GEMINI_2_5_PRO,
        GEMINI_2_5_PRO_THINKING_HIGH,
        GEMINI_2_5_PRO_THINKING_MAX,
        GEMINI_2_5_FLASH,
        GEMINI_2_5_FLASH_LITE,
    ] {
        assert!(models.iter().any(|model| model.id == legacy_id));
    }
}

#[test]
fn gemini_3_1_flash_image_is_routable_for_image_generation() {
    let models = known_models();
    let model = models
        .iter()
        .find(|model| model.id == GEMINI_3_1_FLASH_IMAGE)
        .expect("Gemini image model should exist");

    assert_eq!(model.provider, ProviderKind::Google);
    assert_eq!(model.provider_model_id, GEMINI_3_1_FLASH_IMAGE);
    assert_eq!(model.context_window_tokens, 131_072);
    assert!(model.has_feature(ModelFeature::ImageGeneration));
    assert!(model.has_feature(ModelFeature::Vision));
    assert!(!model.has_feature(ModelFeature::ToolCalling));
}

#[test]
fn latest_gemini_models_have_current_metadata() {
    let models = known_models();

    for model_id in [GEMINI_3_6_FLASH, GEMINI_3_5_FLASH_LITE] {
        let model = models
            .iter()
            .find(|model| model.id == model_id)
            .expect("latest Gemini model should exist");
        assert_eq!(model.provider, ProviderKind::Google);
        assert_eq!(model.context_window_tokens, 1_048_576);
        assert!(model.has_feature(ModelFeature::Reasoning));
        assert!(model.has_feature(ModelFeature::Vision));
    }

    let high = models
        .iter()
        .find(|model| model.id == GEMINI_3_6_FLASH_THINKING_HIGH)
        .expect("Gemini 3.6 high-thinking model should exist");
    assert_eq!(high.provider_model_id, GEMINI_3_6_FLASH);
    assert_eq!(high.thinking_level, ThinkingLevel::High);
}
