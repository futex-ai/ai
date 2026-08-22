//! MiniMax model construction and streaming request dispatch.

use std::{sync::Arc, time::Duration};

use ai_interface::{
    Model, ModelCompletionEventSink, ModelControl, ModelError, ModelRequest, ModelResponse,
    ModelToolChoice, NoopModelCompletionEventSink, ProviderKind,
};
use ai_models_core::{
    ThinkingLevel, classify_json_http_stream_error, resolve_catalog_thinking_level,
};
use async_trait::async_trait;
use json_http::{DynJsonHttpAuth, DynJsonHttpClient, StaticHeaderAuth};

use crate::catalog::known_models;

use super::{request, stream};

const CHAT_COMPLETIONS_URL: &str = "https://api.minimax.io/v1/chat/completions";
const COMPLETION_STREAM_TIMEOUT: Duration = Duration::from_secs(3_600);
const COMPLETION_STREAM_IDLE_TIMEOUT: Duration = Duration::from_secs(120);
const PROVIDER: &str = "minimax";

#[derive(Clone)]
/// MiniMax-backed `ai_interface::Model` implementation.
pub struct MiniMaxModel {
    http_client: DynJsonHttpClient,
    catalog_model_id: String,
    provider_model_id: String,
    thinking_level: ThinkingLevel,
    auth: DynJsonHttpAuth,
}

impl MiniMaxModel {
    /// Builds a MiniMax model from an explicit API key.
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

    /// Builds a MiniMax model with adaptive thinking and an explicit auth hook.
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
            ThinkingLevel::Medium,
            auth,
        )
    }

    /// Builds a MiniMax model from catalog metadata and an explicit auth hook.
    pub fn with_catalog_auth(
        http_client: DynJsonHttpClient,
        catalog_model_id: impl Into<String>,
        provider_model_id: impl Into<String>,
        thinking_level: ThinkingLevel,
        auth: DynJsonHttpAuth,
    ) -> Self {
        let provider_model_id = provider_model_id.into();
        let thinking_level = resolve_catalog_thinking_level(
            &known_models(),
            ProviderKind::MiniMax,
            &provider_model_id,
            thinking_level,
        )
        .unwrap_or(thinking_level);
        Self {
            http_client,
            catalog_model_id: catalog_model_id.into(),
            provider_model_id,
            thinking_level,
            auth,
        }
    }
}

impl MiniMaxModel {
    async fn complete_with_sink(
        &self,
        model_request: &ModelRequest,
        event_sink: &dyn ModelCompletionEventSink,
    ) -> std::result::Result<ModelResponse, ModelError> {
        self.validate_controls(model_request)?;
        let request_body =
            request::build_request(&self.provider_model_id, self.thinking_level, model_request);
        let timeout = model_request
            .controls
            .execution
            .total_timeout
            .unwrap_or(COMPLETION_STREAM_TIMEOUT);
        let builder = self
            .http_client
            .post(CHAT_COMPLETIONS_URL)
            .auth(self.auth.clone())
            .timeout(timeout)
            .idle_timeout(COMPLETION_STREAM_IDLE_TIMEOUT);
        let request = match builder.json(request_body) {
            Ok(request) => request,
            Err(source) => return Err(ModelError::internal(source)),
        };
        let event_stream = match request.send_sse().await {
            Ok(event_stream) => event_stream,
            Err(source) => {
                return Err(classify_json_http_stream_error(
                    PROVIDER,
                    &self.provider_model_id,
                    0,
                    source,
                ));
            }
        };
        stream::complete(
            event_stream,
            &self.catalog_model_id,
            &self.provider_model_id,
            self.thinking_level,
            model_request.response_schema.as_ref(),
            event_sink,
        )
        .await
    }
}

#[async_trait]
impl Model for MiniMaxModel {
    async fn complete(
        &self,
        model_request: &ModelRequest,
    ) -> std::result::Result<ModelResponse, ModelError> {
        self.complete_with_sink(model_request, &NoopModelCompletionEventSink)
            .await
    }

    async fn complete_with_events(
        &self,
        model_request: &ModelRequest,
        event_sink: &dyn ModelCompletionEventSink,
    ) -> std::result::Result<ModelResponse, ModelError> {
        if model_request.response_schema.is_some() {
            return self
                .complete_with_sink(model_request, &NoopModelCompletionEventSink)
                .await;
        }
        self.complete_with_sink(model_request, event_sink).await
    }
}

impl MiniMaxModel {
    fn validate_controls(&self, request: &ModelRequest) -> Result<(), ModelError> {
        if let Err(control) = request.controls.execution.resolve_deferred(false) {
            return Err(ModelError::unsupported_control(
                PROVIDER,
                &self.provider_model_id,
                control,
            ));
        }
        if !request.controls.generation.stop_sequences.is_empty() {
            return Err(ModelError::unsupported_control(
                PROVIDER,
                &self.provider_model_id,
                ModelControl::StopSequences,
            ));
        }
        if matches!(
            request.controls.generation.tool_choice,
            Some(ModelToolChoice::Function(_))
        ) || request.controls.generation.tool_choice == Some(ModelToolChoice::Required)
            && self.provider_model_id != crate::MINIMAX_M3
        {
            return Err(ModelError::unsupported_control(
                PROVIDER,
                &self.provider_model_id,
                ModelControl::ToolChoice,
            ));
        }
        Ok(())
    }
}
