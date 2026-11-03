//! Google image generation transport client.

use std::{sync::Arc, time::Duration};

use ai_interface::{
    ImageGenerationRequest, ImageGenerationResponse, ImageGenerationResult, ImageGenerator,
    ModelSpec,
};
use ai_models_core::{ApiKeyScheme, Clock, ProviderConfig, SystemClock};
use async_trait::async_trait;
use json_http::{DynJsonHttpAuth, DynJsonHttpClient};

use crate::{
    PROVIDER_ID,
    google::{GOOGLE_API_BASE_URL, error_context},
};

use super::{
    error::{classify_request_error, classify_status},
    request::build_request,
    response::parse_response,
};

const DEFAULT_IMAGE_TIMEOUT: Duration = Duration::from_secs(120);

/// Google-backed `ai_interface::ImageGenerator` implementation.
#[derive(Clone)]
pub struct GoogleImageGenerator {
    http_client: DynJsonHttpClient,
    model_id: String,
    config: ProviderConfig,
    endpoint_override: Option<String>,
    pub(super) timeout: Duration,
    clock: Arc<dyn Clock>,
}

impl GoogleImageGenerator {
    /// Builds a Google image generator from shared provider settings.
    pub fn from_config(
        http_client: DynJsonHttpClient,
        spec: ModelSpec,
        config: ProviderConfig,
    ) -> Self {
        Self::with_config(http_client, spec.provider_model_id.into_owned(), config)
    }

    /// Builds a Google image generator from an injected client and explicit API key.
    pub fn new(
        http_client: DynJsonHttpClient,
        model_id: impl Into<String>,
        api_key: impl Into<String>,
    ) -> Self {
        Self::with_config(
            http_client,
            model_id.into(),
            ProviderConfig::new(PROVIDER_ID, api_key),
        )
    }

    /// Builds a Google image generator from an injected client and auth hook.
    pub fn with_auth(
        http_client: DynJsonHttpClient,
        model_id: impl Into<String>,
        auth: DynJsonHttpAuth,
    ) -> Self {
        Self::with_config(
            http_client,
            model_id.into(),
            ProviderConfig::with_auth(PROVIDER_ID, auth),
        )
    }

    fn with_config(
        http_client: DynJsonHttpClient,
        model_id: String,
        config: ProviderConfig,
    ) -> Self {
        Self {
            http_client,
            model_id,
            endpoint_override: None,
            timeout: config
                .timeouts
                .total_timeout
                .unwrap_or(DEFAULT_IMAGE_TIMEOUT),
            clock: Arc::new(SystemClock),
            config,
        }
    }

    /// Uses an injected wall clock for response retry dates.
    pub fn with_clock(mut self, clock: Arc<dyn Clock>) -> Self {
        self.clock = clock;
        self
    }

    /// Overrides the generated provider endpoint.
    pub fn with_endpoint(mut self, endpoint: impl Into<String>) -> Self {
        self.endpoint_override = Some(endpoint.into());
        self
    }

    /// Overrides the request timeout.
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    pub(super) fn endpoint(&self) -> String {
        self.endpoint_override.clone().unwrap_or_else(|| {
            self.config.endpoint(
                GOOGLE_API_BASE_URL,
                &format!("/v1/models/{}:generateContent", self.model_id),
            )
        })
    }
}

#[async_trait]
impl ImageGenerator for GoogleImageGenerator {
    async fn generate(
        &self,
        request: &ImageGenerationRequest,
    ) -> ImageGenerationResult<ImageGenerationResponse> {
        let context = error_context::new(&self.model_id, self.clock.clone());
        self.config.capture_secrets(&context);
        let body = build_request(&context, request)?;
        let request = match self
            .config
            .apply_headers(
                self.http_client
                    .post(&self.endpoint())
                    .auth(
                        context
                            .capture_auth(self.config.auth(ApiKeyScheme::Header("x-goog-api-key"))),
                    )
                    .timeout(self.timeout),
                &self.endpoint(),
            )
            .json(body)
        {
            Ok(request) => request,
            Err(source) => return Err(classify_request_error(&context, source)),
        };
        let response = match request.send_value().await {
            Ok(response) => response,
            Err(source) => return Err(classify_request_error(&context, source)),
        };
        if response.status >= 400 {
            return Err(classify_status(
                &context,
                response.status,
                &response.headers,
                &response.body,
            ));
        }
        let context = context.for_response(&response.headers);
        parse_response(&context, &response.headers, &self.model_id, response.body)
    }
}
