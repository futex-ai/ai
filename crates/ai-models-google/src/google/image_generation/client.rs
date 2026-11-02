//! Google image generation transport client.

use std::{collections::BTreeMap, sync::Arc, time::Duration};

use ai_interface::{
    ImageGenerationRequest, ImageGenerationResponse, ImageGenerationResult, ImageGenerator,
};
use ai_models_core::{Clock, SystemClock};
use async_trait::async_trait;
use json_http::{DynJsonHttpAuth, DynJsonHttpClient, StaticHeaderAuth};

use crate::google::error_context;

use super::{
    error::{classify_request_error, classify_status},
    request::build_request,
    response::parse_response,
};

const GOOGLE_GENERATE_CONTENT_URL_PREFIX: &str =
    "https://generativelanguage.googleapis.com/v1/models";
const DEFAULT_IMAGE_TIMEOUT: Duration = Duration::from_secs(120);

/// Google-backed `ai_interface::ImageGenerator` implementation.
#[derive(Clone)]
pub struct GoogleImageGenerator {
    http_client: DynJsonHttpClient,
    model_id: String,
    auth: DynJsonHttpAuth,
    endpoint_override: Option<String>,
    pub(super) timeout: Duration,
    clock: Arc<dyn Clock>,
    initial_secrets: Vec<String>,
}

impl GoogleImageGenerator {
    /// Builds a Google image generator from an injected client and explicit API key.
    pub fn new(
        http_client: DynJsonHttpClient,
        model_id: impl Into<String>,
        api_key: impl Into<String>,
    ) -> Self {
        let api_key = api_key.into();
        let mut model = Self::with_auth(
            http_client,
            model_id,
            Arc::new(StaticHeaderAuth::new(BTreeMap::from([(
                "x-goog-api-key".to_owned(),
                api_key.clone(),
            )]))),
        );
        model.initial_secrets.push(api_key);
        model
    }

    /// Builds a Google image generator from an injected client and auth hook.
    pub fn with_auth(
        http_client: DynJsonHttpClient,
        model_id: impl Into<String>,
        auth: DynJsonHttpAuth,
    ) -> Self {
        Self {
            http_client,
            model_id: model_id.into(),
            auth,
            endpoint_override: None,
            timeout: DEFAULT_IMAGE_TIMEOUT,
            clock: Arc::new(SystemClock),
            initial_secrets: Vec::new(),
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
            format!(
                "{GOOGLE_GENERATE_CONTENT_URL_PREFIX}/{}:generateContent",
                self.model_id
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
        for secret in &self.initial_secrets {
            context.add_secret(secret);
            if let Some(token) = secret.split_whitespace().last() {
                context.add_secret(token);
            }
        }
        let body = build_request(&context, request)?;
        let request = match self
            .http_client
            .post(&self.endpoint())
            .auth(context.capture_auth(self.auth.clone()))
            .timeout(self.timeout)
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
