//! Response reconstruction, tool retention, and fallback attempt requirements.

use serde_json::json;

use crate::{
    ConversationPart, Effort, ErrorKind, FinishReason, ModelAttempt, ModelError, ModelEvent,
    ModelResponse, ModelUsage, ToolCall,
};

use super::fixtures::{arguments, finish, fold, reasoning, text, tool_start, usage};

#[test]
fn fold_orders_interleaved_parts_and_keeps_terminal_metadata() {
    let response = fold(vec![
        reasoning(0, "private"),
        text(1, "first"),
        tool_start(2, "one"),
        text(3, "last"),
        arguments(2, "{\"key\":"),
        reasoning(0, " thought"),
        text(1, " middle "),
        tool_start(4, "two"),
        arguments(4, "[1,true]"),
        ModelEvent::ToolCallEnd {
            part_index: 4,
            provider_state: None,
        },
        arguments(2, "\"value\"}"),
        ModelEvent::ToolCallEnd {
            part_index: 2,
            provider_state: None,
        },
        usage(),
        finish(FinishReason::ToolCalls),
    ])
    .expect("ordered interleaved parts must fold");

    assert_eq!(
        response,
        ModelResponse {
            provider: "mock".to_owned(),
            model_id: "mock-dev".to_owned(),
            catalog_model_id: Some("mock/mock-dev".to_owned()),
            thinking_level: Some(Effort::High),
            parts: vec![
                ConversationPart::reasoning("private thought"),
                ConversationPart::text("first middle "),
                ConversationPart::from_tool_call(ToolCall {
                    id: "one".to_owned(),
                    name: "lookup".to_owned(),
                    input: json!({"key": "value"}),
                    operation_id: Some("operation-one".to_owned()),
                }),
                ConversationPart::text("last"),
                ConversationPart::from_tool_call(ToolCall {
                    id: "two".to_owned(),
                    name: "lookup".to_owned(),
                    input: json!([1, true]),
                    operation_id: Some("operation-two".to_owned()),
                }),
            ],
            finish_reason: FinishReason::ToolCalls,
            structured_output: Some(json!({"answer": "ready"})),
            usage: ModelUsage {
                input_tokens: 3,
                output_tokens: 7,
                total_tokens: 10,
                ..Default::default()
            },
            attempts: Vec::new(),
        }
    );
}

#[test]
fn fold_keeps_tool_calls_only_for_the_tool_calls_finish() {
    for reason in [
        FinishReason::ToolCalls,
        FinishReason::Stop,
        FinishReason::Truncated,
        FinishReason::Filtered,
        FinishReason::Other("missing".to_owned()),
    ] {
        let response = fold(vec![
            tool_start(0, "one"),
            arguments(0, "{}"),
            ModelEvent::ToolCallEnd {
                part_index: 0,
                provider_state: None,
            },
            usage(),
            finish(reason.clone()),
        ])
        .expect("all normal finish reasons must fold");

        let expected = match reason {
            FinishReason::ToolCalls => vec![ToolCall {
                id: "one".to_owned(),
                name: "lookup".to_owned(),
                input: json!({}),
                operation_id: Some("operation-one".to_owned()),
            }],
            _ => Vec::new(),
        };
        assert_eq!(response.tool_calls(), expected, "finish reason: {reason:?}");
    }
}

#[test]
fn fallback_discards_partial_parts_and_records_each_failure() {
    let response = fold(vec![
        text(0, "discarded"),
        tool_start(1, "discarded"),
        ModelEvent::Fallback {
            error: ModelError::interrupted("first", "one", "closed"),
            emitted_events: true,
        },
        ModelEvent::Fallback {
            error: ModelError::rate_limited("second", "two", "busy"),
            emitted_events: false,
        },
        text(0, "retained"),
        usage(),
        finish(FinishReason::Stop),
    ])
    .expect("fallback must reset unfinished content and part positions");

    assert_eq!(response.text(), "retained");
    assert_eq!(response.tool_calls(), Vec::new());
    assert_eq!(
        response.attempts,
        vec![
            ModelAttempt {
                provider: "first".to_owned(),
                model_id: "one".to_owned(),
                kind: ErrorKind::Interrupted,
                message: "closed".to_owned(),
                emitted_events: true,
            },
            ModelAttempt {
                provider: "second".to_owned(),
                model_id: "two".to_owned(),
                kind: ErrorKind::RateLimited,
                message: "busy".to_owned(),
                emitted_events: false,
            },
        ]
    );
    let serialized = serde_json::to_value(&response).unwrap();
    assert_eq!(
        serde_json::from_value::<ModelResponse>(serialized).unwrap(),
        response
    );
}

#[test]
fn fallback_records_identical_failures_with_their_explicit_visibility() {
    let response = fold(vec![
        ModelEvent::Fallback {
            error: ModelError::interrupted("provider", "model", "closed"),
            emitted_events: false,
        },
        text(0, "discarded"),
        ModelEvent::Fallback {
            error: ModelError::interrupted("provider", "model", "closed"),
            emitted_events: true,
        },
        text(0, "retained"),
        usage(),
        finish(FinishReason::Stop),
    ])
    .unwrap();

    assert_eq!(response.text(), "retained");
    assert_eq!(response.attempts.len(), 2);
    assert_eq!(response.attempts[0].kind, ErrorKind::Interrupted);
    assert!(!response.attempts[0].emitted_events);
    assert!(response.attempts[1].emitted_events);
    let mut first = response.attempts[0].clone();
    first.emitted_events = true;
    assert_eq!(first, response.attempts[1]);
}
