//! Forced, automatic, and fallback tool choices follow spec data.

use ai_interface::{Effort, ForcedToolChoice, ModelControl, ModelToolChoice};

use crate::ResolvedToolChoice;

use super::support::{request, resolve, spec, unsupported};

#[test]
fn always_rule_allows_required_and_named_functions_while_thinking() {
    for (choice, expected) in [
        (ModelToolChoice::Required, ResolvedToolChoice::Required),
        (
            ModelToolChoice::Function("lookup".to_owned()),
            ResolvedToolChoice::Function("lookup".to_owned()),
        ),
    ] {
        let mut request = request();
        request.controls.generation.tool_choice = Some(choice);

        assert_eq!(
            resolve(&request, &spec()).map(|controls| controls.tool_choice),
            Ok(Some(expected))
        );
    }
}

#[test]
fn without_thinking_rule_uses_resolved_effort_for_both_forced_choices() {
    let mut spec = spec();
    spec.forced_tool_choice = ForcedToolChoice::OnlyWithoutThinking;
    for (choice, expected) in [
        (ModelToolChoice::Required, ResolvedToolChoice::Required),
        (
            ModelToolChoice::Function("lookup".to_owned()),
            ResolvedToolChoice::Function("lookup".to_owned()),
        ),
    ] {
        let mut request = request();
        request.controls.generation.effort = Some(Effort::Medium);
        request.controls.generation.tool_choice = Some(choice);

        assert_eq!(
            resolve(&request, &spec).map(|controls| controls.tool_choice),
            Ok(Some(expected))
        );

        request.controls.generation.effort = Some(Effort::High);
        assert_eq!(
            resolve(&request, &spec),
            Err(unsupported(ModelControl::ToolChoice))
        );
    }
}

#[test]
fn required_only_rule_rejects_named_functions_but_allows_required() {
    let mut spec = spec();
    spec.forced_tool_choice = ForcedToolChoice::RequiredOnly;
    let mut request = request();
    request.controls.generation.tool_choice = Some(ModelToolChoice::Function("lookup".to_owned()));

    assert_eq!(
        resolve(&request, &spec),
        Err(unsupported(ModelControl::ToolChoice))
    );

    request.controls.generation.tool_choice = Some(ModelToolChoice::Required);
    assert_eq!(
        resolve(&request, &spec).map(|controls| controls.tool_choice),
        Ok(Some(ResolvedToolChoice::Required))
    );
}

#[test]
fn never_rule_rejects_both_forced_choices() {
    let mut spec = spec();
    spec.forced_tool_choice = ForcedToolChoice::Never;
    for effort in [Effort::Disabled, Effort::High] {
        for choice in [
            ModelToolChoice::Required,
            ModelToolChoice::Function("lookup".to_owned()),
        ] {
            let mut request = request();
            request.controls.generation.effort = Some(effort);
            request.controls.generation.tool_choice = Some(choice);

            assert_eq!(
                resolve(&request, &spec),
                Err(unsupported(ModelControl::ToolChoice))
            );
        }
    }
}

#[test]
fn nonforcing_choices_pass_through_the_never_rule() {
    let mut spec = spec();
    spec.forced_tool_choice = ForcedToolChoice::Never;
    for (choice, expected) in [
        (Some(ModelToolChoice::None), Some(ResolvedToolChoice::None)),
        (Some(ModelToolChoice::Auto), Some(ResolvedToolChoice::Auto)),
        (None, None),
    ] {
        let mut request = request();
        request.controls.generation.tool_choice = choice;

        assert_eq!(
            resolve(&request, &spec).map(|controls| controls.tool_choice),
            Ok(expected)
        );
    }
}

#[test]
fn required_or_auto_resolves_to_the_allowed_required_choice_or_auto() {
    for (rule, effort, expected) in [
        (
            ForcedToolChoice::Always,
            Effort::High,
            ResolvedToolChoice::Required,
        ),
        (
            ForcedToolChoice::OnlyWithoutThinking,
            Effort::Medium,
            ResolvedToolChoice::Required,
        ),
        (
            ForcedToolChoice::OnlyWithoutThinking,
            Effort::High,
            ResolvedToolChoice::Auto,
        ),
        (
            ForcedToolChoice::RequiredOnly,
            Effort::High,
            ResolvedToolChoice::Required,
        ),
        (
            ForcedToolChoice::Never,
            Effort::High,
            ResolvedToolChoice::Auto,
        ),
        (
            ForcedToolChoice::Never,
            Effort::Disabled,
            ResolvedToolChoice::Auto,
        ),
    ] {
        let mut spec = spec();
        spec.forced_tool_choice = rule;
        let mut request = request();
        request.controls.generation.effort = Some(effort);
        request.controls.generation.tool_choice = Some(ModelToolChoice::RequiredOrAuto);

        assert_eq!(
            resolve(&request, &spec).map(|controls| controls.tool_choice),
            Ok(Some(expected)),
            "forced tool choice: {rule:?}, requested effort: {effort:?}"
        );
    }
}
