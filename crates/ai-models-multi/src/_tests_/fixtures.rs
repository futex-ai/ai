//! Model fixtures and typed event observations for fallback tests.

use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

use ai_interface::{
    DynModel, DynModelEventStream, FinishReason, Model, ModelAttempt, ModelEvent,
    ModelEventStreamMock, ModelMock, ModelRequest, ModelResponse, ModelResponseFold, ModelResult,
    ModelUsage, test_support::model_response_stream,
};
use unimock::{MockFn, Unimock, matching};

#[derive(Debug, PartialEq)]
pub(super) enum Observed {
    Text(usize, String),
    Reasoning(usize, String),
    Fallback(ModelAttempt),
    Usage(ModelUsage),
    Finish(FinishReason, String, String),
    Other,
}

pub(super) fn observe(event: &ModelEvent) -> Observed {
    match event {
        ModelEvent::TextDelta { part_index, delta } => Observed::Text(*part_index, delta.clone()),
        ModelEvent::ReasoningDelta { part_index, delta } => {
            Observed::Reasoning(*part_index, delta.clone())
        }
        ModelEvent::Fallback { error } => Observed::Fallback(ModelAttempt::from(error)),
        ModelEvent::Usage { usage } => Observed::Usage(usage.clone()),
        ModelEvent::Finish {
            finish_reason,
            provider,
            model_id,
            ..
        } => Observed::Finish(finish_reason.clone(), provider.clone(), model_id.clone()),
        _ => Observed::Other,
    }
}

pub(super) async fn collect(model: &dyn Model) -> (ModelResult<ModelResponse>, Vec<Observed>) {
    let mut stream = match model.stream(&request()).await {
        Ok(stream) => stream,
        Err(error) => return (Err(error), Vec::new()),
    };
    let mut fold = ModelResponseFold::new();
    let mut events = Vec::new();
    loop {
        match stream.next().await {
            Ok(Some(event)) => {
                events.push(observe(&event));
                if let Err(error) = fold.push(event) {
                    return (Err(error), events);
                }
            }
            Ok(None) => return (fold.finish(), events),
            Err(error) => return (Err(error), events),
        }
    }
}

pub(super) fn request() -> ModelRequest {
    ModelRequest {
        system_prompt: "system".to_owned(),
        messages: Vec::new(),
        tools: Vec::new(),
        response_schema: None,
        controls: Default::default(),
    }
}

pub(super) fn response(message: &str) -> ModelResponse {
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
        attempts: Vec::new(),
    }
}

pub(super) fn text(delta: &str) -> ModelEvent {
    ModelEvent::TextDelta {
        part_index: 0,
        delta: delta.to_owned(),
    }
}

pub(super) fn success(message: &str) -> DynModelEventStream {
    model_response_stream(response(message))
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

pub(super) fn model(opens: Vec<ModelResult<DynModelEventStream>>) -> DynModel {
    let opens = Mutex::new(VecDeque::from(opens));
    Arc::new(Unimock::new(
        ModelMock::stream
            .each_call(matching!(_))
            .answers_arc(Arc::new(move |_, _| {
                opens.lock().unwrap().pop_front().expect("unexpected open")
            })),
    ))
}
