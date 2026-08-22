//! Ordered fallback model combinator over multiple `ai-interface` models.

#![warn(unreachable_pub)]

use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use ai_interface::{
    Model, ModelCompletionEvent, ModelCompletionEventSink, ModelError, ModelRequest, ModelResponse,
};
use async_trait::async_trait;
use thiserror::Error;

#[derive(Debug, Error)]
enum Error {
    #[error("[ai_models_multi] no models configured")]
    NoModelsConfigured,
}

#[derive(Clone, Default)]
/// Ordered fallback model that tries wrapped models in vector order.
pub struct MultiModel {
    models: Vec<Arc<dyn Model>>,
}

impl MultiModel {
    /// Builds a fallback model over the provided ordered model list.
    pub fn new(models: Vec<Arc<dyn Model>>) -> Self {
        Self { models }
    }
}

#[async_trait]
impl Model for MultiModel {
    async fn complete(
        &self,
        request: &ModelRequest,
    ) -> std::result::Result<ModelResponse, ModelError> {
        let mut last_error = None;

        for model in &self.models {
            match model.complete(request).await {
                Ok(response) => return Ok(response),
                Err(error) => last_error = Some(error),
            }
        }

        match last_error {
            Some(error) => Err(error),
            None => Err(ModelError::internal(Error::NoModelsConfigured)),
        }
    }

    async fn complete_with_events(
        &self,
        request: &ModelRequest,
        event_sink: &dyn ModelCompletionEventSink,
    ) -> std::result::Result<ModelResponse, ModelError> {
        let tracking_sink = TrackingEventSink::new(event_sink);
        let mut last_error = None;

        for (index, model) in self.models.iter().enumerate() {
            match model.complete_with_events(request, &tracking_sink).await {
                Ok(response) => return Ok(response),
                Err(error) => {
                    last_error = Some(error);
                    if index + 1 < self.models.len() && tracking_sink.has_public_text() {
                        tracking_sink
                            .emit(ModelCompletionEvent::AttemptRestarted)
                            .await;
                    }
                }
            }
        }

        match last_error {
            Some(error) => Err(error),
            None => Err(ModelError::internal(Error::NoModelsConfigured)),
        }
    }
}

struct TrackingEventSink<'a> {
    inner: &'a dyn ModelCompletionEventSink,
    has_public_text: AtomicBool,
}

impl<'a> TrackingEventSink<'a> {
    fn new(inner: &'a dyn ModelCompletionEventSink) -> Self {
        Self {
            inner,
            has_public_text: AtomicBool::new(false),
        }
    }

    fn has_public_text(&self) -> bool {
        self.has_public_text.load(Ordering::Relaxed)
    }
}

#[async_trait]
impl ModelCompletionEventSink for TrackingEventSink<'_> {
    async fn emit(&self, event: ModelCompletionEvent) {
        match &event {
            ModelCompletionEvent::AssistantTextDelta { .. }
            | ModelCompletionEvent::ReasoningTextDelta { .. } => {
                self.has_public_text.store(true, Ordering::Relaxed);
            }
            ModelCompletionEvent::AttemptRestarted => {
                self.has_public_text.store(false, Ordering::Relaxed);
            }
            _ => {}
        }
        self.inner.emit(event).await;
    }
}

#[cfg(test)]
#[path = "_tests_/multi_tests.rs"]
mod multi_tests;

#[cfg(test)]
#[path = "_tests_/multi_event_tests.rs"]
mod multi_event_tests;
