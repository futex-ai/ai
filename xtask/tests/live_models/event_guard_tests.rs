//! Offline guards for ordered events in every live completion mode.

use std::sync::Arc;

use ai_interface::{
    ConversationMessage, DynModelEventStream, FinishReason, MockModel, Model, ModelAttemptError,
    ModelRequest, ModelResult, ModelUsage, StructuredOutputSchema,
};
use ai_testkit::event_snapshot::ModelEventSnapshot;
use async_trait::async_trait;
use serde_json::json;

use crate::provider_tests::{CompletionEventExpectation, LiveProvider};

use super::{completion_event_failures, observing_model};

struct EventingProbeModel {
    inner: MockModel,
}

#[async_trait]
impl Model for EventingProbeModel {
    async fn stream(&self, request: &ModelRequest) -> ModelResult<DynModelEventStream> {
        self.inner.stream(request).await
    }
}

#[tokio::test]
async fn observing_model_routes_complete_through_the_public_event_boundary() {
    let (model, events) = observing_model(Arc::new(EventingProbeModel {
        inner: MockModel::new("eventing-probe"),
    }));
    let response = model
        .complete(&request())
        .await
        .expect("event-observing model should complete");
    assert!(
        completion_event_failures(
            "eventing-probe",
            CompletionEventExpectation::AssistantTextParity,
            &response.assistant_message,
            &events.recorded()
        )
        .is_empty()
    );
}

#[test]
fn every_provider_has_parity_in_synchronous_and_preferred_modes() {
    for provider in LiveProvider::ALL {
        assert_eq!(
            provider.synchronous_event_expectation(),
            CompletionEventExpectation::AssistantTextParity,
            "{provider:?} must exercise synchronous completion events"
        );
    }
    let preferred_parity = LiveProvider::ALL
        .into_iter()
        .filter(|provider| {
            provider.preferred_mode_event_expectation()
                == CompletionEventExpectation::AssistantTextParity
        })
        .collect::<Vec<_>>();
    assert_eq!(preferred_parity, LiveProvider::ALL.to_vec());
}

#[test]
fn assistant_event_validation_accepts_reasoning_and_terminal_parity() {
    let failures = validate(
        "model-id",
        "hello world",
        with_terminal(vec![
            ModelEventSnapshot::ReasoningDelta {
                part_index: 0,
                delta: "thinking".to_owned(),
            },
            text(1, "hello "),
            text(1, "world"),
        ]),
    );
    assert!(failures.is_empty());
}

#[test]
fn assistant_event_validation_rejects_missing_or_mismatched_text() {
    let missing = validate("missing", "terminal", with_terminal(Vec::new()));
    let mismatched = validate(
        "mismatched",
        "terminal",
        with_terminal(vec![text(0, "different")]),
    );
    assert_eq!(
        missing,
        vec!["missing: synchronous completion emitted no assistant text events"]
    );
    assert_eq!(
        mismatched,
        vec!["mismatched: assistant events did not have terminal parity"]
    );
}

#[test]
fn assistant_event_validation_rejects_empty_deltas_and_restarts() {
    let failures = validate(
        "invalid-events",
        "",
        with_terminal(vec![
            text(0, ""),
            ModelEventSnapshot::ReasoningDelta {
                part_index: 1,
                delta: String::new(),
            },
            ModelEventSnapshot::Fallback {
                error: ModelAttemptError::Provider {
                    provider: "mock".to_owned(),
                    model_id: "model".to_owned(),
                    message: "failed".to_owned(),
                },
            },
        ]),
    );
    assert_eq!(
        failures,
        vec![
            "invalid-events: emitted an empty assistant delta",
            "invalid-events: emitted an empty reasoning delta",
            "invalid-events: direct provider probe unexpectedly restarted",
        ]
    );
}

#[test]
fn deferred_event_validation_accepts_deliberate_emission() {
    let failures = validate(
        "xai-deferred",
        "terminal",
        with_terminal(vec![text(0, "terminal")]),
    );
    assert_eq!(failures, Vec::<String>::new());
}

#[test]
fn assistant_event_validation_orders_interleaved_text_parts() {
    let failures = validate(
        "interleaved",
        "hello world",
        with_terminal(vec![text(0, "he"), text(1, " world"), text(0, "llo")]),
    );
    assert_eq!(failures, Vec::<String>::new());
}

#[test]
fn assistant_event_validation_requires_usage_and_finish() {
    let failures = validate("unfinished", "terminal", vec![text(0, "terminal")]);
    assert_eq!(
        failures,
        vec![
            "unfinished: completion did not emit exactly one usage event",
            "unfinished: completion did not emit exactly one finish event",
        ]
    );
}

#[tokio::test]
async fn observing_model_records_schema_text_usage_and_finish() {
    let (model, events) = observing_model(Arc::new(MockModel::new("schema-probe")));
    let mut request = request();
    request.response_schema = Some(StructuredOutputSchema {
        name: "answer".to_owned(),
        schema: json!({"const": {"ok": true}}),
    });
    let response = model.complete(&request).await.unwrap();
    assert_eq!(events.recorded().len(), 3);
    assert_eq!(response.structured_output, Some(json!({"ok": true})));
    assert_eq!(
        completion_event_failures(
            "schema-probe",
            CompletionEventExpectation::AssistantTextParity,
            &response.assistant_message,
            &events.recorded()
        ),
        Vec::<String>::new()
    );
}

fn validate(model_id: &str, message: &str, events: Vec<ModelEventSnapshot>) -> Vec<String> {
    completion_event_failures(
        model_id,
        CompletionEventExpectation::AssistantTextParity,
        message,
        &events,
    )
}

fn text(part_index: usize, delta: &str) -> ModelEventSnapshot {
    ModelEventSnapshot::TextDelta {
        part_index,
        delta: delta.to_owned(),
    }
}

fn with_terminal(mut events: Vec<ModelEventSnapshot>) -> Vec<ModelEventSnapshot> {
    events.push(ModelEventSnapshot::Usage {
        usage: ModelUsage::default(),
    });
    events.push(ModelEventSnapshot::Finish {
        finish_reason: FinishReason::Stop,
        provider: "mock".to_owned(),
        model_id: "model".to_owned(),
        catalog_model_id: None,
        thinking_level: None,
        structured_output: None,
        provider_context: Vec::new(),
    });
    events
}

fn request() -> ModelRequest {
    ModelRequest {
        system_prompt: "system".to_owned(),
        messages: vec![ConversationMessage::user("probe")],
        tools: Vec::new(),
        response_schema: None,
        controls: Default::default(),
    }
}
