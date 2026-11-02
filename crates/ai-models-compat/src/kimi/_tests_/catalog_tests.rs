//! Kimi catalog metadata and supported-effort tests.

use ai_interface::{CostTier, Effort, IntelligenceScore, ModelFeature, SpeedTier};

use super::{CATALOG, PROVIDER_ID, known_models};

#[test]
fn catalog_exposes_one_k3_model_with_all_efforts() {
    let models = known_models();
    assert_eq!(models, CATALOG);
    assert_eq!(models.len(), 1);
    assert_eq!(models[0].provider_model_id, "kimi-k3");
    assert_eq!(models[0].default_effort, Effort::Max);
    assert_eq!(
        models[0].efforts.iter().collect::<Vec<_>>(),
        vec![Effort::Low, Effort::High, Effort::Max]
    );
}

#[test]
fn catalog_model_keeps_k3_provider_contract() {
    let model = &CATALOG[0];
    assert_eq!(model.provider, PROVIDER_ID);
    assert_eq!(model.context_window_tokens, 1_000_000);
    assert_eq!(model.max_output_tokens, 131_072);
    assert_eq!(
        model.features.as_ref(),
        [
            ModelFeature::ToolCalling,
            ModelFeature::StructuredOutput,
            ModelFeature::Vision,
            ModelFeature::LongContext,
            ModelFeature::Reasoning
        ]
    );
    assert_eq!(
        model.routing.expect("routing tiers").intelligence,
        IntelligenceScore::Ten
    );
}

#[test]
fn catalog_model_keeps_default_speed_and_cost() {
    let tiers = CATALOG[0].routing.expect("routing tiers");
    assert_eq!(tiers.speed, SpeedTier::Slow);
    assert_eq!(tiers.cost, CostTier::Premium);
}
