//! Custom specs inherit only the defaults supplied by their provider.

use std::borrow::Cow;

use crate::{
    CustomModelDefaults, Effort, EffortSet, ForcedToolChoice, ModelSpec, ProviderId, Sampling,
};

#[test]
fn custom_spec_uses_provider_defaults_and_no_routing_claims() {
    let defaults = CustomModelDefaults {
        context_window_tokens: 128_000,
        max_output_tokens: 64_000,
        efforts: EffortSet::new(Effort::Disabled).with(Effort::High),
        default_effort: Effort::High,
        forced_tool_choice: ForcedToolChoice::OnlyWithoutThinking,
        sampling: Sampling::FixedWhileThinking,
        supports_stop_sequences: true,
        supports_deferred: false,
        requires_output_limit: true,
        replay_efforts: &[Effort::High],
    };
    let spec = ModelSpec::custom(
        ProviderId::new("provider-a"),
        String::from("organization/custom-model"),
        &defaults,
    );

    assert_eq!(
        spec,
        ModelSpec {
            provider: ProviderId::new("provider-a"),
            provider_model_id: Cow::Borrowed("organization/custom-model"),
            context_window_tokens: 128_000,
            max_output_tokens: 64_000,
            efforts: EffortSet::new(Effort::Disabled).with(Effort::High),
            default_effort: Effort::High,
            forced_tool_choice: ForcedToolChoice::OnlyWithoutThinking,
            sampling: Sampling::FixedWhileThinking,
            supports_stop_sequences: true,
            supports_deferred: false,
            requires_output_limit: true,
            replay_efforts: Cow::Borrowed(&[Effort::High]),
            features: Cow::Borrowed(&[]),
            routing: None,
        }
    );
    assert_eq!(spec.text_id(), "provider-a/organization/custom-model");
}
