//! Default-effort marking and supported replay facts at the render boundary.

use ai_interface::{
    CustomModelDefaults, Effort, EffortSet, ForcedToolChoice, ModelSpec, ProviderId, Sampling,
};

use super::render_matrix;

#[test]
fn marks_default_and_lists_only_supported_replay_efforts() {
    let defaults = CustomModelDefaults {
        context_window_tokens: 100,
        max_output_tokens: 7,
        efforts: EffortSet::new(Effort::Disabled).with(Effort::High),
        default_effort: Effort::High,
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
