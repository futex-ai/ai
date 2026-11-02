//! Google catalog tests.

use ai_interface::{Effort, ModelFeature, ProviderId};
use serde_json::json;

use super::{
    GEMINI_3_1_FLASH_IMAGE, GEMINI_3_5_FLASH_LITE, GEMINI_3_6_FLASH, PROVIDER_ID,
    VEO_3_1_GENERATE_PREVIEW, known_models,
};

#[test]
fn gemini_3_6_flash_is_the_first_catalog_model() {
    assert_eq!(PROVIDER_ID.as_str(), "google");
    assert_eq!(PROVIDER_ID.to_string(), "google");
    assert_eq!(serde_json::to_value(PROVIDER_ID).unwrap(), json!("google"));
    assert_eq!(
        serde_json::from_value::<ProviderId>(json!("google")).unwrap(),
        PROVIDER_ID
    );

    let models = known_models();

    assert_eq!(models.first(), Some(&GEMINI_3_6_FLASH));
}

#[test]
fn veo_3_1_is_routable_for_video_generation() {
    let models = known_models();
    let model = models
        .iter()
        .find(|model| model.provider_model_id == VEO_3_1_GENERATE_PREVIEW.provider_model_id)
        .expect("Veo 3.1 should exist");

    assert_eq!(model.provider, PROVIDER_ID);
    assert_eq!(model.provider_model_id, "veo-3.1-generate-preview");
    assert_eq!(model.context_window_tokens, 1_024);
    assert!(model.has_feature(ModelFeature::VideoGeneration));
    assert!(model.has_feature(ModelFeature::Vision));
    assert!(!model.has_feature(ModelFeature::ImageGeneration));
    assert!(!model.has_feature(ModelFeature::VideoInput));
}

#[test]
fn catalog_excludes_unavailable_gemini_2_5_models() {
    assert!(
        known_models()
            .iter()
            .all(|model| !model.provider_model_id.starts_with("gemini-2.5"))
    );
}

#[test]
fn gemini_3_1_flash_image_is_routable_for_image_generation() {
    let models = known_models();
    let model = models
        .iter()
        .find(|model| model.provider_model_id == GEMINI_3_1_FLASH_IMAGE.provider_model_id)
        .expect("Gemini image model should exist");

    assert_eq!(model.provider, PROVIDER_ID);
    assert_eq!(model.provider_model_id, "gemini-3.1-flash-image");
    assert_eq!(model.context_window_tokens, 131_072);
    assert!(model.has_feature(ModelFeature::ImageGeneration));
    assert!(model.has_feature(ModelFeature::Vision));
    assert!(!model.has_feature(ModelFeature::VideoInput));
    assert!(!model.has_feature(ModelFeature::ToolCalling));
}

#[test]
fn latest_gemini_models_have_current_metadata() {
    let models = known_models();

    for spec in [GEMINI_3_6_FLASH, GEMINI_3_5_FLASH_LITE] {
        let model = models
            .iter()
            .find(|model| model.provider_model_id == spec.provider_model_id)
            .expect("latest Gemini model should exist");
        assert_eq!(model.provider, PROVIDER_ID);
        assert_eq!(model.context_window_tokens, 1_048_576);
        assert!(model.has_feature(ModelFeature::Reasoning));
        assert!(model.has_feature(ModelFeature::Vision));
        assert!(model.has_feature(ModelFeature::VideoInput));
    }

    let flash = models
        .iter()
        .find(|model| model.provider_model_id == GEMINI_3_6_FLASH.provider_model_id)
        .expect("Gemini 3.6 model should exist");
    assert_eq!(flash.provider_model_id, "gemini-3.6-flash");
    assert!(flash.efforts.contains(Effort::High));
}
