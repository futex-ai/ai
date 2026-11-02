//! Catalog fixtures for judgment runner tests.

use std::borrow::Cow;

use ai_interface::{ModelFeature, ModelSpec, ProviderId};

const JUDGMENT_FEATURES: &[ModelFeature] = &[ModelFeature::Judgment];

pub(super) fn judgment_spec(provider: ProviderId, provider_model_id: &'static str) -> ModelSpec {
    ModelSpec {
        features: Cow::Borrowed(JUDGMENT_FEATURES),
        ..ModelSpec::custom(
            provider,
            provider_model_id,
            &ai_models_typesafe::CUSTOM_DEFAULTS,
        )
    }
}

pub(super) fn mock_spec() -> ModelSpec {
    judgment_spec(ai_models_typesafe::PROVIDER_ID, "mock-judgment")
}
