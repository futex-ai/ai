//! OpenAI image generation transport client.

use std::{sync::Arc, time::Duration};

use ai_interface::{
    ImageGenerationRequest, ImageGenerationResponse, ImageGenerationResult, ImageGenerator,
    ModelSpec,
};
use ai_models_core::{ApiKeyScheme, Clock, ErrorContext, ProviderConfig, SystemClock};
use async_trait::async_trait;
use json_http::{DynJsonHttpAuth, DynJsonHttpClient, JsonHttpMultipartField, JsonHttpResponse};

use crate::openai::error_context;
use crate::{CUSTOM_DEFAULTS, PROVIDER_ID};

use super::{
    error::{classify_request_error, classify_status},
    request::{
        OpenAiEditRequest, OpenAiGenerationRequest, OpenAiImageApiRequest, build_request,
        media_type_extension,
    },
    response::parse_response,
};

const OPENAI_BASE_URL: &str = "https://api.openai.com";
const DEFAULT_IMAGE_TIMEOUT: Duration = Duration::from_secs(120);

/// OpenAI-backed `ai_interface::ImageGenerator` implementation.
#[derive(Clone)]
pub struct OpenAiImageGenerator {
    http_client: DynJsonHttpClient,
    model_id: String,
    config: ProviderConfig,
    pub(super) generation_endpoint: String,
    pub(super) edit_endpoint: String,
    pub(super) timeout: Duration,
    clock: Arc<dyn Clock>,
}

impl OpenAiImageGenerator {
    /// Builds an OpenAI image generator from shared provider settings.
    pub fn from_config(
        http_client: DynJsonHttpClient,
        spec: ModelSpec,
        config: ProviderConfig,
    ) -> Self {
        Self {
            http_client,
            model_id: spec.provider_model_id.into_owned(),
            generation_endpoint: config.endpoint(OPENAI_BASE_URL, "/v1/images/generations"),
            edit_endpoint: config.endpoint(OPENAI_BASE_URL, "/v1/images/edits"),
            timeout: config
                .timeouts
                .total_timeout
                .unwrap_or(DEFAULT_IMAGE_TIMEOUT),
            config,
            clock: Arc::new(SystemClock),
        }
    }

    /// Builds an OpenAI image generator from an injected client, model id, and API key.
    pub fn new(
        http_client: DynJsonHttpClient,
        model_id: impl Into<String>,
        api_key: impl Into<String>,
    ) -> Self {
        Self::from_config(
            http_client,
            ModelSpec::custom(PROVIDER_ID, model_id.into(), &CUSTOM_DEFAULTS),
            ProviderConfig::new(PROVIDER_ID, api_key),
        )
    }

    /// Builds an OpenAI image generator from an injected client and auth hook.
    pub fn with_auth(
        http_client: DynJsonHttpClient,
        model_id: impl Into<String>,
        auth: DynJsonHttpAuth,
    ) -> Self {
        Self::from_config(
            http_client,
            ModelSpec::custom(PROVIDER_ID, model_id.into(), &CUSTOM_DEFAULTS),
            ProviderConfig::with_auth(PROVIDER_ID, auth),
        )
    }

    /// Uses an injected wall clock for response retry dates.
    pub fn with_clock(mut self, clock: Arc<dyn Clock>) -> Self {
        self.clock = clock;
        self
    }

    /// Overrides generation and edit endpoints.
    pub fn with_endpoints(
        mut self,
        generation_endpoint: impl Into<String>,
        edit_endpoint: impl Into<String>,
    ) -> Self {
        self.generation_endpoint = generation_endpoint.into();
        self.edit_endpoint = edit_endpoint.into();
        self
    }

    /// Overrides the request timeout.
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    async fn send_generation(
        &self,
        context: &ErrorContext,
        body: OpenAiGenerationRequest,
    ) -> ImageGenerationResult<JsonHttpResponse<serde_json::Value>> {
        let request = match self
            .config
            .apply_headers(
                self.http_client
                    .post(&self.generation_endpoint)
                    .auth(context.capture_auth(self.config.auth(ApiKeyScheme::Bearer)))
                    .timeout(self.timeout),
            )
            .json(body)
        {
            Ok(request) => request,
            Err(source) => return Err(classify_request_error(context, source)),
        };
        match request.send_value().await {
            Ok(response) => Ok(response),
            Err(source) => Err(classify_request_error(context, source)),
        }
    }

    async fn send_edit(
        &self,
        context: &ErrorContext,
        body: OpenAiEditRequest,
    ) -> ImageGenerationResult<JsonHttpResponse<serde_json::Value>> {
        match self
            .config
            .apply_headers(
                self.http_client
                    .post(&self.edit_endpoint)
                    .auth(context.capture_auth(self.config.auth(ApiKeyScheme::Bearer)))
                    .timeout(self.timeout),
            )
            .multipart(edit_fields(body))
            .send_value()
            .await
        {
            Ok(response) => Ok(response),
            Err(source) => Err(classify_request_error(context, source)),
        }
    }

    fn parse_http_response(
        &self,
        context: &ErrorContext,
        response: JsonHttpResponse<serde_json::Value>,
    ) -> ImageGenerationResult<ImageGenerationResponse> {
        if response.status >= 400 {
            return Err(classify_status(
                context,
                response.status,
                &response.headers,
                &response.body,
            ));
        }
        let context = context.for_response(&response.headers);
        parse_response(&context, &self.model_id, response.body)
    }
}

#[async_trait]
impl ImageGenerator for OpenAiImageGenerator {
    async fn generate(
        &self,
        request: &ImageGenerationRequest,
    ) -> ImageGenerationResult<ImageGenerationResponse> {
        let context = error_context::new(&self.model_id, self.clock.clone());
        self.config.capture_secrets(&context);
        let response = match build_request(&context, &self.model_id, request)? {
            OpenAiImageApiRequest::Generation(body) => self.send_generation(&context, body).await?,
            OpenAiImageApiRequest::Edit(body) => self.send_edit(&context, body).await?,
        };
        self.parse_http_response(&context, response)
    }
}

fn edit_fields(body: OpenAiEditRequest) -> Vec<JsonHttpMultipartField> {
    let mut fields = vec![
        text_field("model", body.model),
        text_field("prompt", body.prompt),
        text_field("size", body.size),
        text_field("quality", body.quality),
        text_field("n", "1"),
    ];
    fields.extend(body.images.into_iter().enumerate().map(|(index, image)| {
        let file_name = format!("image-{index}.{}", media_type_extension(&image.mime_type));
        JsonHttpMultipartField::bytes("image[]", image.data)
            .filename(file_name)
            .content_type(image.mime_type)
    }));
    fields
}

fn text_field(name: &str, value: impl Into<String>) -> JsonHttpMultipartField {
    JsonHttpMultipartField::bytes(name, value.into().into_bytes())
}
