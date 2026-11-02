//! Routing requests retain portable text ids and ordered preferences.

use serde_json::json;

use crate::{
    MOCK_PROVIDER, ModelFeature, ModelPreference, ModelRequirement, ModelRouteRequest, ProviderId,
};

#[test]
fn default_route_uses_deployment_priority() {
    assert_eq!(
        ModelRouteRequest::default(),
        ModelRouteRequest {
            requirements: Vec::new(),
            preferences: vec![ModelPreference::DeploymentPriority],
        }
    );
}

#[test]
fn route_builder_preserves_requirement_and_preference_order() {
    let request = ModelRouteRequest::builder()
        .require(ModelRequirement::ModelId("mock/mock".to_owned()))
        .require(ModelRequirement::Provider(MOCK_PROVIDER))
        .require_feature(ModelFeature::StructuredOutput)
        .prefer(ModelPreference::Intelligence)
        .prefer(ModelPreference::LowCost)
        .build();

    assert_eq!(
        request.requirements,
        vec![
            ModelRequirement::ModelId("mock/mock".to_owned()),
            ModelRequirement::Provider(MOCK_PROVIDER),
            ModelRequirement::Feature(ModelFeature::StructuredOutput),
        ]
    );
    assert_eq!(
        request.preferences,
        vec![ModelPreference::Intelligence, ModelPreference::LowCost]
    );
}

#[test]
fn route_requirements_round_trip_text_ids_and_unlisted_provider_ids() {
    let request = ModelRouteRequest::builder()
        .require(ModelRequirement::ModelId(
            "unlisted-provider/organization/model".to_owned(),
        ))
        .require(ModelRequirement::Provider(ProviderId::from(
            "unlisted-provider".to_owned(),
        )))
        .build();
    let expected = json!({
        "requirements": [
            {"model_id": "unlisted-provider/organization/model"},
            {"provider": "unlisted-provider"}
        ],
        "preferences": []
    });

    assert_eq!(
        serde_json::to_value(&request).expect("route request should serialize"),
        expected
    );
    assert_eq!(
        serde_json::from_value::<ModelRouteRequest>(expected)
            .expect("route request should deserialize"),
        request
    );
}

#[test]
fn image_generation_feature_has_stable_config_display_and_serde_values() {
    let feature = ModelFeature::ImageGeneration;

    assert_eq!(feature.as_str(), "image_generation");
    assert_eq!(feature.to_string(), "image_generation");
    assert_eq!(
        serde_json::to_value(feature).unwrap(),
        json!("image_generation")
    );
    assert_eq!(
        serde_json::from_value::<ModelFeature>(json!("image_generation")).unwrap(),
        feature
    );
}

#[test]
fn judgment_feature_has_stable_config_display_and_serde_values() {
    let feature = ModelFeature::Judgment;

    assert_eq!(feature.as_str(), "judgment");
    assert_eq!(feature.to_string(), "judgment");
    assert_eq!(serde_json::to_value(feature).unwrap(), json!("judgment"));
    assert_eq!(
        serde_json::from_value::<ModelFeature>(json!("judgment")).unwrap(),
        feature
    );
}

#[test]
fn video_input_feature_has_stable_config_display_and_serde_values() {
    let feature = ModelFeature::VideoInput;

    assert_eq!(feature.as_str(), "video_input");
    assert_eq!(feature.to_string(), "video_input");
    assert_eq!(serde_json::to_value(feature).unwrap(), json!("video_input"));
    assert_eq!(
        serde_json::from_value::<ModelFeature>(json!("video_input")).unwrap(),
        feature
    );
}

#[test]
fn video_generation_feature_has_stable_config_display_and_serde_values() {
    let feature = ModelFeature::VideoGeneration;

    assert_eq!(feature.as_str(), "video_generation");
    assert_eq!(feature.to_string(), "video_generation");
    assert_eq!(
        serde_json::to_value(feature).unwrap(),
        json!("video_generation")
    );
    assert_eq!(
        serde_json::from_value::<ModelFeature>(json!("video_generation")).unwrap(),
        feature
    );
}
