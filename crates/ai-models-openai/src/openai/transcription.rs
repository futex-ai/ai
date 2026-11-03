//! OpenAI transcription with injected transport and preserved response parsing.

use std::{sync::Arc, time::Duration};

use ai_interface::{
    AudioTranscriber, AudioTranscriptionRequest, AudioTranscriptionResponse, ErrorKind, ModelSpec,
    TranscriptionError, TranscriptionResult,
};
use ai_models_core::{
    ApiKeyScheme, Clock, ErrorContext, ProviderConfig, SuccessfulResponse, SystemClock,
    json_http_error_core,
};
use async_trait::async_trait;
use json_http::{
    DynJsonHttpClient, JsonHttpHeaders, JsonHttpMultipartField, ReqwestJsonHttpClient,
};
use reqwest::{StatusCode, multipart};
use serde::Deserialize;
use serde_json::Value;

use crate::{CUSTOM_DEFAULTS, PROVIDER_ID};

use super::error_context;

const OPENAI_BASE_URL: &str = "https://api.openai.com";
const DEFAULT_TRANSCRIPTION_TIMEOUT: Duration = Duration::from_secs(60);

/// OpenAI-backed `ai_interface::AudioTranscriber` implementation.
#[derive(Clone)]
pub struct OpenAiAudioTranscriber {
    http_client: DynJsonHttpClient,
    config: ProviderConfig,
    model_id: String,
    endpoint: String,
    timeout: Duration,
    clock: Arc<dyn Clock>,
}

impl OpenAiAudioTranscriber {
    /// Builds an OpenAI transcriber from shared provider settings.
    pub fn from_config(
        http_client: DynJsonHttpClient,
        spec: ModelSpec,
        config: ProviderConfig,
    ) -> Self {
        Self {
            http_client,
            model_id: spec.provider_model_id.into_owned(),
            endpoint: config.endpoint(OPENAI_BASE_URL, "/v1/audio/transcriptions"),
            timeout: config
                .timeouts
                .total_timeout
                .unwrap_or(DEFAULT_TRANSCRIPTION_TIMEOUT),
            config,
            clock: Arc::new(SystemClock),
        }
    }

    /// Builds an OpenAI audio transcriber from an explicit API key.
    pub fn new(model_id: impl Into<String>, api_key: impl Into<String>) -> Self {
        Self::from_config(
            Arc::new(ReqwestJsonHttpClient::new()),
            ModelSpec::custom(PROVIDER_ID, model_id.into(), &CUSTOM_DEFAULTS),
            ProviderConfig::new(PROVIDER_ID, api_key),
        )
    }

    /// Uses an injected wall clock for response retry dates.
    pub fn with_clock(mut self, clock: Arc<dyn Clock>) -> Self {
        self.clock = clock;
        self
    }

    fn context(&self) -> ErrorContext {
        let context = error_context::new(&self.model_id, self.clock.clone());
        self.config.capture_secrets(&context);
        context
    }

    async fn read_response(
        &self,
        context: &ErrorContext,
        response: reqwest::Response,
    ) -> TranscriptionResult<AudioTranscriptionResponse> {
        let headers = JsonHttpHeaders::from(response.headers().clone());
        let context = context.for_response(&headers);
        let status = response.status();
        let response = match SuccessfulResponse::check(status.as_u16(), response) {
            Ok(response) => response.into_inner(),
            Err(response) => {
                let body = match response.text().await {
                    Ok(body) => body,
                    Err(source) => return Err(request_failure(&context, source)),
                };
                return Err(classify_status(&context, status, &headers, body));
            }
        };
        let response = match response.json::<OpenAiTranscriptionResponse>().await {
            Ok(response) => response,
            Err(source) => return Err(request_failure(&context, source)),
        };
        let text = response.text.trim().to_owned();
        if text.is_empty() {
            return Err(TranscriptionError::EmptyTranscript {
                core: Box::new(context.core(ErrorKind::InvalidOutput, "transcript text was empty")),
            });
        }
        Ok(AudioTranscriptionResponse { text })
    }
}

