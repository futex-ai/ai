//! Fallback events and complete ordered leaf-attempt histories.

use std::{collections::VecDeque, sync::Arc};

use ai_interface::{
    DynModelEventStream, Model, ModelAttempt, ModelError, ModelEvent, ModelEventStream,
    ModelRequest, ModelResult,
};
use async_trait::async_trait;

use super::FallbackPolicy;

/// Retains global history while tracking the active inner leaf separately.
pub(super) struct FallbackStream {
    models: Vec<Arc<dyn Model>>,
    policy: Arc<dyn FallbackPolicy>,
    next_model: usize,
    request: ModelRequest,
    inner: Option<DynModelEventStream>,
    pending: VecDeque<ModelEvent>,
    attempts: Vec<ModelAttempt>,
    forwarded: usize,
    emitted_events: bool,
    terminal: bool,
}

impl FallbackStream {
    /// Starts a call before its first candidate has opened.
    pub(super) fn new(
        models: Vec<Arc<dyn Model>>,
        policy: Arc<dyn FallbackPolicy>,
        request: ModelRequest,
    ) -> Self {
        Self {
            models,
            policy,
            next_model: 0,
            request,
            inner: None,
            pending: VecDeque::new(),
            attempts: Vec::new(),
            forwarded: 0,
            emitted_events: false,
            terminal: false,
        }
    }

    /// Opens a stream or returns the final failure with earlier attempts.
    pub(super) async fn open_first(&mut self) -> ModelResult<()> {
        while self.inner.is_none() && self.next_model < self.models.len() {
            self.open_next().await?;
        }
        Ok(())
    }

    async fn open_next(&mut self) -> ModelResult<()> {
        let Some(model) = self.models.get(self.next_model).cloned() else {
            return Ok(());
        };
        self.next_model += 1;
        self.forwarded = 0;
        self.emitted_events = false;
        match model.stream(&self.request).await {
            Ok(inner) => {
                self.inner = Some(inner);
                Ok(())
            }
            Err(error) => self.failed(error),
        }
    }

    /// Inner histories start with the fallback records already forwarded.
    /// Skip their count, so equal but distinct later records remain intact.
    fn failed(&mut self, mut error: ModelError) -> ModelResult<()> {
        let unseen: Vec<_> = error
            .take_attempts()
            .into_iter()
            .skip(self.forwarded)
            .collect();
        self.attempts.extend(unseen.iter().cloned());
        let allowed = self.policy.should_fallback(error.core().kind);
        if !allowed || self.next_model == self.models.len() {
            return Err(error.with_attempts(std::mem::take(&mut self.attempts)));
        }
        for attempt in unseen {
            let emitted_events = attempt.emitted_events;
            self.pending.push_back(ModelEvent::Fallback {
                error: ModelError::from_attempt(attempt),
                emitted_events,
            });
        }
        self.attempts.push(error.attempt(self.emitted_events));
        self.pending.push_back(ModelEvent::Fallback {
            error,
            emitted_events: self.emitted_events,
        });
        Ok(())
    }

    fn terminate(&mut self) {
        self.inner = None;
        self.pending.clear();
        self.terminal = true;
    }
}

#[async_trait]
impl ModelEventStream for FallbackStream {
    async fn next(&mut self) -> ModelResult<Option<ModelEvent>> {
        if self.terminal {
            return Ok(None);
        }
        loop {
            if let Some(event) = self.pending.pop_front() {
                return Ok(Some(event));
            }
            if self.inner.is_none() {
                if self.next_model == self.models.len() {
                    self.terminate();
                    return Ok(None);
                }
                if let Err(error) = self.open_next().await {
                    self.terminate();
                    return Err(error);
                }
                continue;
            }
            let Some(inner) = &mut self.inner else {
                continue;
            };
            match inner.next().await {
                Ok(Some(event)) => {
                    match &event {
                        ModelEvent::Fallback {
                            error,
                            emitted_events,
                        } => {
                            self.attempts.push(error.attempt(*emitted_events));
                            self.forwarded += 1;
                            self.emitted_events = false;
                        }
                        _ => self.emitted_events = true,
                    }
                    return Ok(Some(event));
                }
                Ok(None) => {
                    self.terminate();
                    return Ok(None);
                }
                Err(error) => {
                    self.inner = None;
                    if let Err(error) = self.failed(error) {
                        self.terminate();
                        return Err(error);
                    }
                }
            }
        }
    }
}
