//! Transient retry wrapper for model implementations.

use std::{sync::Arc, time::Duration};

use ai_interface::{
    DynModelEventStream, ErrorKind, Model, ModelError, ModelEvent, ModelEventStream, ModelRequest,
    ModelResult,
};
use async_trait::async_trait;

use crate::{DynSleeper, TokioSleeper};

/// Default transient retry schedule: `100ms`, then `250ms`.
pub const STANDARD_TRANSIENT_RETRY_DELAYS: [Duration; 2] =
    [Duration::from_millis(100), Duration::from_millis(250)];

#[derive(Clone)]
/// Wrapper that retries transient model failures before returning the last error.
pub struct RetryingModel {
    inner: Arc<dyn Model>,
    sleeper: DynSleeper,
    retry_delays: Vec<Duration>,
}

impl RetryingModel {
    /// Builds a retrying wrapper with the provided sleeper and delay schedule.
    pub fn new(inner: Arc<dyn Model>, sleeper: DynSleeper, retry_delays: Vec<Duration>) -> Self {
        Self {
            inner,
            sleeper,
            retry_delays,
        }
    }

    /// Builds a retrying wrapper that preserves the standard transient schedule.
    pub fn with_standard_transient_retry(inner: Arc<dyn Model>) -> Self {
        Self::new(
            inner,
            Arc::new(TokioSleeper),
            STANDARD_TRANSIENT_RETRY_DELAYS.to_vec(),
        )
    }
}

#[async_trait]
impl Model for RetryingModel {
    async fn stream(&self, request: &ModelRequest) -> ModelResult<DynModelEventStream> {
        let mut stream = RetryStream {
            inner: self.inner.clone(),
            sleeper: self.sleeper.clone(),
            retry_delays: self.retry_delays.clone(),
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
    retry_delays: Vec<Duration>,
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

    async fn wait_for_retry(&mut self, error: &ModelError) -> bool {
        if self.emitted_event || error.core().kind != ErrorKind::Transient {
            return false;
        }
        let Some(delay) = self.retry_delays.get(self.retry_index).copied() else {
            return false;
        };
        self.retry_index += 1;
        self.sleeper.sleep(delay).await;
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
