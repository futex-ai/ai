//! Xai public constructors over a model spec and the shared engine.

use std::sync::Arc;

use ai_interface::{DynModelEventStream, Model, ModelRequest, ModelResult, ModelSpec};
use ai_models_core::{Clock, ProviderConfig};
use async_trait::async_trait;
use json_http::{DynJsonHttpAuth, DynJsonHttpClient};

use crate::engine::CompatModel;

use super::{
    PROVIDER_ID,
    deferred::{DeferredRuntime, TokioDeferredRuntime, XaiDeferredCompletion},
    dialect::DIALECT,
};

#[derive(Clone)]
/// Xai-backed `ai_interface::Model` implementation.
pub struct XaiModel {
    inner: CompatModel,
}

impl XaiModel {
    /// Builds a model from a spec and shared provider connection settings.
    pub fn from_config(
        http_client: DynJsonHttpClient,
        spec: ModelSpec,
        config: ProviderConfig,
    ) -> Self {
        Self::from_config_with_runtime(http_client, spec, config, Arc::new(TokioDeferredRuntime))
    }

    /// Builds a Xai model from its spec and an API key.
    pub fn new(
        http_client: DynJsonHttpClient,
        spec: ModelSpec,
        api_key: impl Into<String>,
    ) -> Self {
        Self::from_config(http_client, spec, ProviderConfig::new(PROVIDER_ID, api_key))
    }

    /// Builds a Xai model from its spec and an auth hook.
    pub fn with_auth(
        http_client: DynJsonHttpClient,
        spec: ModelSpec,
        auth: DynJsonHttpAuth,
    ) -> Self {
        Self::with_catalog_auth(http_client, spec, auth)
    }

    /// Builds a Xai model from a catalog or custom spec and explicit auth.
    pub fn with_catalog_auth(
        http_client: DynJsonHttpClient,
        spec: ModelSpec,
        auth: DynJsonHttpAuth,
    ) -> Self {
        Self::with_catalog_auth_and_runtime(http_client, spec, auth, Arc::new(TokioDeferredRuntime))
    }

    pub(super) fn from_config_with_runtime(
        http_client: DynJsonHttpClient,
        spec: ModelSpec,
        config: ProviderConfig,
        runtime: Arc<dyn DeferredRuntime>,
    ) -> Self {
        let deferred_completion = Arc::new(XaiDeferredCompletion::new(
            http_client.clone(),
            config.clone(),
            runtime,
        ));
        Self {
            inner: CompatModel::from_config(&DIALECT, http_client, spec, config)
                .with_deferred(deferred_completion),
        }
    }

    pub(super) fn with_catalog_auth_and_runtime(
        http_client: DynJsonHttpClient,
        spec: ModelSpec,
        auth: DynJsonHttpAuth,
        runtime: Arc<dyn DeferredRuntime>,
    ) -> Self {
        Self::from_config_with_runtime(
            http_client,
            spec,
            ProviderConfig::with_auth(PROVIDER_ID, auth),
            runtime,
        )
    }

    /// Uses an injected wall clock to evaluate response retry dates.
    pub fn with_clock(mut self, clock: Arc<dyn Clock>) -> Self {
        self.inner = self.inner.with_clock(clock);
        self
    }
}

#[async_trait]
impl Model for XaiModel {
    async fn stream(&self, request: &ModelRequest) -> ModelResult<DynModelEventStream> {
        self.inner.stream(request).await
    }
}
