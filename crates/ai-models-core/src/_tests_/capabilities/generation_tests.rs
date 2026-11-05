//! Output limits, sampling rules, and ordered stop sequences.

use ai_interface::{Effort, ModelControl, Sampling};

use super::support::{request, resolve, spec, unsupported};

#[test]
fn caps_output_limits_and_defaults_only_when_required() {
    for (requested, ceiling, required, expected) in [
        (Some(80_000), 64_000, false, Some(64_000)),
        (Some(12_000), 64_000, true, Some(12_000)),
        (None, 64_000, false, None),
        (None, 64_000, true, Some(64_000)),
        (Some(1), 0, false, Some(0)),
        (Some(0), 64_000, true, Some(0)),
    ] {
        let mut request = request();
        request.controls.generation.max_output_tokens = requested;
        let mut spec = spec();
        spec.max_output_tokens = ceiling;
        spec.requires_output_limit = required;

        assert_eq!(
            resolve(&request, &spec).map(|controls| controls.max_output_tokens),
            Ok(expected),
            "requested: {requested:?}, ceiling: {ceiling}, required: {required}"
        );
    }
}

#[test]
fn sampling_values_follow_the_rule_at_the_resolved_effort() {
    for (rule, requested, expected) in [
        (Sampling::Adjustable, Effort::High, (Some(0.3), Some(0.8))),
        (
            Sampling::FixedWhileThinking,
            Effort::Medium,
            (Some(0.3), Some(0.8)),
        ),
        (Sampling::FixedWhileThinking, Effort::High, (None, None)),
        (Sampling::Fixed, Effort::Disabled, (None, None)),
    ] {
        let mut request = request();
        request.controls.generation.effort = Some(requested);
        request.controls.generation.temperature = Some(0.3);
        request.controls.generation.top_p = Some(0.8);
        let mut spec = spec();
        spec.sampling = rule;

        assert_eq!(
            resolve(&request, &spec).map(|controls| (controls.temperature, controls.top_p)),
            Ok(expected),
            "sampling: {rule:?}, requested effort: {requested:?}"
        );
    }
}

#[test]
fn preserves_stop_order_and_rejects_only_unsupported_nonempty_stops() {
    let mut request = request();
    request.controls.generation.stop_sequences = vec!["second".to_owned(), "first".to_owned()];
    let mut spec = spec();

    assert_eq!(
        resolve(&request, &spec).map(|controls| controls.stop_sequences),
        Ok(vec!["second".to_owned(), "first".to_owned()])
    );

    spec.supports_stop_sequences = false;
    assert_eq!(
        resolve(&request, &spec),
        Err(unsupported(ModelControl::StopSequences))
    );

    request.controls.generation.stop_sequences.clear();
    assert_eq!(
        resolve(&request, &spec).map(|controls| controls.stop_sequences),
        Ok(Vec::new())
    );
}
