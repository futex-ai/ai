//! Default-effort marking and supported replay facts at the render boundary.

use ai_interface::{
    CustomModelDefaults, Effort, EffortSet, ForcedToolChoice, MOCK_CUSTOM_DEFAULTS, MOCK_PROVIDER,
    ModelSpec, ProviderId, Sampling,
};

use super::render_matrix;

#[test]
fn marks_default_and_lists_only_supported_replay_efforts() {
    let defaults = CustomModelDefaults {
        context_window_tokens: 100,
        max_output_tokens: 7,
        efforts: EffortSet::new(Effort::Disabled).with(Effort::High),
        default_effort: Some(Effort::High),
        forced_tool_choice: ForcedToolChoice::OnlyWithoutThinking,
        sampling: Sampling::FixedWhileThinking,
        supports_stop_sequences: true,
        supports_deferred: false,
        requires_output_limit: true,
        replay_efforts: &[Effort::Low, Effort::High],
    };
    let spec = ModelSpec::custom(ProviderId::new("example"), "model", &defaults);
    let rendered = render_matrix(&[spec]);
    assert_eq!(
        rendered.lines().last(),
        Some(concat!(
            "| `example/model` | disabled, **high** | 7 default | 100 | ",
            "fixed while thinking | yes | only without thinking | no | high |",
        ))
    );
}

#[test]
fn marks_provider_default_without_an_explicit_off_claim() {
    let mut spec = ModelSpec::custom(MOCK_PROVIDER, "uncontrolled", &MOCK_CUSTOM_DEFAULTS);
    let without_controls = render_matrix(&[spec.clone()]);
    assert_eq!(
        without_controls.lines().last(),
        Some(
            "| `mock/uncontrolled` | **provider default** | 128000 | 128000 | adjustable | yes | always | no | none |"
        )
    );
    spec.efforts = EffortSet::new(Effort::Low).with(Effort::High);
    let with_controls = render_matrix(&[spec]);
    assert_eq!(
        with_controls.lines().last(),
        Some(
            "| `mock/uncontrolled` | **provider default**, low, high | 128000 | 128000 | adjustable | yes | always | no | none |"
        )
    );
}
