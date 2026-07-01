//! Per-model concurrency limiter wrapper.

use std::sync::Arc;

use ai_interface::{Model, ModelError, ModelRequest, ModelResponse, ModelResult};
use async_trait::async_trait;
use thiserror::Error;
use tokio::sync::Semaphore;

#[derive(Debug, Error)]
enum Error {
    #[error("[ai_models_core/concurrency] concurrency limiter closed for model `{model_id}`")]
    ConcurrencyLimiterClosed { model_id: String },
}

#[derive(Clone)]
/// Wrapper that enforces a maximum number of in-flight calls for one model.
pub struct ConcurrencyLimitedModel {
    inner: Arc<dyn Model>,
    model_id: String,
    semaphore: Option<Arc<Semaphore>>,
}

impl ConcurrencyLimitedModel {
    /// Builds a wrapper for the provided model and concurrency limit.
    pub fn new(inner: Arc<dyn Model>, model_id: impl Into<String>, max_concurrent: u32) -> Self {
        Self {
            inner,
            model_id: model_id.into(),
            semaphore: (max_concurrent > 0)
                .then(|| Arc::new(Semaphore::new(max_concurrent as usize))),
        }
    }
}

#[async_trait]
impl Model for ConcurrencyLimitedModel {
    async fn complete(&self, request: &ModelRequest) -> ModelResult<ModelResponse> {
        let Some(semaphore) = &self.semaphore else {
            return self.inner.complete(request).await;
        };
        let _permit = semaphore.clone().acquire_owned().await.map_err(|_| {
            ModelError::internal(Error::ConcurrencyLimiterClosed {
                model_id: self.model_id.clone(),
            })
        })?;
        self.inner.complete(request).await
    }
}
