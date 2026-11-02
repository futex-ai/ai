//! DeepSeek public constructors over a model spec and the shared engine.

use std::sync::Arc;

use ai_interface::{DynModelEventStream, Model, ModelRequest, ModelResult, ModelSpec};
use ai_models_core::{Clock, ProviderConfig};
use async_trait::async_trait;
use json_http::{DynJsonHttpAuth, DynJsonHttpClient};

#[cfg(test)]
use ai_interface::ModelError;
#[cfg(test)]
use ai_models_core::{ErrorContext, SystemClock};

use crate::engine::CompatModel;

use super::{DEEPSEEK_V4_PRO, PROVIDER_ID, dialect::DIALECT};

#[cfg(test)]
const PROVIDER: &str = "deepseek";

#[derive(Clone)]
/// DeepSeek-backed `ai_interface::Model` implementation.
pub struct DeepSeekModel {
    inner: CompatModel,
}

impl DeepSeekModel {
    /// Builds a model from a spec and shared provider connection settings.
    pub fn from_config(
        http_client: DynJsonHttpClient,
        spec: ModelSpec,
        config: ProviderConfig,
    ) -> Self {
        Self {
            inner: CompatModel::from_config(&DIALECT, http_client, spec, config),
        }
    }

    /// Builds the default DeepSeek model from an API key.
    pub fn new(http_client: DynJsonHttpClient, api_key: impl Into<String>) -> Self {
        Self::from_config(
            http_client,
            DEEPSEEK_V4_PRO,
            ProviderConfig::new(PROVIDER_ID, api_key),
        )
    }

    /// Builds the default DeepSeek model from an auth hook.
    pub fn with_auth(http_client: DynJsonHttpClient, auth: DynJsonHttpAuth) -> Self {
        Self::with_catalog_auth(http_client, DEEPSEEK_V4_PRO, auth)
    }

    /// Builds a DeepSeek model from a catalog or custom spec and explicit auth.
    pub fn with_catalog_auth(
        http_client: DynJsonHttpClient,
        spec: ModelSpec,
        auth: DynJsonHttpAuth,
    ) -> Self {
        Self::from_config(
            http_client,
            spec,
            ProviderConfig::with_auth(PROVIDER_ID, auth),
        )
    }

    /// Uses an injected wall clock to evaluate response retry dates.
    pub fn with_clock(mut self, clock: Arc<dyn Clock>) -> Self {
        self.inner = self.inner.with_clock(clock);
        self
    }
}

#[async_trait]
impl Model for DeepSeekModel {
    async fn stream(&self, request: &ModelRequest) -> ModelResult<DynModelEventStream> {
        self.inner.stream(request).await
    }
}

#[cfg(test)]
pub(super) fn request_error(source: json_http::Error, model_id: &str) -> ModelError {
    let context = ErrorContext::new(PROVIDER, model_id, &[], Arc::new(SystemClock));
    ModelError::with_source(context.request_error(&source), source)
}
