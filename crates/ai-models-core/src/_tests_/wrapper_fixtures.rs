//! In-memory model streams for wrapper requirements.

use std::{
    collections::VecDeque,
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
};

use ai_interface::{
    DynModel, DynModelEventStream, FinishReason, ModelEvent, ModelEventStreamMock, ModelMock,
    ModelRequest, ModelResponse, ModelResult, ModelUsage, test_support::model_response_stream,
};
use unimock::{MockFn, Unimock, matching};

pub(super) fn request() -> ModelRequest {
    ModelRequest {
        system_prompt: "system".to_owned(),
        messages: Vec::new(),
        tools: Vec::new(),
        response_schema: None,
        controls: Default::default(),
    }
}

pub(super) fn text(delta: &str) -> ModelEvent {
    ModelEvent::TextDelta {
        provider_state: None,
        part_index: 0,
        delta: delta.to_owned(),
    }
}

pub(super) fn response(message: &str) -> ModelResponse {
    ModelResponse {
        provider: "mock".to_owned(),
        model_id: "mock".to_owned(),
        catalog_model_id: None,
        thinking_level: None,
        parts: vec![ai_interface::ConversationPart::text(message.to_owned())],
        finish_reason: FinishReason::Stop,
        structured_output: None,

        usage: ModelUsage::default(),
        attempts: Vec::new(),
    }
}

pub(super) fn stream(items: Vec<ModelResult<Option<ModelEvent>>>) -> DynModelEventStream {
    let items = Mutex::new(VecDeque::from(items));
    Box::new(Unimock::new(
        ModelEventStreamMock::next
            .each_call(matching!())
            .answers_arc(Arc::new(move |_| {
                items.lock().unwrap().pop_front().unwrap_or(Ok(None))
            })),
    ))
}

pub(super) fn success(message: &str) -> DynModelEventStream {
    model_response_stream(response(message))
}

pub(super) fn scripted_model(
    attempts: Vec<ModelResult<DynModelEventStream>>,
    calls: Arc<AtomicUsize>,
) -> DynModel {
    let attempts = Mutex::new(VecDeque::from(attempts));
    Arc::new(Unimock::new(
        ModelMock::stream
            .each_call(matching!(_))
            .answers_arc(Arc::new(move |_, _| {
                calls.fetch_add(1, Ordering::SeqCst);
                attempts
                    .lock()
                    .unwrap()
                    .pop_front()
                    .expect("unexpected open")
            })),
    ))
}
