//! OpenAI image generation transport client.

use std::time::Duration;

use ai_interface::{
    ImageGenerationError, ImageGenerationRequest, ImageGenerationResponse, ImageGenerationResult,
    ImageGenerator,
};
use async_trait::async_trait;
use reqwest::{Client, Response, multipart};
use serde_json::Value;

use super::{
    error::{classify_status, request_error},
    request::{
        OpenAiEditRequest, OpenAiGenerationRequest, OpenAiImageApiRequest, build_request,
        media_type_extension,
    },
    response::parse_response,
};

const OPENAI_IMAGE_GENERATIONS_URL: &str = "https://api.openai.com/v1/images/generations";
const OPENAI_IMAGE_EDITS_URL: &str = "https://api.openai.com/v1/images/edits";
const DEFAULT_IMAGE_TIMEOUT: Duration = Duration::from_secs(120);

/// OpenAI-backed `ai_interface::ImageGenerator` implementation.
#[derive(Clone)]
pub struct OpenAiImageGenerator {
    client: Client,
    model_id: String,
    api_key: String,
    pub(super) generation_endpoint: String,
    pub(super) edit_endpoint: String,
    pub(super) timeout: Duration,
}

impl OpenAiImageGenerator {
    /// Builds an OpenAI image generator from a model id and explicit API key.
    pub fn new(model_id: impl Into<String>, api_key: impl Into<String>) -> Self {
        Self {
            client: Client::new(),
            model_id: model_id.into(),
            api_key: api_key.into(),
            generation_endpoint: OPENAI_IMAGE_GENERATIONS_URL.to_owned(),
            edit_endpoint: OPENAI_IMAGE_EDITS_URL.to_owned(),
            timeout: DEFAULT_IMAGE_TIMEOUT,
        }
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
        body: OpenAiGenerationRequest,
    ) -> ImageGenerationResult<Response> {
        match self
            .client
            .post(&self.generation_endpoint)
            .bearer_auth(&self.api_key)
            .json(&body)
            .timeout(self.timeout)
            .send()
            .await
        {
            Ok(response) => Ok(response),
            Err(source) => Err(request_error(&self.model_id, source.to_string())),
        }
    }

    async fn send_edit(&self, body: OpenAiEditRequest) -> ImageGenerationResult<Response> {
        let mut form = multipart::Form::new()
            .text("model", body.model)
            .text("prompt", body.prompt)
            .text("size", body.size)
            .text("quality", body.quality)
            .text("n", "1");
        for (index, image) in body.images.into_iter().enumerate() {
            let file_name = format!("image-{index}.{}", media_type_extension(&image.mime_type));
            let part = match multipart::Part::bytes(image.data)
                .file_name(file_name)
                .mime_str(&image.mime_type)
            {
                Ok(part) => part,
                Err(source) => return Err(ImageGenerationError::internal(source)),
            };
            form = form.part("image[]", part);
        }
        match self
            .client
            .post(&self.edit_endpoint)
            .bearer_auth(&self.api_key)
            .multipart(form)
            .timeout(self.timeout)
            .send()
            .await
        {
            Ok(response) => Ok(response),
            Err(source) => Err(request_error(&self.model_id, source.to_string())),
        }
    }

    async fn parse_http_response(
        &self,
        response: Response,
    ) -> ImageGenerationResult<ImageGenerationResponse> {
        let status = response.status();
        let body = match response.text().await {
            Ok(body) => body,
            Err(source) => return Err(request_error(&self.model_id, source.to_string())),
        };
        if status.is_client_error() || status.is_server_error() {
            return Err(classify_status(status, &self.model_id, &body));
        }
        let body = match serde_json::from_str::<Value>(&body) {
            Ok(body) => body,
            Err(source) => return Err(ImageGenerationError::internal(source)),
        };
        parse_response(&self.model_id, body)
    }
}

#[async_trait]
impl ImageGenerator for OpenAiImageGenerator {
    async fn generate(
        &self,
        request: &ImageGenerationRequest,
    ) -> ImageGenerationResult<ImageGenerationResponse> {
        let response = match build_request(&self.model_id, request)? {
            OpenAiImageApiRequest::Generation(body) => self.send_generation(body).await?,
            OpenAiImageApiRequest::Edit(body) => self.send_edit(body).await?,
        };
        self.parse_http_response(response).await
    }
}
