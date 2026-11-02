//! Per-model concurrency limiter wrapper.

use std::sync::Arc;

use ai_interface::{
    DynModelEventStream, Model, ModelError, ModelEvent, ModelEventStream, ModelRequest, ModelResult,
};
use async_trait::async_trait;
use thiserror::Error;
use tokio::sync::{OwnedSemaphorePermit, Semaphore};

#[derive(Debug, Error)]
enum Error {
    #[error("[ai_models_policy/concurrency] concurrency limiter closed for model `{model_id}`")]
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
    async fn stream(&self, request: &ModelRequest) -> ModelResult<DynModelEventStream> {
        let permit = match &self.semaphore {
            Some(semaphore) => match semaphore.clone().acquire_owned().await {
                Ok(permit) => Some(permit),
                Err(_) => {
                    return Err(ModelError::internal(
                        "local",
                        &self.model_id,
                        Error::ConcurrencyLimiterClosed {
                            model_id: self.model_id.clone(),
                        },
                    ));
                }
            },
            None => None,
        };
        let inner = self.inner.stream(request).await?;
        Ok(Box::new(ConcurrencyStream {
            inner: Some(inner),
            permit,
        }))
    }
}

struct ConcurrencyStream {
    inner: Option<DynModelEventStream>,
    permit: Option<OwnedSemaphorePermit>,
}

#[async_trait]
impl ModelEventStream for ConcurrencyStream {
    async fn next(&mut self) -> ModelResult<Option<ModelEvent>> {
        let Some(inner) = &mut self.inner else {
            return Ok(None);
        };
        let result = inner.next().await;
        if !matches!(&result, Ok(Some(_))) {
            self.inner = None;
            self.permit = None;
        }
        result
    }
}
