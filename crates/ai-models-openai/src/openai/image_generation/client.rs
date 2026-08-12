//! OpenAI image generation transport client.

use std::{sync::Arc, time::Duration};

use ai_interface::{
    ImageGenerationRequest, ImageGenerationResponse, ImageGenerationResult, ImageGenerator,
};
use async_trait::async_trait;
use json_http::{
    DynJsonHttpAuth, DynJsonHttpClient, JsonHttpMultipartField, JsonHttpResponse, StaticHeaderAuth,
};

use super::{
    error::{classify_request_error, classify_status},
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
    http_client: DynJsonHttpClient,
    model_id: String,
    auth: DynJsonHttpAuth,
    pub(super) generation_endpoint: String,
    pub(super) edit_endpoint: String,
    pub(super) timeout: Duration,
}

impl OpenAiImageGenerator {
    /// Builds an OpenAI image generator from an injected client, model id, and API key.
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

    /// Builds an OpenAI image generator from an injected client and auth hook.
    pub fn with_auth(
        http_client: DynJsonHttpClient,
        model_id: impl Into<String>,
        auth: DynJsonHttpAuth,
    ) -> Self {
        Self {
            http_client,
            model_id: model_id.into(),
            auth,
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
    ) -> ImageGenerationResult<JsonHttpResponse<serde_json::Value>> {
        let request = match self
            .http_client
            .post(&self.generation_endpoint)
            .auth(self.auth.clone())
            .timeout(self.timeout)
            .json(body)
        {
            Ok(request) => request,
            Err(source) => return Err(classify_request_error(source, &self.model_id)),
        };
        match request.send_value().await {
            Ok(response) => Ok(response),
            Err(source) => Err(classify_request_error(source, &self.model_id)),
        }
    }

    async fn send_edit(
        &self,
        body: OpenAiEditRequest,
    ) -> ImageGenerationResult<JsonHttpResponse<serde_json::Value>> {
        match self
            .http_client
            .post(&self.edit_endpoint)
            .auth(self.auth.clone())
            .timeout(self.timeout)
            .multipart(edit_fields(body))
            .send_value()
            .await
        {
            Ok(response) => Ok(response),
            Err(source) => Err(classify_request_error(source, &self.model_id)),
        }
    }

    fn parse_http_response(
        &self,
        response: JsonHttpResponse<serde_json::Value>,
    ) -> ImageGenerationResult<ImageGenerationResponse> {
        if response.status >= 400 {
            return Err(classify_status(
                response.status,
                &self.model_id,
                &response.body,
            ));
        }
        parse_response(&self.model_id, response.body)
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
        self.parse_http_response(response)
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
