//! xAI catalog metadata and supported-effort tests.

use ai_interface::{Effort, ModelFeature, ProviderId};
use serde_json::json;

use super::{CATALOG, PROVIDER_ID, known_models};

#[test]
fn grok_4_5_is_the_first_catalog_model() {
    assert_eq!(PROVIDER_ID.as_str(), "xai");
    assert_eq!(PROVIDER_ID.to_string(), "xai");
    assert_eq!(serde_json::to_value(PROVIDER_ID).unwrap(), json!("xai"));
    assert_eq!(
        serde_json::from_value::<ProviderId>(json!("xai")).unwrap(),
        PROVIDER_ID
    );

    let models = known_models();
    assert_eq!(models, CATALOG);
    assert_eq!(
        models.first().map(|model| model.provider_model_id.as_ref()),
        Some("grok-4.5")
    );
    for legacy_id in ["grok-4.20-reasoning", "grok-4.20"] {
        assert!(
            models
                .iter()
                .any(|model| model.provider_model_id == legacy_id)
        );
    }
}

#[test]
fn catalog_excludes_unavailable_grok_4_20_aliases() {
    let models = known_models();
    let model_ids = models
        .iter()
        .map(|model| model.provider_model_id.as_ref())
        .collect::<Vec<_>>();
    assert!(!model_ids.contains(&"grok-4.20-mini"));
    assert!(!model_ids.contains(&"grok-4.20-thinking-high"));
}

#[test]
fn grok_4_5_keeps_current_metadata_and_all_efforts() {
    let model = &CATALOG[0];
    assert_eq!(model.provider, PROVIDER_ID);
    assert_eq!(model.provider_model_id, "grok-4.5");
    assert_eq!(model.context_window_tokens, 500_000);
    assert_eq!(model.max_output_tokens, 500_000);
    assert_eq!(model.default_effort, Effort::High);
    assert_eq!(
        model.efforts.iter().collect::<Vec<_>>(),
        vec![Effort::Low, Effort::Medium, Effort::High]
    );
    assert!(model.has_feature(ModelFeature::Reasoning));
    assert!(model.has_feature(ModelFeature::Vision));
}

#[test]
fn grok_primary_models_advertise_vision() {
    for (model, expected_effort) in [(&CATALOG[1], Effort::High), (&CATALOG[2], Effort::Disabled)] {
        assert!(model.has_feature(ModelFeature::Vision));
        assert_eq!(model.context_window_tokens, 1_000_000);
        assert_eq!(model.default_effort, expected_effort);
        assert_eq!(
            model.efforts.iter().collect::<Vec<_>>(),
            vec![expected_effort]
        );
    }
}
