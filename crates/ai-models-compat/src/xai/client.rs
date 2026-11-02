//! Xai public constructors over a model spec and the shared engine.

use std::sync::Arc;

use ai_interface::{DynModelEventStream, Model, ModelRequest, ModelResult, ModelSpec};
use async_trait::async_trait;
use json_http::{DynJsonHttpAuth, DynJsonHttpClient, StaticHeaderAuth};

use crate::engine::CompatModel;

use super::{
    deferred::{DeferredRuntime, TokioDeferredRuntime, XaiDeferredCompletion},
    dialect::DIALECT,
};

#[derive(Clone)]
/// Xai-backed `ai_interface::Model` implementation.
pub struct XaiModel {
    inner: CompatModel,
}

impl XaiModel {
    /// Builds a Xai model from its spec and an API key.
    pub fn new(
        http_client: DynJsonHttpClient,
        spec: ModelSpec,
        api_key: impl Into<String>,
    ) -> Self {
        Self::with_auth(
            http_client,
            spec,
            Arc::new(StaticHeaderAuth::bearer_token(api_key)),
        )
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

    pub(super) fn with_catalog_auth_and_runtime(
        http_client: DynJsonHttpClient,
        spec: ModelSpec,
        auth: DynJsonHttpAuth,
        runtime: Arc<dyn DeferredRuntime>,
    ) -> Self {
        let deferred_completion = Arc::new(XaiDeferredCompletion::new(
            http_client.clone(),
            auth.clone(),
            spec.provider_model_id.to_string(),
            runtime,
        ));
        Self {
            inner: CompatModel::new(&DIALECT, http_client, spec, auth)
                .with_deferred(deferred_completion),
        }
    }
}

#[async_trait]
impl Model for XaiModel {
    async fn stream(&self, request: &ModelRequest) -> ModelResult<DynModelEventStream> {
        self.inner.stream(request).await
    }
}
