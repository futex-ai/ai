//! Explicit-policy fallback model and initial stream opening.

use std::sync::Arc;

use ai_interface::{DynModelEventStream, Model, ModelError, ModelRequest, ModelResult};
use async_trait::async_trait;
use thiserror::Error;

use super::{FallbackPolicy, stream::FallbackStream};

#[derive(Debug, Error)]
enum Error {
    #[error("[ai_models_policy/fallback] no models configured")]
    NoModelsConfigured,
}

#[derive(Clone)]
/// Tries models in order while the required failure policy permits it.
pub struct FallbackModel {
    models: Vec<Arc<dyn Model>>,
    policy: Arc<dyn FallbackPolicy>,
}

impl FallbackModel {
    /// Builds an ordered chain with an explicit kind-based fallback policy.
    pub fn new(models: Vec<Arc<dyn Model>>, policy: Arc<dyn FallbackPolicy>) -> Self {
        Self { models, policy }
    }
}

#[async_trait]
impl Model for FallbackModel {
    async fn stream(&self, request: &ModelRequest) -> ModelResult<DynModelEventStream> {
        if self.models.is_empty() {
            return Err(ModelError::internal(
                "local",
                "fallback",
                Error::NoModelsConfigured,
            ));
        }
        let mut stream =
            FallbackStream::new(self.models.clone(), self.policy.clone(), request.clone());
        stream.open_first().await?;
        Ok(Box::new(stream))
    }
}
