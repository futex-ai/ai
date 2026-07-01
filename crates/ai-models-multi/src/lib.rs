//! Ordered fallback model combinator over multiple `ai-interface` models.

#![warn(unreachable_pub)]

use std::sync::Arc;

use ai_interface::{Model, ModelError, ModelRequest, ModelResponse};
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
}

#[cfg(test)]
#[path = "_tests_/multi_tests.rs"]
mod multi_tests;
