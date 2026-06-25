use crate::{ModelFeature, ModelPreference, ModelRequirement, ModelRouteRequest, ProviderKind};

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
        .require(ModelRequirement::ModelId("gpt-5.5".to_owned()))
        .require(ModelRequirement::Provider(ProviderKind::OpenAi))
        .require_feature(ModelFeature::StructuredOutput)
        .prefer(ModelPreference::Intelligence)
        .prefer(ModelPreference::LowCost)
        .build();

    assert_eq!(
        request.requirements,
        vec![
            ModelRequirement::ModelId("gpt-5.5".to_owned()),
            ModelRequirement::Provider(ProviderKind::OpenAi),
            ModelRequirement::Feature(ModelFeature::StructuredOutput),
        ]
    );
    assert_eq!(
        request.preferences,
        vec![ModelPreference::Intelligence, ModelPreference::LowCost]
    );
}

#[test]
fn provider_kind_round_trips_config_strings() {
    assert_eq!(
        ProviderKind::from_config_str("anthropic"),
        Some(ProviderKind::Anthropic)
    );
    assert_eq!(ProviderKind::Xai.as_str(), "xai");
    assert_eq!(ProviderKind::from_config_str("unknown"), None);
}
