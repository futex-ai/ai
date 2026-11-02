//! Internal-error handling for model event order violations.

use crate::{ErrorKind, FinishReason, ModelError, ModelEvent, ModelResponseFold};

use super::fixtures::{arguments, finish, fold, reasoning, text, tool_start, usage};

#[test]
fn fold_rejects_invalid_event_order_with_an_internal_error() {
    let cases = [
        ("skipped position", vec![text(1, "wrong")]),
        (
            "changed part kind",
            vec![text(0, "text"), reasoning(0, "thought")],
        ),
        ("empty text", vec![text(0, "")]),
        ("empty reasoning", vec![reasoning(0, "")]),
        ("arguments without start", vec![arguments(0, "{}")]),
        (
            "end without start",
            vec![ModelEvent::ToolCallEnd {
                part_index: 0,
                provider_state: None,
            }],
        ),
        (
            "duplicate start",
            vec![tool_start(0, "one"), tool_start(0, "two")],
        ),
        (
            "empty arguments",
            vec![tool_start(0, "one"), arguments(0, "")],
        ),
        (
            "arguments after end",
            vec![
                tool_start(0, "one"),
                ModelEvent::ToolCallEnd {
                    part_index: 0,
                    provider_state: None,
                },
                arguments(0, "{}"),
            ],
        ),
        (
            "duplicate end",
            vec![
                tool_start(0, "one"),
                ModelEvent::ToolCallEnd {
                    part_index: 0,
                    provider_state: None,
                },
                ModelEvent::ToolCallEnd {
                    part_index: 0,
                    provider_state: None,
                },
            ],
        ),
        ("open call at usage", vec![tool_start(0, "one"), usage()]),
        ("duplicate usage", vec![usage(), usage()]),
        ("content after usage", vec![usage(), text(0, "late")]),
        ("finish before usage", vec![finish(FinishReason::Stop)]),
        (
            "content after finish",
            vec![usage(), finish(FinishReason::Stop), text(0, "late")],
        ),
        ("missing usage and finish", Vec::new()),
        ("missing finish", vec![usage()]),
        (
            "tool finish without a call",
            vec![usage(), finish(FinishReason::ToolCalls)],
        ),
        (
            "invalid completed arguments",
            vec![
                tool_start(0, "one"),
                arguments(0, "{"),
                ModelEvent::ToolCallEnd {
                    part_index: 0,
                    provider_state: None,
                },
                usage(),
                finish(FinishReason::ToolCalls),
            ],
        ),
        (
            "duplicate finish",
            vec![
                usage(),
                finish(FinishReason::Stop),
                finish(FinishReason::Stop),
            ],
        ),
        (
            "fallback after usage",
            vec![
                usage(),
                ModelEvent::Fallback {
                    error: ModelError::invalid_request("p", "m", "failed"),
                    emitted_events: false,
                },
            ],
        ),
    ];

    for (name, events) in cases {
        assert!(
            matches!(fold(events), Err(ModelError::Internal { core, .. }) if core.kind == ErrorKind::Internal),
            "case: {name}"
        );
    }
}

#[test]
fn fold_discards_invalid_arguments_when_the_finish_has_no_tool_calls() {
    let mut fold = ModelResponseFold::new();
    for event in [
        tool_start(0, "unused"),
        arguments(0, "incomplete JSON"),
        ModelEvent::ToolCallEnd {
            part_index: 0,
            provider_state: None,
        },
        usage(),
        finish(FinishReason::Truncated),
    ] {
        fold.push(event).unwrap();
    }

    let response = fold.finish().unwrap();
    assert_eq!(response.finish_reason, FinishReason::Truncated);
    assert_eq!(response.tool_calls(), Vec::new());
}
