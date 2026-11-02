//! Anthropic catalog tests.

use ai_interface::{Effort, ModelCatalog, ModelFeature, ProviderId};
use serde_json::json;

use super::{
    CATALOG, CLAUDE_FABLE_5, CLAUDE_HAIKU_4_5, CLAUDE_OPUS_4_7, CLAUDE_OPUS_5, CLAUDE_SONNET_4_6,
    CLAUDE_SONNET_5, PROVIDER_ID, known_models,
};

#[test]
fn claude_sonnet_5_is_the_first_catalog_model() {
    assert_eq!(PROVIDER_ID.as_str(), "anthropic");
    assert_eq!(PROVIDER_ID.to_string(), "anthropic");
    assert_eq!(
        serde_json::to_value(PROVIDER_ID).unwrap(),
        json!("anthropic")
    );
    assert_eq!(
        serde_json::from_value::<ProviderId>(json!("anthropic")).unwrap(),
        PROVIDER_ID
    );

    let models = known_models();

    assert_eq!(models.first(), Some(&CLAUDE_SONNET_5));
    for legacy in [CLAUDE_SONNET_4_6, CLAUDE_OPUS_4_7, CLAUDE_HAIKU_4_5] {
        assert!(models.iter().any(|model| model == &legacy));
    }
}

#[test]
fn claude_5_family_has_current_metadata() {
    let models = known_models();

    for spec in [CLAUDE_SONNET_5, CLAUDE_OPUS_5, CLAUDE_FABLE_5] {
        let model = models
            .iter()
            .find(|model| model.provider_model_id == spec.provider_model_id)
            .expect("Claude 5 model should exist");
        assert_eq!(model.provider, PROVIDER_ID);
        assert_eq!(model.context_window_tokens, 1_000_000);
        assert!(model.has_feature(ModelFeature::Reasoning));
        assert!(model.has_feature(ModelFeature::Vision));
    }

    let opus = models
        .iter()
        .find(|model| model.provider_model_id == CLAUDE_OPUS_5.provider_model_id)
        .expect("Claude Opus 5 model should exist");
    assert_eq!(opus.provider_model_id, "claude-opus-5");
    assert!(opus.efforts.contains(Effort::Max));
}

#[test]
fn catalog_lookup_resolves_specs_and_rejects_unknown_ids() {
    let catalog = ModelCatalog::new().with_models(CATALOG.to_vec());
    let model = catalog
        .find("anthropic/claude-opus-5")
        .expect("Claude Opus 5 text id should resolve");

    assert_eq!(model.provider_model_id, "claude-opus-5");
    assert!(model.efforts.contains(Effort::Max));
    assert!(catalog.find("anthropic/custom-claude").is_none());
}
