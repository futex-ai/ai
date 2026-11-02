//! OpenAI catalog tests.

use ai_interface::{Effort, ModelFeature};

use super::{
    GPT_5_4_MINI, GPT_5_4_NANO, GPT_5_5, GPT_5_6_LUNA, GPT_5_6_SOL, GPT_5_6_TERRA, GPT_IMAGE_2,
    PROVIDER_ID, SORA_2, known_models,
};

#[test]
fn gpt_5_6_sol_is_the_first_catalog_model() {
    let models = known_models();

    assert_eq!(models.first(), Some(&GPT_5_6_SOL));
    for legacy in [GPT_5_5, GPT_5_4_MINI, GPT_5_4_NANO] {
        assert!(models.iter().any(|model| model == &legacy));
    }
}

#[test]
fn sora_2_is_routable_for_video_generation() {
    let models = known_models();
    let model = models
        .iter()
        .find(|model| model.provider_model_id == SORA_2.provider_model_id)
        .expect("Sora 2 should exist");

    assert_eq!(model.provider, PROVIDER_ID);
    assert_eq!(model.provider_model_id, "sora-2");
    assert_eq!(model.context_window_tokens, 0);
    assert!(model.has_feature(ModelFeature::VideoGeneration));
    assert!(model.has_feature(ModelFeature::Vision));
    assert!(!model.has_feature(ModelFeature::ImageGeneration));
}

#[test]
fn cost_optimized_models_use_supported_gpt_5_4_ids() {
    let models = known_models();
    let model_ids = models
        .iter()
        .map(|model| model.provider_model_id.as_ref())
        .collect::<Vec<_>>();

    assert!(model_ids.contains(&"gpt-5.4-mini"));
    assert!(model_ids.contains(&"gpt-5.4-nano"));
    assert!(!model_ids.contains(&"gpt-5.5-mini"));
    assert!(!model_ids.contains(&"gpt-5.5-nano"));
}

#[test]
fn cost_optimized_models_have_current_metadata() {
    let models = known_models();

    for spec in [GPT_5_4_MINI, GPT_5_4_NANO] {
        let model = models
            .iter()
            .find(|model| model.provider_model_id == spec.provider_model_id)
            .expect("GPT-5.4 cost-optimized model should exist");
        assert_eq!(model.context_window_tokens, 400_000);
        assert!(model.has_feature(ModelFeature::Reasoning));
        assert!(model.has_feature(ModelFeature::Vision));
    }
}

#[test]
fn gpt_5_6_family_has_current_metadata() {
    let models = known_models();

    for spec in [GPT_5_6_SOL, GPT_5_6_TERRA, GPT_5_6_LUNA] {
        let model = models
            .iter()
            .find(|model| model.provider_model_id == spec.provider_model_id)
            .expect("GPT-5.6 model should exist");
        assert_eq!(model.provider, PROVIDER_ID);
        assert_eq!(model.context_window_tokens, 1_050_000);
        assert!(model.has_feature(ModelFeature::Reasoning));
        assert!(model.has_feature(ModelFeature::Vision));
    }

    let sol = models
        .iter()
        .find(|model| model.provider_model_id == GPT_5_6_SOL.provider_model_id)
        .expect("GPT-5.6 Sol model should exist");
    assert_eq!(sol.provider_model_id, "gpt-5.6-sol");
    assert!(sol.efforts.contains(Effort::Max));
}

#[test]
fn gpt_image_2_is_routable_for_image_generation() {
    let models = known_models();
    let model = models
        .iter()
        .find(|model| model.provider_model_id == GPT_IMAGE_2.provider_model_id)
        .expect("GPT Image 2 should exist");

    assert_eq!(model.provider, PROVIDER_ID);
    assert_eq!(model.provider_model_id, "gpt-image-2");
    assert_eq!(model.context_window_tokens, 0);
    assert!(model.has_feature(ModelFeature::ImageGeneration));
    assert!(model.has_feature(ModelFeature::Vision));
}
