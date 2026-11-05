//! Qwen catalog metadata and supported-effort tests.

use ai_interface::{CostTier, Effort, IntelligenceScore, ModelFeature, ProviderId, SpeedTier};
use serde_json::json;

use super::{CATALOG, PROVIDER_ID, known_models};

#[test]
fn catalog_exposes_each_stable_qwen_model_once_with_all_efforts() {
    assert_eq!(PROVIDER_ID.as_str(), "qwen");
    assert_eq!(PROVIDER_ID.to_string(), "qwen");
    assert_eq!(serde_json::to_value(PROVIDER_ID).unwrap(), json!("qwen"));
    assert_eq!(
        serde_json::from_value::<ProviderId>(json!("qwen")).unwrap(),
        PROVIDER_ID
    );
    let alias = serde_json::from_value::<ProviderId>(json!("qwencloud")).unwrap();
    assert_ne!(alias, PROVIDER_ID);
    assert_eq!(alias.as_str(), "qwencloud");

    let models = known_models();
    assert_eq!(models, CATALOG);
    assert_eq!(
        models
            .iter()
            .map(|model| model.provider_model_id.as_ref())
            .collect::<Vec<_>>(),
        vec!["qwen3.7-max", "qwen3.7-plus", "qwen3.7-flash"]
    );
    for model in models {
        assert_eq!(model.default_effort, Some(Effort::High));
        assert_eq!(
            model.efforts.iter().collect::<Vec<_>>(),
            vec![Effort::Disabled, Effort::High]
        );
    }
}

#[test]
fn catalog_matches_qwen_context_and_capability_contract() {
    for (index, model) in CATALOG.iter().enumerate() {
        assert_eq!(model.provider, PROVIDER_ID);
        assert_eq!(model.context_window_tokens, 1_000_000);
        assert_eq!(model.max_output_tokens, 65_536);
        assert!(model.has_feature(ModelFeature::ToolCalling));
        assert!(model.has_feature(ModelFeature::StructuredOutput));
        assert!(model.has_feature(ModelFeature::LongContext));
        assert!(model.has_feature(ModelFeature::Reasoning));
        assert_eq!(model.has_feature(ModelFeature::Vision), index != 0);
    }
}

#[test]
fn catalog_matches_default_routing_metadata() {
    let max = CATALOG[0].routing.expect("Max routing tiers");
    let plus = CATALOG[1].routing.expect("Plus routing tiers");
    let flash = CATALOG[2].routing.expect("Flash routing tiers");
    assert_eq!(max.intelligence, IntelligenceScore::Ten);
    assert_eq!(max.speed, SpeedTier::Slow);
    assert_eq!(max.cost, CostTier::Premium);
    assert_eq!(plus.intelligence, IntelligenceScore::Nine);
    assert_eq!(plus.speed, SpeedTier::Medium);
    assert_eq!(plus.cost, CostTier::Medium);
    assert_eq!(flash.intelligence, IntelligenceScore::Eight);
    assert_eq!(flash.speed, SpeedTier::Fast);
    assert_eq!(flash.cost, CostTier::Low);
}
