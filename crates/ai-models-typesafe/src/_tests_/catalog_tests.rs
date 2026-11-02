//! TypeSafe catalog contract tests.

use ai_interface::{CostTier, Effort, IntelligenceScore, ModelFeature, SpeedTier};

use super::{JEV_1_13_0, JEV_LATEST, PROVIDER_ID, known_models};

#[test]
fn catalog_exposes_exact_jev_models() {
    let models = known_models();

    assert_eq!(
        models
            .iter()
            .map(|model| model.text_id())
            .collect::<Vec<_>>(),
        vec!["typesafe/jev-latest", "typesafe/jev-1.13.0"]
    );
    assert_eq!(
        models
            .iter()
            .map(|model| model.provider_model_id.as_ref())
            .collect::<Vec<_>>(),
        vec![
            JEV_LATEST.provider_model_id.as_ref(),
            JEV_1_13_0.provider_model_id.as_ref()
        ]
    );
}

#[test]
fn catalog_entries_have_the_typesafe_judgment_contract() {
    for model in known_models() {
        let routing = model.routing.expect("catalog routing metadata");
        assert_eq!(model.provider, PROVIDER_ID);
        assert_eq!(model.context_window_tokens, 64_000);
        assert_eq!(routing.intelligence, IntelligenceScore::Five);
        assert_eq!(routing.speed, SpeedTier::VeryFast);
        assert_eq!(routing.cost, CostTier::Low);
        assert_eq!(model.default_effort, Effort::Disabled);
        assert_eq!(model.features.as_ref(), [ModelFeature::Judgment]);
    }
}
