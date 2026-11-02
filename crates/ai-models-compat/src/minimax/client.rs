//! MiniMax public constructors over a model spec and the shared engine.

use std::sync::Arc;

use ai_interface::{DynModelEventStream, Model, ModelRequest, ModelResult, ModelSpec};
use ai_models_core::Clock;
use async_trait::async_trait;
use json_http::{DynJsonHttpAuth, DynJsonHttpClient, StaticHeaderAuth};

use crate::engine::CompatModel;

use super::dialect::DIALECT;

#[derive(Clone)]
/// MiniMax-backed `ai_interface::Model` implementation.
pub struct MiniMaxModel {
    inner: CompatModel,
}

impl MiniMaxModel {
    /// Builds a MiniMax model from its spec and an API key.
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

    /// Builds a MiniMax model from its spec and an auth hook.
    pub fn with_auth(
        http_client: DynJsonHttpClient,
        spec: ModelSpec,
        auth: DynJsonHttpAuth,
    ) -> Self {
        Self::with_catalog_auth(http_client, spec, auth)
    }

    /// Builds a MiniMax model from a catalog or custom spec and explicit auth.
    pub fn with_catalog_auth(
        http_client: DynJsonHttpClient,
        spec: ModelSpec,
        auth: DynJsonHttpAuth,
    ) -> Self {
        Self {
            inner: CompatModel::new(&DIALECT, http_client, spec, auth),
        }
    }

    /// Uses an injected wall clock to evaluate response retry dates.
    pub fn with_clock(mut self, clock: Arc<dyn Clock>) -> Self {
        self.inner = self.inner.with_clock(clock);
        self
    }
}

#[async_trait]
impl Model for MiniMaxModel {
    async fn stream(&self, request: &ModelRequest) -> ModelResult<DynModelEventStream> {
        self.inner.stream(request).await
    }
}
