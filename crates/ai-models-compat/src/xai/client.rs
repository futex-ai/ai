//! xAI public constructors over the shared engine.

use super::{
    catalog::known_models,
    deferred::{DeferredRuntime, TokioDeferredRuntime, XaiDeferredCompletion},
    dialect::DIALECT,
};
use crate::engine::CompatModel;
use ai_interface::{
    Model, ModelCompletionEventSink, ModelRequest, ModelResponse, ModelResult, ProviderKind,
};
use ai_models_core::{ThinkingLevel, resolve_catalog_thinking_level};
use async_trait::async_trait;
use json_http::{DynJsonHttpAuth, DynJsonHttpClient, StaticHeaderAuth};
use std::sync::Arc;

#[derive(Clone)]
/// xAI-backed `ai_interface::Model` implementation.
pub struct XaiModel {
    inner: CompatModel,
}

impl XaiModel {
    /// Builds an xAI model from an explicit API key.
    pub fn new(
        http_client: DynJsonHttpClient,
        model_id: impl Into<String>,
        api_key: impl Into<String>,
    ) -> Self {
        Self::with_auth(
            http_client,
            model_id,
            Arc::new(StaticHeaderAuth::bearer_token(api_key)),
        )
    }

    /// Builds an xAI model from an explicit auth hook.
    pub fn with_auth(
        http_client: DynJsonHttpClient,
        model_id: impl Into<String>,
        auth: DynJsonHttpAuth,
    ) -> Self {
        let model_id = model_id.into();
        Self::with_catalog_auth(
            http_client,
            model_id.clone(),
            model_id,
            ThinkingLevel::Disabled,
            auth,
        )
    }

    /// Builds an xAI model from catalog metadata and an explicit auth hook.
    pub fn with_catalog_auth(
        http_client: DynJsonHttpClient,
        catalog_model_id: impl Into<String>,
        provider_model_id: impl Into<String>,
        thinking_level: ThinkingLevel,
        auth: DynJsonHttpAuth,
    ) -> Self {
        Self::with_catalog_auth_and_runtime(
            http_client,
            catalog_model_id,
            provider_model_id,
            thinking_level,
            auth,
            Arc::new(TokioDeferredRuntime),
        )
    }

    pub(super) fn with_catalog_auth_and_runtime(
        http_client: DynJsonHttpClient,
        catalog_model_id: impl Into<String>,
        provider_model_id: impl Into<String>,
        thinking_level: ThinkingLevel,
        auth: DynJsonHttpAuth,
        runtime: Arc<dyn DeferredRuntime>,
    ) -> Self {
        let provider_model_id = provider_model_id.into();
        let models = known_models();
        let thinking_level = resolve_catalog_thinking_level(
            &models,
            ProviderKind::Xai,
            &provider_model_id,
            thinking_level,
        )
        .unwrap_or(thinking_level);
        let deferred_completion = Arc::new(XaiDeferredCompletion::new(
            http_client.clone(),
            auth.clone(),
            provider_model_id.clone(),
            runtime,
        ));
        Self {
            inner: CompatModel::new(
                &DIALECT,
                http_client,
                catalog_model_id.into(),
                provider_model_id,
                thinking_level,
                auth,
            )
            .with_deferred(deferred_completion),
        }
    }
}

#[async_trait]
impl Model for XaiModel {
    async fn complete(&self, request: &ModelRequest) -> ModelResult<ModelResponse> {
        self.inner.complete(request).await
    }
    async fn complete_with_events(
        &self,
        request: &ModelRequest,
        sink: &dyn ModelCompletionEventSink,
    ) -> ModelResult<ModelResponse> {
        self.inner.complete_with_events(request, sink).await
    }
}
