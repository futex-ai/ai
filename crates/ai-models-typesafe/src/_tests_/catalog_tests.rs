//! TypeSafe catalog contract tests.

use ai_interface::{CostTier, IntelligenceScore, ModelFeature, ProviderId, SpeedTier};
use serde_json::json;

use super::{JEV_1_13_0, JEV_LATEST, PROVIDER_ID, known_models};

#[test]
fn catalog_exposes_exact_jev_models() {
    assert_eq!(PROVIDER_ID.as_str(), "typesafe");
    assert_eq!(PROVIDER_ID.to_string(), "typesafe");
    assert_eq!(
        serde_json::to_value(PROVIDER_ID).unwrap(),
        json!("typesafe")
    );
    assert_eq!(
        serde_json::from_value::<ProviderId>(json!("typesafe")).unwrap(),
        PROVIDER_ID
    );
    let alias = serde_json::from_value::<ProviderId>(json!("type_safe")).unwrap();
    assert_ne!(alias, PROVIDER_ID);
    assert_eq!(alias.as_str(), "type_safe");

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
        assert_eq!(model.default_effort, None);
        assert_eq!(model.features.as_ref(), [ModelFeature::Judgment]);
    }
}
