//! Ordered fallback model and initial stream opening.

use std::{collections::VecDeque, sync::Arc};

use ai_interface::{DynModelEventStream, Model, ModelError, ModelRequest, ModelResult};
use async_trait::async_trait;
use thiserror::Error;

use crate::stream::FallbackStream;

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
    async fn stream(&self, request: &ModelRequest) -> ModelResult<DynModelEventStream> {
        let mut errors = VecDeque::new();
        for (index, model) in self.models.iter().enumerate() {
            match model.stream(request).await {
                Ok(inner) => {
                    return Ok(Box::new(FallbackStream::new(
                        self.models.clone(),
                        index + 1,
                        request.clone(),
                        inner,
                        errors,
                    )));
                }
                Err(error) if index + 1 == self.models.len() => return Err(error),
                Err(error) => errors.push_back(error),
            }
        }
        Err(ModelError::internal(Error::NoModelsConfigured))
    }
}
