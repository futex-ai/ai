//! DeepSeek catalog metadata and supported-effort tests.

use ai_interface::{CostTier, Effort, IntelligenceScore, ModelFeature, SpeedTier};

use super::{CATALOG, PROVIDER_ID, known_models};

#[test]
fn catalog_exposes_each_deepseek_model_once_with_all_efforts() {
    let models = known_models();
    assert_eq!(models, CATALOG);
    assert_eq!(
        models
            .iter()
            .map(|model| model.provider_model_id.as_ref())
            .collect::<Vec<_>>(),
        vec!["deepseek-v4-pro", "deepseek-v4-flash"]
    );
    for model in models {
        assert_eq!(model.default_effort, Effort::High);
        assert_eq!(
            model.efforts.iter().collect::<Vec<_>>(),
            vec![Effort::Disabled, Effort::High, Effort::Max]
        );
    }
}

#[test]
fn catalog_models_match_provider_metadata_and_features() {
    let expected_features = [
        ModelFeature::ToolCalling,
        ModelFeature::StructuredOutput,
        ModelFeature::LongContext,
        ModelFeature::Reasoning,
    ];
    for model in known_models() {
        assert_eq!(model.provider, PROVIDER_ID);
        assert_eq!(model.context_window_tokens, 1_000_000);
        assert_eq!(model.max_output_tokens, 393_216);
        assert_eq!(model.routing.expect("routing tiers").cost, CostTier::Low);
        assert_eq!(model.features.as_ref(), expected_features);
        assert!(!model.has_feature(ModelFeature::Vision));
    }
}

#[test]
fn catalog_models_match_default_intelligence_and_speed() {
    let models = known_models();
    let pro = models[0].routing.expect("Pro routing tiers");
    let flash = models[1].routing.expect("Flash routing tiers");
    assert_eq!(pro.intelligence, IntelligenceScore::Ten);
    assert_eq!(pro.speed, SpeedTier::Medium);
    assert_eq!(flash.intelligence, IntelligenceScore::Nine);
    assert_eq!(flash.speed, SpeedTier::Fast);
}
