//! Model stream observation and credential-free live-event validation.

use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
};

use ai_interface::{
    DynModel, DynModelEventStream, Model, ModelError, ModelEvent, ModelEventStream, ModelRequest,
    ModelResult,
};
use ai_testkit::event_snapshot::ModelEventSnapshot;
use async_trait::async_trait;
use json_http::test_support::Redactor;

use super::provider_tests::CompletionEventExpectation;

#[derive(Clone, Default)]
pub(super) struct CompletionEventLog {
    events: Arc<Mutex<Vec<ModelEventSnapshot>>>,
}

impl CompletionEventLog {
    pub(super) fn recorded(&self) -> Vec<ModelEventSnapshot> {
        self.events.lock().expect("completion event lock").clone()
    }
}

struct EventObservingModel {
    inner: DynModel,
    events: CompletionEventLog,
}

#[async_trait]
impl Model for EventObservingModel {
    async fn stream(&self, request: &ModelRequest) -> ModelResult<DynModelEventStream> {
        Ok(Box::new(EventObservingStream {
            inner: Some(self.inner.stream(request).await?),
            events: self.events.clone(),
        }))
    }
}

struct EventObservingStream {
    inner: Option<DynModelEventStream>,
    events: CompletionEventLog,
}

#[async_trait]
impl ModelEventStream for EventObservingStream {
    async fn next(&mut self) -> ModelResult<Option<ModelEvent>> {
        let Some(inner) = &mut self.inner else {
            return Ok(None);
        };
        match inner.next().await {
            Ok(Some(event)) => {
                let snapshot = match ModelEventSnapshot::capture(&event, &Redactor::default()) {
                    Ok(snapshot) => snapshot,
                    Err(source) => {
                        self.inner = None;
                        return Err(ModelError::internal(source));
                    }
                };
                self.events
                    .events
                    .lock()
                    .expect("completion event lock")
                    .push(snapshot);
                Ok(Some(event))
            }
            result => {
                self.inner = None;
                result
            }
        }
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
    _expectation: CompletionEventExpectation,
    assistant_message: &str,
    events: &[ModelEventSnapshot],
) -> Vec<String> {
    let mut failures = Vec::new();
    let mut assistant_parts = BTreeMap::new();
    let mut assistant_event_count = 0;
    let mut usage_event_count = 0;
    let mut finish_event_count = 0;
    for event in events {
        match event {
            ModelEventSnapshot::TextDelta {
                part_index,
                delta,
                provider_state,
            } => {
                if !delta.is_empty() {
                    assistant_event_count += 1;
                }
                if delta.is_empty() && provider_state.is_none() {
                    failures.push(format!("{model_id}: emitted an empty assistant delta"));
                }
                assistant_parts
                    .entry(*part_index)
                    .or_insert_with(String::new)
                    .push_str(delta);
            }
            ModelEventSnapshot::ReasoningDelta {
                delta,
                provider_state,
                ..
            } if delta.is_empty() && provider_state.is_none() => {
                failures.push(format!("{model_id}: emitted an empty reasoning delta"))
            }
            ModelEventSnapshot::Fallback { .. } => failures.push(format!(
                "{model_id}: direct provider probe unexpectedly restarted"
            )),
            ModelEventSnapshot::Usage { .. } => usage_event_count += 1,
            ModelEventSnapshot::Finish { .. } => finish_event_count += 1,
            _ => {}
        }
    }
    let assistant_text = assistant_parts.into_values().collect::<String>();
    if assistant_event_count == 0 {
        failures.push(format!(
            "{model_id}: completion emitted no nonempty text delta"
        ));
    } else if assistant_text != assistant_message {
        failures.push(format!(
            "{model_id}: assistant events did not have terminal parity"
        ));
    }
    if usage_event_count != 1 {
        failures.push(format!(
            "{model_id}: completion did not emit exactly one usage event"
        ));
    }
    if finish_event_count != 1 {
        failures.push(format!(
            "{model_id}: completion did not emit exactly one finish event"
        ));
    }
    failures
}

#[cfg(test)]
#[path = "event_guard_tests.rs"]
mod event_guard_tests;
