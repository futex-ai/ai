//! Tests for ordered fallback completion events.

use std::sync::{Arc, Mutex};

use ai_interface::{
    FinishReason, Model, ModelCompletionEvent, ModelCompletionEventSink,
    ModelCompletionEventSinkMock, ModelError, ModelRequest, ModelResponse, ModelResult, ModelUsage,
};
use async_trait::async_trait;
use unimock::{MockFn, Unimock, matching};

use crate::MultiModel;

#[tokio::test]
async fn pre_delta_fallback_does_not_emit_a_restart() {
    let model = MultiModel::new(vec![
        event_model(Vec::new(), Err(provider_error("first"))),
        event_model(vec![assistant_delta("done")], Ok(success_response("done"))),
    ]);
    let events = Arc::new(Mutex::new(Vec::new()));
    let sink = recording_sink(events.clone());

    model
        .complete_with_events(&empty_request(), &sink)
        .await
        .expect("fallback should succeed");

    assert_eq!(recorded_events(&events), vec![assistant_delta("done")]);
}

#[tokio::test]
async fn post_delta_fallback_emits_restart_before_the_next_lane() {
    let model = MultiModel::new(vec![
        event_model(
            vec![assistant_delta("partial"), reasoning_delta("thought")],
            Err(provider_error("first")),
        ),
        event_model(vec![assistant_delta("done")], Ok(success_response("done"))),
    ]);
    let events = Arc::new(Mutex::new(Vec::new()));
    let sink = recording_sink(events.clone());

    let response = model
        .complete_with_events(&empty_request(), &sink)
        .await
        .expect("fallback should succeed");

    assert_eq!(
        recorded_events(&events),
        vec![
            assistant_delta("partial"),
            reasoning_delta("thought"),
            ModelCompletionEvent::AttemptRestarted,
            assistant_delta("done"),
        ]
    );
    assert_eq!(response.assistant_message, "done");
}

#[tokio::test]
async fn final_lane_failure_does_not_emit_a_restart() {
    let model = MultiModel::new(vec![event_model(
        vec![assistant_delta("partial")],
        Err(provider_error("only")),
    )]);
    let events = Arc::new(Mutex::new(Vec::new()));
    let sink = recording_sink(events.clone());

    model
        .complete_with_events(&empty_request(), &sink)
        .await
        .expect_err("only lane should fail");

    assert_eq!(recorded_events(&events), vec![assistant_delta("partial")]);
}

#[tokio::test]
async fn nested_restart_resets_outer_public_text_tracking() {
    let model = MultiModel::new(vec![
        event_model(
            vec![
                assistant_delta("discard"),
                ModelCompletionEvent::AttemptRestarted,
            ],
            Err(provider_error("nested")),
        ),
        event_model(vec![assistant_delta("done")], Ok(success_response("done"))),
    ]);
    let events = Arc::new(Mutex::new(Vec::new()));
    let sink = recording_sink(events.clone());

    model
        .complete_with_events(&empty_request(), &sink)
        .await
        .expect("outer fallback should succeed");

    assert_eq!(
        recorded_events(&events),
        vec![
            assistant_delta("discard"),
            ModelCompletionEvent::AttemptRestarted,
            assistant_delta("done"),
        ]
    );
}

struct EventModel {
    events: Vec<ModelCompletionEvent>,
    result: Mutex<Option<ModelResult<ModelResponse>>>,
}

#[async_trait]
impl Model for EventModel {
    async fn complete(&self, _request: &ModelRequest) -> ModelResult<ModelResponse> {
        self.take_result()
    }

    async fn complete_with_events(
        &self,
        _request: &ModelRequest,
        event_sink: &dyn ModelCompletionEventSink,
    ) -> ModelResult<ModelResponse> {
        for event in self.events.clone() {
            event_sink.emit(event).await;
        }
        self.take_result()
    }
}

impl EventModel {
    fn take_result(&self) -> ModelResult<ModelResponse> {
        self.result
            .lock()
            .expect("result lock should not be poisoned")
            .take()
            .expect("unexpected model call")
    }
}

fn event_model(
    events: Vec<ModelCompletionEvent>,
    result: ModelResult<ModelResponse>,
) -> Arc<dyn Model> {
    Arc::new(EventModel {
        events,
        result: Mutex::new(Some(result)),
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

fn reasoning_delta(delta: &str) -> ModelCompletionEvent {
    ModelCompletionEvent::ReasoningTextDelta {
        delta: delta.to_owned(),
    }
}

fn provider_error(model_id: &str) -> ModelError {
    ModelError::provider("mock", model_id, "failed")
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

fn success_response(message: &str) -> ModelResponse {
    ModelResponse {
        provider: "mock".to_owned(),
        model_id: "mock".to_owned(),
        catalog_model_id: None,
        thinking_level: None,
        assistant_message: message.to_owned(),
        tool_calls: Vec::new(),
        finish_reason: FinishReason::Stop,
        structured_output: None,
        provider_context: Vec::new(),
        usage: ModelUsage::default(),
    }
}
