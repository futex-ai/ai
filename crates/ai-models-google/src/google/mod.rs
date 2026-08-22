//! Google Gemini generate-content model client.

mod image_generation;
mod request;
mod response;
mod stream;
mod thinking;
mod tool_config;
mod video_generation;

pub use image_generation::GoogleImageGenerator;
pub use video_generation::GoogleVideoGenerator;

use std::{collections::BTreeMap, sync::Arc, time::Duration};

use ai_interface::{Model, ModelError, ModelRequest, ModelResponse, ProviderKind};
use ai_models_core::{
    ThinkingLevel, classify_json_http_stream_error, resolve_catalog_thinking_level,
    synthetic_tool_call_scope,
};
use async_trait::async_trait;
use json_http::{DynJsonHttpAuth, DynJsonHttpClient, StaticHeaderAuth};

use crate::catalog::known_models;

const GOOGLE_GENERATE_CONTENT_URL_PREFIX: &str =
    "https://generativelanguage.googleapis.com/v1beta/models";
const COMPLETION_TIMEOUT: Duration = Duration::from_secs(3_600);
const PROVIDER: &str = "google";
const STREAM_IDLE_TIMEOUT: Duration = Duration::from_secs(120);

#[derive(Clone)]
/// Google-backed `ai_interface::Model` implementation.
pub struct GoogleModel {
    http_client: DynJsonHttpClient,
    catalog_model_id: String,
    provider_model_id: String,
    thinking_level: ThinkingLevel,
    auth: DynJsonHttpAuth,
}

impl GoogleModel {
    /// Builds a Google model from an explicit API key.
    pub fn new(
        http_client: DynJsonHttpClient,
        model_id: impl Into<String>,
        api_key: impl Into<String>,
    ) -> Self {
        Self::with_auth(
            http_client,
            model_id,
            Arc::new(StaticHeaderAuth::new(BTreeMap::from([(
                "x-goog-api-key".to_owned(),
                api_key.into(),
            )]))),
        )
    }

    /// Builds a Google model from an explicit auth hook.
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

    /// Builds a Google model from catalog metadata and an explicit auth hook.
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
            ProviderKind::Google,
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

    fn endpoint(&self) -> String {
        format!(
            "{GOOGLE_GENERATE_CONTENT_URL_PREFIX}/{}:streamGenerateContent?alt=sse",
            self.provider_model_id
        )
    }
}

#[async_trait]
impl Model for GoogleModel {
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
        let response_schema = request.response_schema.as_ref();
        let synthetic_tool_call_scope = synthetic_tool_call_scope(request);
        let builder = self
            .http_client
            .post(&self.endpoint())
            .auth(self.auth.clone())
            .timeout(
                request
                    .controls
                    .execution
                    .total_timeout
                    .unwrap_or(COMPLETION_TIMEOUT),
            )
            .idle_timeout(STREAM_IDLE_TIMEOUT);
        let builder = match builder.json(request::build_request(
            &self.provider_model_id,
            request,
            self.thinking_level,
        )) {
            Ok(builder) => builder,
            Err(source) => return Err(ModelError::internal(source)),
        };
        let stream = match builder.send_sse().await {
            Ok(stream) => stream,
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
            stream,
            &self.catalog_model_id,
            &self.provider_model_id,
            self.thinking_level,
            &synthetic_tool_call_scope,
            response_schema,
        )
        .await
    }
}

#[cfg(test)]
#[path = "_tests_/google/mod.rs"]
mod google_tests;
