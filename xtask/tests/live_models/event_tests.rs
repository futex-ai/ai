//! Completion-event observation, validation, and credential-free guards.

use std::sync::{Arc, Mutex};

use ai_interface::{
    ConversationMessage, DynModel, MockModel, Model, ModelCompletionEvent,
    ModelCompletionEventSink, ModelRequest, ModelResponse, ModelResult,
};
use async_trait::async_trait;

use super::provider_tests::{CompletionEventExpectation, LiveProvider};

#[derive(Clone, Default)]
pub(super) struct CompletionEventLog {
    events: Arc<Mutex<Vec<ModelCompletionEvent>>>,
}

impl CompletionEventLog {
    pub(super) fn recorded(&self) -> Vec<ModelCompletionEvent> {
        self.events
            .lock()
            .expect("completion event lock should not be poisoned")
            .clone()
    }
}

#[async_trait]
impl ModelCompletionEventSink for CompletionEventLog {
    async fn emit(&self, event: ModelCompletionEvent) {
        self.events
            .lock()
            .expect("completion event lock should not be poisoned")
            .push(event);
    }
}

struct EventObservingModel {
    inner: DynModel,
    events: CompletionEventLog,
}

#[async_trait]
impl Model for EventObservingModel {
    async fn complete(&self, request: &ModelRequest) -> ModelResult<ModelResponse> {
        self.inner.complete_with_events(request, &self.events).await
    }
}

pub(super) fn observing_model(inner: DynModel) -> (DynModel, CompletionEventLog) {
    let events = CompletionEventLog::default();
    let model = Arc::new(EventObservingModel {
        inner,
        events: events.clone(),
    });
    (model, events)
}

pub(super) fn completion_event_failures(
    model_id: &str,
    expectation: CompletionEventExpectation,
    assistant_message: &str,
    events: &[ModelCompletionEvent],
) -> Vec<String> {
    if expectation == CompletionEventExpectation::Silent {
        return if events.is_empty() {
            Vec::new()
        } else {
            vec![format!(
                "{model_id}: deferred completion must not emit completion events"
            )]
        };
    }

    let mut failures = Vec::new();
    let mut assistant_text = String::new();
    let mut assistant_event_count = 0;
    for event in events {
        match event {
            ModelCompletionEvent::AssistantTextDelta { delta } => {
                assistant_event_count += 1;
                if delta.is_empty() {
                    failures.push(format!("{model_id}: emitted an empty assistant delta"));
                }
                assistant_text.push_str(delta);
            }
            ModelCompletionEvent::ReasoningTextDelta { delta } if delta.is_empty() => {
                failures.push(format!("{model_id}: emitted an empty reasoning delta"));
            }
            ModelCompletionEvent::ReasoningTextDelta { .. } => {}
            ModelCompletionEvent::AttemptRestarted => failures.push(format!(
                "{model_id}: direct provider probe unexpectedly restarted"
            )),
            _ => {}
        }
    }
    if assistant_event_count == 0 {
        failures.push(format!(
            "{model_id}: synchronous completion emitted no assistant text events"
        ));
    } else if assistant_text != assistant_message {
        failures.push(format!(
            "{model_id}: assistant events did not have terminal parity"
        ));
    }
    failures
}

struct EventingProbeModel {
    inner: MockModel,
}

#[async_trait]
impl Model for EventingProbeModel {
    async fn complete(&self, request: &ModelRequest) -> ModelResult<ModelResponse> {
        self.inner.complete(request).await
    }

    async fn complete_with_events(
        &self,
        request: &ModelRequest,
        event_sink: &dyn ModelCompletionEventSink,
    ) -> ModelResult<ModelResponse> {
        let response = self.inner.complete(request).await?;
        event_sink
            .emit(ModelCompletionEvent::AssistantTextDelta {
                delta: response.assistant_message.clone(),
            })
            .await;
        Ok(response)
    }
}

#[tokio::test]
async fn observing_model_routes_complete_through_the_public_event_boundary() {
    let (model, events) = observing_model(Arc::new(EventingProbeModel {
        inner: MockModel::new("eventing-probe"),
    }));
    let response = model
        .complete(&ModelRequest {
            system_prompt: "system".to_owned(),
            messages: vec![ConversationMessage::user("probe")],
            tools: Vec::new(),
            response_schema: None,
            controls: Default::default(),
        })
        .await
        .expect("event-observing model should complete");

    assert!(
        completion_event_failures(
            "eventing-probe",
            CompletionEventExpectation::AssistantTextParity,
            &response.assistant_message,
            &events.recorded(),
        )
        .is_empty()
    );
}

#[test]
fn every_provider_has_synchronous_parity_and_only_xai_has_deferred_silence() {
    for provider in LiveProvider::ALL {
        assert_eq!(
            provider.synchronous_event_expectation(),
            CompletionEventExpectation::AssistantTextParity,
            "{provider:?} must exercise synchronous completion events"
        );
    }

    let deferred_silence = LiveProvider::ALL
        .into_iter()
        .filter(|provider| {
            provider.preferred_mode_event_expectation() == CompletionEventExpectation::Silent
        })
        .collect::<Vec<_>>();
    assert_eq!(deferred_silence, vec![LiveProvider::Xai]);
}

#[test]
fn assistant_event_validation_accepts_reasoning_and_terminal_parity() {
    let failures = completion_event_failures(
        "model-id",
        CompletionEventExpectation::AssistantTextParity,
        "hello world",
        &[
            ModelCompletionEvent::ReasoningTextDelta {
                delta: "thinking".to_owned(),
            },
            ModelCompletionEvent::AssistantTextDelta {
                delta: "hello ".to_owned(),
            },
            ModelCompletionEvent::AssistantTextDelta {
                delta: "world".to_owned(),
            },
        ],
    );

    assert!(failures.is_empty());
}

#[test]
fn assistant_event_validation_rejects_missing_or_mismatched_text() {
    let missing = completion_event_failures(
        "missing",
        CompletionEventExpectation::AssistantTextParity,
        "terminal",
        &[],
    );
    let mismatched = completion_event_failures(
        "mismatched",
        CompletionEventExpectation::AssistantTextParity,
        "terminal",
        &[ModelCompletionEvent::AssistantTextDelta {
            delta: "different".to_owned(),
        }],
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
    let failures = completion_event_failures(
        "invalid-events",
        CompletionEventExpectation::AssistantTextParity,
        "",
        &[
            ModelCompletionEvent::AssistantTextDelta {
                delta: String::new(),
            },
            ModelCompletionEvent::ReasoningTextDelta {
                delta: String::new(),
            },
            ModelCompletionEvent::AttemptRestarted,
        ],
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
fn silent_event_validation_rejects_deferred_emission() {
    let failures = completion_event_failures(
        "xai-deferred",
        CompletionEventExpectation::Silent,
        "terminal",
        &[ModelCompletionEvent::AssistantTextDelta {
            delta: "unexpected".to_owned(),
        }],
    );

    assert_eq!(
        failures,
        vec!["xai-deferred: deferred completion must not emit completion events"]
    );
}