#[async_trait]
impl AudioTranscriber for OpenAiAudioTranscriber {
    async fn transcribe(
        &self,
        request: &AudioTranscriptionRequest,
    ) -> TranscriptionResult<AudioTranscriptionResponse> {
        let context = self.context();
        validate_media_type(&context, &request.content_type)?;
        let response = match self
            .config
            .apply_headers(
                self.http_client
                    .post(&self.endpoint)
                    .auth(context.capture_auth(self.config.auth(ApiKeyScheme::Bearer)))
                    .timeout(self.timeout),
                &self.endpoint,
            )
            .multipart(vec![
                JsonHttpMultipartField::bytes("model", self.model_id.as_bytes().to_vec()),
                JsonHttpMultipartField::bytes("response_format", b"json".to_vec()),
                JsonHttpMultipartField::bytes("file", request.audio.clone())
                    .filename(&request.filename)
                    .content_type(&request.content_type),
            ])
            .send_bytes()
            .await
        {
            Ok(response) => response,
            Err(source) => return Err(http_failure(&context, source)),
        };
        let mut buffered = match http::Response::builder()
            .status(response.status)
            .body(response.body)
        {
            Ok(buffered) => buffered,
            Err(source) => {
                return Err(TranscriptionError::with_source(
                    context.core(ErrorKind::Internal, "internal error"),
                    context.source(source),
                ));
            }
        };
        *buffered.headers_mut() = response.headers.as_raw().clone();
        self.read_response(&context, reqwest::Response::from(buffered))
            .await
    }
}

#[derive(Deserialize)]
struct OpenAiTranscriptionResponse {
    text: String,
}

fn validate_media_type(context: &ErrorContext, content_type: &str) -> TranscriptionResult<()> {
    match multipart::Part::bytes(Vec::new()).mime_str(content_type) {
        Ok(_) => Ok(()),
        Err(source) => Err(TranscriptionError::with_source(
            context.core(
                ErrorKind::UnsupportedInput,
                format!("unsupported media type `{content_type}`"),
            ),
            context.source(source),
        )),
    }
}

fn http_failure(context: &ErrorContext, source: json_http::Error) -> TranscriptionError {
    let context = match source.response_headers() {
        Some(headers) => context.for_response(headers),
        None => context.clone(),
    };
    match source.into_cause() {
        json_http::Error::ReqwestTransport { source } => request_failure(&context, source),
        source => {
            let mut core = context.request_error(&source);
            context.apply(&mut core);
            TranscriptionError::with_source(core, context.source(source))
        }
    }
}

fn classify_status(
    context: &ErrorContext,
    status: StatusCode,
    headers: &JsonHttpHeaders,
    body: String,
) -> TranscriptionError {
    let value = serde_json::from_str(&body).unwrap_or_else(|_| Value::String(body.clone()));
    let mut core = json_http_error_core(context, status.as_u16(), headers, &value);
    core.message = context.redact(&body);
    if status.as_u16() >= 400 {
        return TranscriptionError::new(core);
    }
    TranscriptionError::with_source(
        core,
        context.source(json_http::Error::HttpStatus {
            status: status.as_u16(),
            headers: Box::new(headers.clone()),
            body: value,
        }),
    )
}

fn request_failure(context: &ErrorContext, source: reqwest::Error) -> TranscriptionError {
    if let Some(url) = source.url() {
        context.add_secret(url.as_str());
    }
    let kind = if source.is_decode() {
        ErrorKind::InvalidOutput
    } else if source.is_builder() {
        ErrorKind::Internal
    } else {
        ErrorKind::Transient
    };
    let message = if kind == ErrorKind::Internal {
        "internal error".to_owned()
    } else {
        source.to_string()
    };
    TranscriptionError::with_source(context.core(kind, message), context.source(source))
}

#[cfg(test)]
#[path = "_tests_/openai_transcription_tests.rs"]
mod openai_transcription_tests;

#[cfg(test)]
#[path = "_tests_/transcription_context_tests.rs"]
mod transcription_context_tests;
