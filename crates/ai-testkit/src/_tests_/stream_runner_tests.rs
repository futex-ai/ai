//! Ordered sanitized events remain beside the folded response or original error.

use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
    time::Duration,
};

use ai_interface::{
    DynModel, DynModelEventStream, Effort, ErrorCore, ErrorKind, FinishReason, ModelAttempt,
    ModelError, ModelEvent, ModelEventStreamMock, ModelMock, ModelUsage, ProviderId, ProviderState,
    SafeErrorSource, test_support::model_event_stream,
};
use json_http::test_support::Redactor;
use serde_json::json;
use unimock::{MockFn, Unimock, matching};

use crate::{
    provider::ProviderModelFactory,
    runner::{ConformanceRunner, ScenarioRunner},
    scenario::Scenario,
};

#[tokio::test]
async fn execute_snapshots_interleaved_events_and_the_folded_response() {
    let model = model(model_event_stream(vec![
        ModelEvent::ReasoningDelta {
            part_index: 0,
            delta: "secret-token".to_owned(),
            provider_state: None,
        },
        ModelEvent::TextDelta {
            part_index: 1,
            delta: "he".to_owned(),
            provider_state: Some(ProviderState {
                provider: ProviderId::new("openai"),
                model: "snapshot".to_owned(),
                version: 1,
                payload: json!({"phase":"final"}).as_object().unwrap().clone(),
            }),
        },
        text(2, " world"),
        text(1, "llo"),
        ModelEvent::Usage {
            usage: ModelUsage {
                input_tokens: 1,
                output_tokens: 2,
                total_tokens: 3,
                ..Default::default()
            },
        },
        ModelEvent::Finish {
            finish_reason: FinishReason::Stop,
            provider: "mock".to_owned(),
            model_id: "snapshot".to_owned(),
            catalog_model_id: Some("mock/snapshot".to_owned()),
            thinking_level: Some(Effort::High),
            structured_output: None,
        },
    ]));
    let runner = ScenarioRunner::new(Arc::new(ProviderModelFactory));
    let outcome = runner
        .execute(
            model,
            Scenario::TextTurn,
            Some(Effort::High),
            &Redactor::new(["secret-token".to_owned()]),
        )
        .await
        .unwrap();

    assert_eq!(
        serde_json::to_value(outcome).unwrap(),
        json!({
            "steps": [{
                "kind": "response",
                "events": [
                    {"ReasoningDelta": {"part_index": 0, "delta": "[redacted]"}},
                    {"TextDelta": {"part_index": 1, "delta": "he", "provider_state": {"provider":"openai", "model":"snapshot", "version":1, "payload":{"phase":"final"}}}},
                    {"TextDelta": {"part_index": 2, "delta": " world"}},
                    {"TextDelta": {"part_index": 1, "delta": "llo"}},
                    {"Usage": {"usage": {"input_tokens": 1, "output_tokens": 2, "cached_input_tokens": 0,
                        "reasoning_tokens": 0, "total_tokens": 3, "estimated_cost_microusd": 0}}},
                    {"Finish": {"finish_reason": "Stop", "provider": "mock", "model_id": "snapshot",
                        "catalog_model_id": "mock/snapshot", "effort": "high"}}
                ],
                "response": {
                    "provider": "mock", "model_id": "snapshot", "catalog_model_id": "mock/snapshot", "effort": "high",
                    "parts": [{"type":"reasoning", "text":"[redacted]"}, {"type":"text", "text":"hello", "provider_state":{"provider":"openai", "model":"snapshot", "version":1, "payload":{"phase":"final"}}}, {"type":"text", "text":" world"}], "finish_reason": "Stop",
                    "usage": {"input_tokens": 1, "output_tokens": 2, "cached_input_tokens": 0,
                        "reasoning_tokens": 0, "total_tokens": 3, "estimated_cost_microusd": 0}
                }
            }],
            "remaining_exchanges": 0
        })
    );
}

#[tokio::test]
async fn execute_keeps_partial_events_and_sanitizes_the_original_read_failure() {
    let mut core = ErrorCore::new(
        ErrorKind::Interrupted,
        "mock-secret-token",
        "snapshot-secret-token",
        "closed secret-token",
    );
    core.retry_after = Some(Duration::from_secs(2));
    core.provider_request_id = Some("request-secret-token".to_owned());
    let items = Mutex::new(VecDeque::from(vec![
        Ok(Some(text(0, "secret-token partial"))),
        Err(ModelError::with_source(
            core,
            SafeErrorSource::redacted(std::io::Error::other("source secret-token")),
        )
        .with_attempts(vec![ModelAttempt {
            provider: "first-secret-token".to_owned(),
            model_id: "first-model-secret-token".to_owned(),
            kind: ErrorKind::RejectedKey,
            message: "rejected secret-token".to_owned(),
            emitted_events: false,
        }])),
    ]));
    let stream = Unimock::new(
        ModelEventStreamMock::next
            .each_call(matching!())
            .answers_arc(Arc::new(move |_| {
                items
                    .lock()
                    .unwrap()
                    .pop_front()
                    .expect("one failed stream")
            })),
    );
    let runner = ScenarioRunner::new(Arc::new(ProviderModelFactory));
    let outcome = runner
        .execute(
            model(Box::new(stream)),
            Scenario::StreamCut,
            None,
            &Redactor::new(["secret-token".to_owned()]),
        )
        .await
        .unwrap();

    assert_eq!(
        serde_json::to_value(outcome).unwrap(),
        json!({
            "steps": [{
                "kind": "error",
                "events": [{"TextDelta": {"part_index": 0, "delta": "[redacted] partial"}}],
                "error": {
                    "kind": "Interrupted", "provider": "mock-[redacted]", "model_id": "snapshot-[redacted]",
                    "message": "closed [redacted]", "retry_after": {"secs": 2, "nanos": 0},
                    "provider_request_id": "request-[redacted]",
                    "attempts": [{"provider":"first-[redacted]", "model_id":"first-model-[redacted]",
                        "kind":"RejectedKey", "message":"rejected [redacted]", "emitted_events":false}]
                }
            }],
            "remaining_exchanges": 0
        })
    );
}

fn text(part_index: usize, delta: &str) -> ModelEvent {
    ModelEvent::TextDelta {
        part_index,
        delta: delta.to_owned(),
        provider_state: None,
    }
}

fn model(stream: DynModelEventStream) -> DynModel {
    let stream = Mutex::new(Some(stream));
    Arc::new(Unimock::new(
        ModelMock::stream
            .some_call(matching!(_))
            .answers_arc(Arc::new(move |_, _| {
                Ok(stream.lock().unwrap().take().expect("one scenario step"))
            })),
    ))
}
