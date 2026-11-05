//! TypeSafe judgment specs and provider-owned custom defaults.

use std::borrow::Cow;

use ai_interface::{
    CostTier, CustomModelDefaults, EffortSet, ForcedToolChoice, IntelligenceScore, ModelFeature,
    ModelSpec, ProviderId, RoutingTiers, Sampling, SpeedTier,
};

/// Stable identity of the TypeSafe provider.
pub const PROVIDER_ID: ProviderId = ProviderId::new("typesafe");

/// Descriptive defaults for an unlisted TypeSafe judgment model.
pub const CUSTOM_DEFAULTS: CustomModelDefaults = CustomModelDefaults {
    context_window_tokens: 0,
    max_output_tokens: 0,
    efforts: EffortSet::empty(),
    default_effort: None,
    forced_tool_choice: ForcedToolChoice::Never,
    sampling: Sampling::Fixed,
    supports_stop_sequences: false,
    supports_deferred: false,
    requires_output_limit: false,
    replay_efforts: &[],
};

/// Alias for the newest stable Jev release.
pub const JEV_LATEST: ModelSpec = jev_spec("jev-latest");

/// Pinned Jev 1.13.0 release.
pub const JEV_1_13_0: ModelSpec = jev_spec("jev-1.13.0");

/// One spec for each known TypeSafe provider model.
pub const CATALOG: &[ModelSpec] = &[JEV_LATEST, JEV_1_13_0];

/// Returns TypeSafe models known to this provider crate.
pub fn known_models() -> Vec<ModelSpec> {
    CATALOG.to_vec()
}

const fn jev_spec(model_id: &'static str) -> ModelSpec {
    ModelSpec {
        provider: PROVIDER_ID,
        provider_model_id: Cow::Borrowed(model_id),
        context_window_tokens: 64_000,
        max_output_tokens: 0,
        efforts: EffortSet::empty(),
        default_effort: None,
        forced_tool_choice: ForcedToolChoice::Never,
        sampling: Sampling::Fixed,
        supports_stop_sequences: false,
        supports_deferred: false,
        requires_output_limit: false,
        replay_efforts: Cow::Borrowed(&[]),
        features: Cow::Borrowed(&[ModelFeature::Judgment]),
        routing: Some(RoutingTiers {
            intelligence: IntelligenceScore::Five,
            speed: SpeedTier::VeryFast,
            cost: CostTier::Low,
        }),
    }
}

#[cfg(test)]
#[path = "_tests_/catalog_tests.rs"]
mod catalog_tests;
