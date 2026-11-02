//! Fallback during stream reads with one event for each discarded attempt.

use std::{collections::VecDeque, sync::Arc};

use ai_interface::{
    DynModelEventStream, Model, ModelError, ModelEvent, ModelEventStream, ModelRequest, ModelResult,
};
use async_trait::async_trait;

/// Stream state that advances lanes only after their current attempt fails.
pub(crate) struct FallbackStream {
    models: Vec<Arc<dyn Model>>,
    next_model: usize,
    request: ModelRequest,
    inner: Option<DynModelEventStream>,
    errors: VecDeque<ModelError>,
    terminal: bool,
}

impl FallbackStream {
    /// Retains a successfully opened lane and the failures that preceded it.
    pub(crate) fn new(
        models: Vec<Arc<dyn Model>>,
        next_model: usize,
        request: ModelRequest,
        inner: DynModelEventStream,
        errors: VecDeque<ModelError>,
    ) -> Self {
        Self {
            models,
            next_model,
            request,
            inner: Some(inner),
            errors,
            terminal: false,
        }
    }
}

#[async_trait]
impl ModelEventStream for FallbackStream {
    async fn next(&mut self) -> ModelResult<Option<ModelEvent>> {
        if self.terminal {
            return Ok(None);
        }
        if let Some(error) = self.errors.pop_front() {
            return Ok(Some(ModelEvent::Fallback { error }));
        }
        loop {
            if self.inner.is_none() {
                let Some(model) = self.models.get(self.next_model) else {
                    self.terminal = true;
                    return Ok(None);
                };
                self.next_model += 1;
                match model.stream(&self.request).await {
                    Ok(inner) => self.inner = Some(inner),
                    Err(error) if self.next_model < self.models.len() => {
                        return Ok(Some(ModelEvent::Fallback { error }));
                    }
                    Err(error) => {
                        self.terminal = true;
                        return Err(error);
                    }
                }
            }
            let Some(inner) = &mut self.inner else {
                continue;
            };
            match inner.next().await {
                Ok(Some(event)) => return Ok(Some(event)),
                Ok(None) => {
                    self.inner = None;
                    self.terminal = true;
                    return Ok(None);
                }
                Err(error) => {
                    self.inner = None;
                    if self.next_model < self.models.len() {
                        return Ok(Some(ModelEvent::Fallback { error }));
                    }
                    self.terminal = true;
                    return Err(error);
                }
            }
        }
    }
}
