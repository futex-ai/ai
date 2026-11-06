//! Retry of eligible failures before the first model event.

use std::sync::Arc;

use ai_interface::{
    DynModelEventStream, ErrorKind, Model, ModelError, ModelEvent, ModelEventStream, ModelRequest,
    ModelResult,
};
use async_trait::async_trait;

use crate::{DynRetryRandom, DynSleeper, RetryConfig, SystemRetryRandom, TokioSleeper};

#[derive(Clone)]
/// Retries rate limits and transient failures before any model event.
/// Never repeats a composed model after earlier interruption or emitted events.
pub struct RetryingModel {
    inner: Arc<dyn Model>,
    sleeper: DynSleeper,
    config: RetryConfig,
    random: DynRetryRandom,
}

impl RetryingModel {
    /// Builds a retry layer with injected sleeping, randomness, and bounds.
    pub fn new(
        inner: Arc<dyn Model>,
        sleeper: DynSleeper,
        random: DynRetryRandom,
        config: RetryConfig,
    ) -> Self {
        Self {
            inner,
            sleeper,
            config,
            random,
        }
    }

    /// Uses standard retry bounds, Tokio sleeping, and system randomness.
    pub fn with_standard_retry(inner: Arc<dyn Model>) -> Self {
        Self::new(
            inner,
            Arc::new(TokioSleeper),
            Arc::new(SystemRetryRandom),
            RetryConfig::default(),
        )
    }
}

#[async_trait]
impl Model for RetryingModel {
    async fn stream(&self, request: &ModelRequest) -> ModelResult<DynModelEventStream> {
        let mut stream = RetryStream {
            inner: self.inner.clone(),
            sleeper: self.sleeper.clone(),
            config: self.config.clone(),
            random: self.random.clone(),
            retry_index: 0,
            request: request.clone(),
            stream: None,
            emitted_event: false,
            terminal: false,
        };
        stream.open().await?;
        Ok(Box::new(stream))
    }
}

struct RetryStream {
    inner: Arc<dyn Model>,
    sleeper: DynSleeper,
    config: RetryConfig,
    random: DynRetryRandom,
    retry_index: usize,
    request: ModelRequest,
    stream: Option<DynModelEventStream>,
    emitted_event: bool,
    terminal: bool,
}

impl RetryStream {
    async fn open(&mut self) -> ModelResult<()> {
        loop {
            match self.inner.stream(&self.request).await {
                Ok(stream) => {
                    self.stream = Some(stream);
                    return Ok(());
                }
                Err(error) => {
                    if !self.wait_for_retry(&error).await {
                        return Err(error);
                    }
                }
            }
        }
    }

    /// Earlier leaves can report progress before the composite opens its stream.
    async fn wait_for_retry(&mut self, error: &ModelError) -> bool {
        let core = error.core();
        if self.emitted_event
            || error
                .attempts()
                .iter()
                .any(|attempt| attempt.kind == ErrorKind::Interrupted || attempt.emitted_events)
            || !matches!(core.kind, ErrorKind::RateLimited | ErrorKind::Transient)
            || self.retry_index >= self.config.max_attempts.get() - 1
            || core
                .retry_after
                .is_some_and(|wait| wait > self.config.max_wait)
        {
            return false;
        }
        let Some(base) = self.config.base_delays.get(self.retry_index).copied() else {
            return false;
        };
        let random_wait = self.random.between(base / 2, base);
        let wait = random_wait.max(core.retry_after.unwrap_or_default());
        self.retry_index += 1;
        self.sleeper.sleep(wait).await;
        true
    }

    fn terminate(&mut self) {
        self.stream = None;
        self.terminal = true;
    }
}

#[async_trait]
impl ModelEventStream for RetryStream {
    async fn next(&mut self) -> ModelResult<Option<ModelEvent>> {
        if self.terminal {
            return Ok(None);
        }
        loop {
            let Some(stream) = &mut self.stream else {
                self.terminate();
                return Ok(None);
            };
            match stream.next().await {
                Ok(Some(event)) => {
                    self.emitted_event = true;
                    return Ok(Some(event));
                }
                Ok(None) => {
                    self.terminate();
                    return Ok(None);
                }
                Err(error) => {
                    self.stream = None;
                    if !self.wait_for_retry(&error).await {
                        self.terminate();
                        return Err(error);
                    }
                    if let Err(error) = self.open().await {
                        self.terminate();
                        return Err(error);
                    }
                }
            }
        }
    }
}
