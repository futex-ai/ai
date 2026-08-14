//! MiniMax chat-completions model client.

mod request;
mod request_types;
mod response;

use std::sync::Arc;

use ai_interface::{
    Model, ModelControl, ModelError, ModelRequest, ModelResponse, ModelToolChoice, ProviderKind,
};
use ai_models_core::{ThinkingLevel, classify_json_http_error, resolve_catalog_thinking_level};
use async_trait::async_trait;
use json_http::{DynJsonHttpAuth, DynJsonHttpClient, StaticHeaderAuth};

use crate::catalog::known_models;

const CHAT_COMPLETIONS_URL: &str = "https://api.minimax.io/v1/chat/completions";
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
        let models = known_models();
        let thinking_level = resolve_catalog_thinking_level(
            &models,
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

#[async_trait]
impl Model for MiniMaxModel {
    async fn complete(
        &self,
        request: &ModelRequest,
    ) -> std::result::Result<ModelResponse, ModelError> {
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
        let response_schema = request.response_schema.as_ref();
        let builder = self
            .http_client
            .post(CHAT_COMPLETIONS_URL)
            .auth(self.auth.clone());
        let builder = match request.controls.execution.total_timeout {
            Some(timeout) => builder.timeout(timeout),
            None => builder,
        };
        let request = builder
            .json(request::build_request(
                &self.provider_model_id,
                self.thinking_level,
                request,
            ))
            .map_err(ModelError::internal)?;
        let response = request
            .send_value()
            .await
            .map_err(|source| request_error(source, &self.provider_model_id))?;
        if response.status >= 400 {
            return Err(classify_json_http_error(
                PROVIDER,
                &self.provider_model_id,
                response.status,
                &response.body,
            ));
        }
        response::parse_response(
            &self.catalog_model_id,
            &self.provider_model_id,
            self.thinking_level,
            response.body,
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

#[cfg(test)]
#[path = "_tests_/error_tests.rs"]
mod error_tests;
#[cfg(test)]
#[path = "_tests_/finish_tests.rs"]
mod finish_tests;
#[cfg(test)]
#[path = "_tests_/multimodal_tests.rs"]
mod multimodal_tests;
#[cfg(test)]
#[path = "_tests_/provider_error_tests.rs"]
mod provider_error_tests;
#[cfg(test)]
#[path = "_tests_/replay_tests.rs"]
mod replay_tests;
#[cfg(test)]
#[path = "_tests_/response_shape_tests.rs"]
mod response_shape_tests;
#[cfg(test)]
#[path = "_tests_/structured_output_tests.rs"]
mod structured_output_tests;
#[cfg(test)]
#[path = "_tests_/support.rs"]
mod support;
#[cfg(test)]
#[path = "_tests_/text_tests.rs"]
mod text_tests;
#[cfg(test)]
#[path = "_tests_/thinking_tests.rs"]
mod thinking_tests;
#[cfg(test)]
#[path = "_tests_/tool_tests.rs"]
mod tool_tests;
#[cfg(test)]
#[path = "_tests_/usage_tests.rs"]
mod usage_tests;

#[cfg(test)]
#[path = "_tests_/controls_tests.rs"]
mod controls_tests;
