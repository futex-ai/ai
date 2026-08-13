//! XAI model construction and request dispatch.

use std::{sync::Arc, time::Duration};

use ai_interface::{Model, ModelError, ModelRequest, ModelResponse, ModelResult};
use ai_models_core::{ThinkingLevel, classify_json_http_error, synthetic_tool_call_scope};
use async_trait::async_trait;
use json_http::{DynJsonHttpAuth, DynJsonHttpClient, StaticHeaderAuth};

use super::{
    deferred::{
        DeferredRuntime, DynDeferredCompletion, TokioDeferredRuntime, XaiDeferredCompletion,
    },
    request, response,
};

const DEFAULT_DEFERRED_TIMEOUT: Duration = Duration::from_secs(60);
const XAI_CHAT_COMPLETIONS_URL: &str = "https://api.x.ai/v1/chat/completions";
const PROVIDER: &str = "xai";

#[derive(Clone)]
/// xAI-backed `ai_interface::Model` implementation.
pub struct XaiModel {
    http_client: DynJsonHttpClient,
    catalog_model_id: String,
    provider_model_id: String,
    thinking_level: ThinkingLevel,
    auth: DynJsonHttpAuth,
    deferred_completion: DynDeferredCompletion,
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
        let deferred_completion = Arc::new(XaiDeferredCompletion::new(
            http_client.clone(),
            auth.clone(),
            provider_model_id.clone(),
            runtime,
        ));
        Self {
            http_client,
            catalog_model_id: catalog_model_id.into(),
            provider_model_id,
            thinking_level,
            auth,
            deferred_completion,
        }
    }

    async fn immediate_completion(
        &self,
        request_body: super::request_types::ChatCompletionsRequest,
        total_timeout: Option<Duration>,
    ) -> ModelResult<serde_json::Value> {
        let builder = self
            .http_client
            .post(XAI_CHAT_COMPLETIONS_URL)
            .auth(self.auth.clone());
        let builder = match total_timeout {
            Some(timeout) => builder.timeout(timeout),
            None => builder,
        };
        let request = match builder.json(request_body) {
            Ok(request) => request,
            Err(source) => return Err(ModelError::internal(source)),
        };
        let response = match request.send_value().await {
            Ok(response) => response,
            Err(source) => return Err(request_error(source, &self.provider_model_id)),
        };
        if response.status >= 400 {
            return Err(classify_json_http_error(
                PROVIDER,
                &self.provider_model_id,
                response.status,
                &response.body,
            ));
        }
        Ok(response.body)
    }
}

#[async_trait]
impl Model for XaiModel {
    async fn complete(&self, request: &ModelRequest) -> ModelResult<ModelResponse> {
        let use_deferred = match request.controls.execution.resolve_deferred(true) {
            Ok(use_deferred) => use_deferred,
            Err(control) => {
                return Err(ModelError::unsupported_control(
                    PROVIDER,
                    &self.provider_model_id,
                    control,
                ));
            }
        };
        let response_schema = request.response_schema.as_ref();
        let synthetic_tool_call_scope = synthetic_tool_call_scope(request);
        let request_body =
            request::build_request(&self.provider_model_id, self.thinking_level, request)?;
        let body = if use_deferred {
            self.deferred_completion
                .complete(
                    request_body,
                    request
                        .controls
                        .execution
                        .total_timeout
                        .unwrap_or(DEFAULT_DEFERRED_TIMEOUT),
                )
                .await?
        } else {
            self.immediate_completion(request_body, request.controls.execution.total_timeout)
                .await?
        };
        response::parse_response(
            &self.catalog_model_id,
            &self.provider_model_id,
            self.thinking_level,
            &synthetic_tool_call_scope,
            body,
            response_schema,
        )
    }
}

fn request_error(source: json_http::Error, model_id: &str) -> ModelError {
    match source {
        json_http::Error::Transport { .. } | json_http::Error::Auth { .. } => {
            ModelError::transient_provider(PROVIDER, model_id, source.to_string())
        }
        json_http::Error::SerializeRequest { .. }
        | json_http::Error::DeserializeResponse { .. } => ModelError::internal(source),
    }
}
