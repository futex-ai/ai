//! Event-observing tests for transient retry behavior.

use std::{
    collections::VecDeque,
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

use ai_interface::{
    FinishReason, Model, ModelCompletionEvent, ModelCompletionEventSink,
    ModelCompletionEventSinkMock, ModelError, ModelRequest, ModelResponse, ModelResult, ModelUsage,
};
use async_trait::async_trait;
use unimock::{MockFn, Unimock, matching};

use crate::{RetryingModel, SleeperMock};

#[tokio::test]
async fn transient_retry_emits_only_the_successful_attempt_events() {
    let calls = Arc::new(AtomicUsize::new(0));
    let model = RetryingModel::new(
        scripted_event_model(
            vec![
                EventAttempt {
                    events: Vec::new(),
                    result: Err(ModelError::transient_provider("openai", "gpt", "retry")),
                },
                EventAttempt {
                    events: vec![assistant_delta("ok")],
                    result: Ok(success_response()),
                },
            ],
            calls.clone(),
        ),
        Arc::new(Unimock::new(
            SleeperMock::sleep.next_call(matching!(_)).returns(()),
        )),
        vec![Duration::ZERO],
    );
    let events = Arc::new(Mutex::new(Vec::new()));
    let sink = recording_sink(events.clone());

    let response = model
        .complete_with_events(&empty_request(), &sink)
        .await
        .expect("second attempt should succeed");

    assert_eq!(response.assistant_message, "ok");
    assert_eq!(calls.load(Ordering::SeqCst), 2);
    assert_eq!(recorded_events(&events), vec![assistant_delta("ok")]);
}

#[tokio::test]
async fn interrupted_event_attempt_is_not_retried() {
    let calls = Arc::new(AtomicUsize::new(0));
    let model = RetryingModel::new(
        scripted_event_model(
            vec![
                EventAttempt {
                    events: vec![assistant_delta("partial")],
                    result: Err(ModelError::interrupted("openai", "gpt", "stream closed")),
                },
                EventAttempt {
                    events: vec![assistant_delta("unexpected")],
                    result: Ok(success_response()),
                },
            ],
            calls.clone(),
        ),
        Arc::new(Unimock::new(())),
        Vec::new(),
    );
    let events = Arc::new(Mutex::new(Vec::new()));
    let sink = recording_sink(events.clone());

    let error = model
        .complete_with_events(&empty_request(), &sink)
        .await
        .expect_err("interruption should be returned immediately");

    assert!(matches!(error, ModelError::Interrupted { .. }));
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert_eq!(recorded_events(&events), vec![assistant_delta("partial")]);
}

struct EventAttempt {
    events: Vec<ModelCompletionEvent>,
    result: ModelResult<ModelResponse>,
}

struct ScriptedEventModel {
    attempts: Mutex<VecDeque<EventAttempt>>,
    calls: Arc<AtomicUsize>,
}

#[async_trait]
impl Model for ScriptedEventModel {
    async fn complete(&self, _request: &ModelRequest) -> ModelResult<ModelResponse> {
        self.take_attempt().result
    }

    async fn complete_with_events(
        &self,
        _request: &ModelRequest,
        event_sink: &dyn ModelCompletionEventSink,
    ) -> ModelResult<ModelResponse> {
        let attempt = self.take_attempt();
        for event in attempt.events {
            event_sink.emit(event).await;
        }
        attempt.result
    }
}

impl ScriptedEventModel {
    fn take_attempt(&self) -> EventAttempt {
        self.calls.fetch_add(1, Ordering::SeqCst);
        self.attempts
            .lock()
            .expect("attempt lock should not be poisoned")
            .pop_front()
            .expect("unexpected model attempt")
    }
}

fn scripted_event_model(attempts: Vec<EventAttempt>, calls: Arc<AtomicUsize>) -> Arc<dyn Model> {
    Arc::new(ScriptedEventModel {
        attempts: Mutex::new(VecDeque::from(attempts)),
        calls,
    })
}

fn recording_sink(events: Arc<Mutex<Vec<ModelCompletionEvent>>>) -> Unimock {
    Unimock::new(
        ModelCompletionEventSinkMock::emit
            .each_call(matching!(_))
            .answers_arc(Arc::new(move |_, event| {
                events
                    .lock()
                    .expect("event lock should not be poisoned")
                    .push(event);
            })),
    )
}

fn recorded_events(events: &Mutex<Vec<ModelCompletionEvent>>) -> Vec<ModelCompletionEvent> {
    events
        .lock()
        .expect("event lock should not be poisoned")
        .clone()
}

fn assistant_delta(delta: &str) -> ModelCompletionEvent {
    ModelCompletionEvent::AssistantTextDelta {
        delta: delta.to_owned(),
    }
}

fn empty_request() -> ModelRequest {
    ModelRequest {
        system_prompt: "system".to_owned(),
        messages: Vec::new(),
        tools: Vec::new(),
        response_schema: None,
        controls: Default::default(),
    }
}

fn success_response() -> ModelResponse {
    ModelResponse {
        provider: "openai".to_owned(),
        model_id: "gpt".to_owned(),
        catalog_model_id: None,
        thinking_level: None,
        assistant_message: "ok".to_owned(),
        tool_calls: Vec::new(),
        finish_reason: FinishReason::Stop,
        structured_output: None,
        provider_context: Vec::new(),
        usage: ModelUsage::default(),
    }
}
