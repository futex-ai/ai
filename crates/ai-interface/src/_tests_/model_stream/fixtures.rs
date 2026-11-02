//! Fixtures for model stream boundary tests.

use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

use serde_json::json;
use unimock::{MockFn, Unimock, matching};

use crate::{
    ConversationMessage, DynModelEventStream, Effort, FinishReason, ModelEvent,
    ModelEventStreamMock, ModelMock, ModelRequest, ModelResponse, ModelResponseFold, ModelResult,
    ModelUsage,
};

pub(super) fn request() -> ModelRequest {
    ModelRequest {
        system_prompt: "Be concise.".to_owned(),
        messages: vec![ConversationMessage::user("- body: inspect the queue")],
        tools: Vec::new(),
        response_schema: None,
        controls: Default::default(),
    }
}

pub(super) fn text(part_index: usize, delta: &str) -> ModelEvent {
    ModelEvent::TextDelta {
        part_index,
        delta: delta.to_owned(),
        provider_state: None,
    }
}

pub(super) fn reasoning(part_index: usize, delta: &str) -> ModelEvent {
    ModelEvent::ReasoningDelta {
        part_index,
        delta: delta.to_owned(),
        provider_state: None,
    }
}

pub(super) fn tool_start(part_index: usize, id: &str) -> ModelEvent {
    ModelEvent::ToolCallStart {
        part_index,
        id: id.to_owned(),
        name: "lookup".to_owned(),
        operation_id: Some(format!("operation-{id}")),
    }
}

pub(super) fn arguments(part_index: usize, delta: &str) -> ModelEvent {
    ModelEvent::ToolCallArgumentsDelta {
        part_index,
        delta: delta.to_owned(),
    }
}

pub(super) fn usage() -> ModelEvent {
    ModelEvent::Usage {
        usage: ModelUsage {
            input_tokens: 3,
            output_tokens: 7,
            total_tokens: 10,
            ..Default::default()
        },
    }
}

pub(super) fn finish(finish_reason: FinishReason) -> ModelEvent {
    ModelEvent::Finish {
        finish_reason,
        provider: "mock".to_owned(),
        model_id: "mock-dev".to_owned(),
        catalog_model_id: Some("mock/mock-dev".to_owned()),
        thinking_level: Some(Effort::High),
        structured_output: Some(json!({"answer": "ready"})),
    }
}

pub(super) fn fold(events: Vec<ModelEvent>) -> ModelResult<ModelResponse> {
    let mut fold = ModelResponseFold::new();
    for event in events {
        fold.push(event)?;
    }
    fold.finish()
}

pub(super) fn mocked_stream(results: Vec<ModelResult<Option<ModelEvent>>>) -> DynModelEventStream {
    let count = results.len();
    let results = Mutex::new(VecDeque::from(results));
    Box::new(Unimock::new(
        ModelEventStreamMock::next
            .each_call(matching!())
            .answers_arc(Arc::new(move |_| {
                results
                    .lock()
                    .unwrap()
                    .pop_front()
                    .expect("unexpected read")
            }))
            .n_times(count),
    ))
}

pub(super) fn model_for(results: Vec<ModelResult<Option<ModelEvent>>>) -> Unimock {
    let stream = Mutex::new(Some(mocked_stream(results)));
    Unimock::new(
        ModelMock::stream
            .some_call(matching!(_))
            .answers_arc(Arc::new(move |_, _| {
                Ok(stream.lock().unwrap().take().expect("one stream open"))
            })),
    )
}
