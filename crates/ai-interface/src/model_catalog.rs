//! Local model lookup by case-sensitive provider/model text ids.

use crate::ModelSpec;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
/// Combined provider-owned model specs in insertion order.
pub struct ModelCatalog {
    models: Vec<ModelSpec>,
}

#[cfg(test)]
#[path = "_tests_/model_catalog_tests.rs"]
mod model_catalog_tests;

impl ModelCatalog {
    /// Builds an empty model catalog.
    pub fn new() -> Self {
        Self::default()
    }

    /// Appends provider specs to the catalog.
    pub fn with_models(mut self, models: impl IntoIterator<Item = ModelSpec>) -> Self {
        self.models.extend(models);
        self
    }

    /// Returns the specs in insertion order.
    pub fn all(&self) -> &[ModelSpec] {
        &self.models
    }

    /// Finds a text id, splitting the provider at the first slash.
    pub fn find(&self, text_id: &str) -> Option<&ModelSpec> {
        let (provider, model_id) = text_id.split_once('/')?;
        self.models.iter().find(|model| {
            model.provider.as_str() == provider && model.provider_model_id.as_ref() == model_id
        })
    }
}
