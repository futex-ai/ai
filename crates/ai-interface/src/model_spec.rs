//! Constant-friendly model facts with explicit provider-owned custom defaults.

use std::borrow::Cow;

use crate::{
    CustomModelDefaults, Effort, EffortSet, ForcedToolChoice, ModelFeature, ProviderId,
    RoutingTiers, Sampling,
};

#[derive(Clone, Debug, Eq, PartialEq)]
/// Provider model facts used by adapters, catalogs, and routing.
pub struct ModelSpec {
    /// Provider that serves the model.
    pub provider: ProviderId,
    /// Concrete model identifier sent to the provider.
    pub provider_model_id: Cow<'static, str>,
    /// Documented total context tokens, or zero when unknown.
    pub context_window_tokens: u32,
    /// Maximum allowed generated-token limit.
    pub max_output_tokens: u32,
    /// Ordered set of explicitly supported efforts, which can be empty.
    pub efforts: EffortSet,
    /// Effort used when the request supplies none. `None` leaves reasoning to the provider.
    pub default_effort: Option<Effort>,
    /// Conditions under which forced tool choices are allowed.
    pub forced_tool_choice: ForcedToolChoice,
    /// Conditions under which sampling values are adjustable.
    pub sampling: Sampling,
    /// Whether the model accepts nonempty stop sequences.
    pub supports_stop_sequences: bool,
    /// Whether the adapter supports a deferred lifecycle.
    pub supports_deferred: bool,
    /// Whether every provider request must include an output-token limit.
    pub requires_output_limit: bool,
    /// Efforts whose tool continuations need private provider replay state.
    pub replay_efforts: Cow<'static, [Effort]>,
    /// Capabilities advertised to routing.
    pub features: Cow<'static, [ModelFeature]>,
    /// Optional routing scores at the model's default effort.
    pub routing: Option<RoutingTiers>,
}

#[cfg(test)]
#[path = "_tests_/model_spec_tests.rs"]
mod model_spec_tests;

impl ModelSpec {
    /// Builds an unlisted model with defaults supplied by its provider crate.
    ///
    /// Custom models start without features or routing scores. Every public
    /// field can be replaced by the caller.
    pub fn custom(
        provider: ProviderId,
        provider_model_id: impl Into<Cow<'static, str>>,
        defaults: &CustomModelDefaults,
    ) -> Self {
        Self {
            provider,
            provider_model_id: provider_model_id.into(),
            context_window_tokens: defaults.context_window_tokens,
            max_output_tokens: defaults.max_output_tokens,
            efforts: defaults.efforts,
            default_effort: defaults.default_effort,
            forced_tool_choice: defaults.forced_tool_choice,
            sampling: defaults.sampling,
            supports_stop_sequences: defaults.supports_stop_sequences,
            supports_deferred: defaults.supports_deferred,
            requires_output_limit: defaults.requires_output_limit,
            replay_efforts: Cow::Borrowed(defaults.replay_efforts),
            features: Cow::Borrowed(&[]),
            routing: None,
        }
    }

    /// Returns the provider/model text id used by host configuration.
    pub fn text_id(&self) -> String {
        format!("{}/{}", self.provider, self.provider_model_id)
    }

    /// Returns whether this model advertises a routing capability.
    pub fn has_feature(&self, feature: ModelFeature) -> bool {
        self.features.contains(&feature)
    }

    /// Returns whether a continuation needs private state for this effort.
    pub fn needs_replay(&self, effort: Effort) -> bool {
        self.replay_efforts.contains(&effort)
    }
}
