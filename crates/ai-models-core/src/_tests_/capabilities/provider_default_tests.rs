//! Unknown provider thinking remains conservative without explicit effort.

use ai_interface::{Effort, EffortSet, ForcedToolChoice, ModelControl, ModelToolChoice, Sampling};

use crate::ResolvedToolChoice;

use super::support::{request, resolve, spec, unsupported};

#[test]
fn provider_default_keeps_unknown_effort_and_conservative_controls() {
    let mut spec = spec();
    spec.default_effort = None;
    spec.sampling = Sampling::FixedWhileThinking;
    spec.forced_tool_choice = ForcedToolChoice::OnlyWithoutThinking;
    let mut request = request();
    request.controls.generation.temperature = Some(0.4);
    request.controls.generation.top_p = Some(0.8);
    request.controls.generation.tool_choice = Some(ModelToolChoice::RequiredOrAuto);
    let resolved = resolve(&request, &spec).unwrap();
    assert_eq!(resolved.effort, None);
    assert_eq!((resolved.temperature, resolved.top_p), (None, None));
    assert_eq!(resolved.tool_choice, Some(ResolvedToolChoice::Auto));
    for choice in [
        ModelToolChoice::Required,
        ModelToolChoice::Function("lookup".to_owned()),
    ] {
        request.controls.generation.tool_choice = Some(choice);
        assert_eq!(
            resolve(&request, &spec),
            Err(unsupported(ModelControl::ToolChoice))
        );
    }
}

#[test]
fn no_effort_controls_accepts_default_and_rejects_explicit_effort() {
    let mut spec = spec();
    spec.default_effort = None;
    spec.efforts = EffortSet::empty();
    let mut request = request();
    assert_eq!(
        resolve(&request, &spec).map(|controls| controls.effort),
        Ok(None)
    );
    for effort in [Effort::Disabled, Effort::High] {
        request.controls.generation.effort = Some(effort);
        assert_eq!(
            resolve(&request, &spec),
            Err(unsupported(ModelControl::Effort))
        );
    }
}
