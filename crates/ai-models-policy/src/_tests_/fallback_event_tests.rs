//! Leaf event flags, flat failures, and winning provider state.

use std::{error::Error, sync::Arc};

use ai_interface::{
    ConversationPart, ErrorCore, ErrorKind, Model, ModelAttempt, ModelError, ModelEvent,
    ModelResponseFold, ProviderId, ProviderState, SafeErrorSource,
    test_support::model_response_stream,
};
use serde_json::json;

use crate::{AnyError, FallbackModel};

use super::fixtures::{model, request, response, stream, text};

#[tokio::test]
async fn emitted_event_flags_reset_per_leaf_and_fallback_keeps_only_winning_provider_state() {
    let discarded_state = state("discarded");
    let winning_state = state("winner");
    let mut winning = response("winner");
    winning.parts =
        vec![ConversationPart::text("winner").with_provider_state(winning_state.clone())];
    let mut forwarded_core = ErrorCore::new(ErrorKind::Transient, "inner", "forwarded", "keep me");
    forwarded_core.provider_request_id = Some("original-request".to_owned());
    let forwarded = ModelError::with_source(
        forwarded_core.clone(),
        SafeErrorSource::redacted(std::io::Error::other("original source")),
    );
    let chain = FallbackModel::new(
        vec![
            model(vec![Err(failure("open"))]),
            model(vec![Ok(stream(vec![Err(failure("pre-event-read"))]))]),
            model(vec![Ok(stream(vec![
                Ok(Some(text("discard text"))),
                Err(failure("text")),
            ]))]),
            model(vec![Ok(stream(vec![
                Ok(Some(ModelEvent::ReasoningDelta {
                    part_index: 0,
                    delta: "discard thought".to_owned(),
                    provider_state: None,
                })),
                Err(failure("reasoning")),
            ]))]),
            model(vec![Ok(stream(vec![
                Ok(Some(ModelEvent::ToolCallStart {
                    part_index: 0,
                    id: "call".to_owned(),
                    name: "tool".to_owned(),
                    operation_id: None,
                })),
                Err(failure("tool")),
            ]))]),
            model(vec![Ok(stream(vec![
                Ok(Some(ModelEvent::TextDelta {
                    part_index: 0,
                    delta: "discard state".to_owned(),
                    provider_state: Some(discarded_state),
                })),
                Ok(Some(ModelEvent::Fallback {
                    error: forwarded,
                    emitted_events: true,
                })),
                Err(failure("after-inner-fallback")),
            ]))]),
            model(vec![Ok(model_response_stream(winning))]),
        ],
        Arc::new(AnyError),
    );
    let mut stream = chain.stream(&request()).await.unwrap();
    let mut histories = Vec::new();
    let mut flags = Vec::new();
    let mut forwarded_sources = Vec::new();
    let mut fold = ModelResponseFold::new();
    while let Some(event) = stream.next().await.unwrap() {
        if let ModelEvent::Fallback {
            error,
            emitted_events,
        } = &event
        {
            histories.push(error.attempts().len());
            flags.push(*emitted_events);
            if error.core().model_id == "forwarded" {
                forwarded_sources.push((
                    error.core().clone(),
                    error
                        .source()
                        .unwrap()
                        .downcast_ref::<SafeErrorSource>()
                        .unwrap()
                        .raw_cause()
                        .downcast_ref::<std::io::Error>()
                        .unwrap()
                        .to_string(),
                ));
            }
        }
        fold.push(event).unwrap();
    }
    let result = fold.finish().unwrap();
    assert_eq!(flags, vec![false, false, true, true, true, true, false]);
    assert_eq!(histories, vec![0; 7]);
    assert_eq!(
        forwarded_sources,
        vec![(forwarded_core, "original source".to_owned())]
    );
    assert_eq!(result.text(), "winner");
    assert_eq!(
        result.parts,
        vec![ConversationPart::text("winner").with_provider_state(winning_state)]
    );
    assert_eq!(
        result.attempts,
        vec![
            record("open", false),
            record("pre-event-read", false),
            record("text", true),
            record("reasoning", true),
            record("tool", true),
            ModelAttempt {
                provider: "inner".to_owned(),
                model_id: "forwarded".to_owned(),
                kind: ErrorKind::Transient,
                message: "keep me".to_owned(),
                emitted_events: true
            },
            record("after-inner-fallback", false),
        ]
    );
}

fn failure(model: &str) -> ModelError {
    ModelError::transient("p", model, "failed")
}

fn record(model: &str, emitted_events: bool) -> ModelAttempt {
    ModelAttempt {
        provider: "p".to_owned(),
        model_id: model.to_owned(),
        kind: ErrorKind::Transient,
        message: "failed".to_owned(),
        emitted_events,
    }
}

fn state(value: &str) -> ProviderState {
    ProviderState {
        provider: ProviderId::new("mock"),
        model: "mock".to_owned(),
        version: 1,
        payload: json!({"value": value}).as_object().unwrap().clone(),
    }
}
