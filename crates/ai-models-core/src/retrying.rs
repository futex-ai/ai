//! Transient retry wrapper for model implementations.

use std::{sync::Arc, time::Duration};

use ai_interface::{Model, ModelError, ModelRequest, ModelResponse, ModelResult};
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
    async fn complete(&self, request: &ModelRequest) -> ModelResult<ModelResponse> {
        let mut retry_index = 0usize;

        loop {
            match self.inner.complete(request).await {
                Err(ModelError::TransientProvider { .. })
                    if retry_index < self.retry_delays.len() =>
                {
                    let delay = self.retry_delays[retry_index];
                    retry_index += 1;
                    self.sleeper.sleep(delay).await;
                }
                other => return other,
            }
        }
    }
}
