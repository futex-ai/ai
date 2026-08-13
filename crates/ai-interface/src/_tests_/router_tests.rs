use crate::{ModelFeature, ModelPreference, ModelRequirement, ModelRouteRequest, ProviderKind};
use serde_json::json;

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
    assert_eq!(
        ProviderKind::from_config_str("deepseek"),
        Some(ProviderKind::DeepSeek)
    );
    assert_eq!(ProviderKind::DeepSeek.as_str(), "deepseek");
    assert_eq!(ProviderKind::DeepSeek.to_string(), "deepseek");
    assert_eq!(
        ProviderKind::from_config_str("kimi"),
        Some(ProviderKind::Kimi)
    );
    assert_eq!(ProviderKind::Kimi.as_str(), "kimi");
    assert_eq!(ProviderKind::Kimi.to_string(), "kimi");
    assert_eq!(
        ProviderKind::from_config_str("minimax"),
        Some(ProviderKind::MiniMax)
    );
    assert_eq!(ProviderKind::MiniMax.as_str(), "minimax");
    assert_eq!(ProviderKind::MiniMax.to_string(), "minimax");
    assert_eq!(
        ProviderKind::from_config_str("qwen"),
        Some(ProviderKind::Qwen)
    );
    assert_eq!(ProviderKind::Qwen.as_str(), "qwen");
    assert_eq!(ProviderKind::Qwen.to_string(), "qwen");
    assert_eq!(ProviderKind::Xai.as_str(), "xai");
    assert_eq!(ProviderKind::from_config_str("unknown"), None);
}

#[test]
fn deepseek_provider_serializes_with_config_identifier() {
    assert_eq!(
        serde_json::to_value(ProviderKind::DeepSeek).unwrap(),
        json!("deepseek")
    );
    assert_eq!(
        serde_json::from_value::<ProviderKind>(json!("deepseek")).unwrap(),
        ProviderKind::DeepSeek
    );
    assert!(serde_json::from_value::<ProviderKind>(json!("deep_seek")).is_err());
    assert!(serde_json::from_value::<ProviderKind>(json!("deepseek-ai")).is_err());
}

#[test]
fn openai_provider_serializes_with_config_identifier() {
    assert_eq!(
        serde_json::to_value(ProviderKind::OpenAi).unwrap(),
        json!("openai")
    );
    assert_eq!(
        serde_json::from_value::<ProviderKind>(json!("openai")).unwrap(),
        ProviderKind::OpenAi
    );
    assert!(serde_json::from_value::<ProviderKind>(json!("open_ai")).is_err());
}

#[test]
fn kimi_provider_serializes_with_config_identifier() {
    assert_eq!(
        serde_json::to_value(ProviderKind::Kimi).unwrap(),
        json!("kimi")
    );
    assert_eq!(
        serde_json::from_value::<ProviderKind>(json!("kimi")).unwrap(),
        ProviderKind::Kimi
    );
    assert!(serde_json::from_value::<ProviderKind>(json!("moonshot")).is_err());
}

#[test]
fn minimax_provider_serializes_with_config_identifier() {
    assert_eq!(
        serde_json::to_value(ProviderKind::MiniMax).unwrap(),
        json!("minimax")
    );
    assert_eq!(
        serde_json::from_value::<ProviderKind>(json!("minimax")).unwrap(),
        ProviderKind::MiniMax
    );
    assert!(serde_json::from_value::<ProviderKind>(json!("mini_max")).is_err());
}

#[test]
fn qwen_provider_serializes_with_config_identifier() {
    assert_eq!(
        serde_json::to_value(ProviderKind::Qwen).unwrap(),
        json!("qwen")
    );
    assert_eq!(
        serde_json::from_value::<ProviderKind>(json!("qwen")).unwrap(),
        ProviderKind::Qwen
    );
    assert!(serde_json::from_value::<ProviderKind>(json!("qwencloud")).is_err());
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
