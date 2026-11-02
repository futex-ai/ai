//! OpenAI transcription retains its direct reqwest transport and response facts.

use std::{sync::Arc, time::Duration};

use ai_interface::{
    AudioTranscriber, AudioTranscriptionRequest, AudioTranscriptionResponse, ErrorKind,
    TranscriptionError, TranscriptionResult,
};
use ai_models_core::{Clock, ErrorContext, SystemClock, json_http_error_core};
use async_trait::async_trait;
use json_http::JsonHttpHeaders;
use reqwest::{Client, StatusCode, multipart};
use serde::Deserialize;
use serde_json::Value;

use super::error_context;

const OPENAI_AUDIO_TRANSCRIPTIONS_URL: &str = "https://api.openai.com/v1/audio/transcriptions";
const DEFAULT_TRANSCRIPTION_TIMEOUT: Duration = Duration::from_secs(60);

/// OpenAI-backed `ai_interface::AudioTranscriber` implementation.
#[derive(Clone)]
pub struct OpenAiAudioTranscriber {
    client: Client,
    model_id: String,
    api_key: String,
    endpoint: String,
    timeout: Duration,
    clock: Arc<dyn Clock>,
}

impl OpenAiAudioTranscriber {
    /// Builds an OpenAI audio transcriber from an explicit API key.
    pub fn new(model_id: impl Into<String>, api_key: impl Into<String>) -> Self {
        Self {
            client: Client::new(),
            model_id: model_id.into(),
            api_key: api_key.into(),
            endpoint: OPENAI_AUDIO_TRANSCRIPTIONS_URL.to_owned(),
            timeout: DEFAULT_TRANSCRIPTION_TIMEOUT,
            clock: Arc::new(SystemClock),
        }
    }

    /// Uses an injected wall clock for response retry dates.
    pub fn with_clock(mut self, clock: Arc<dyn Clock>) -> Self {
        self.clock = clock;
        self
    }

    fn context(&self) -> ErrorContext {
        let context = error_context::new(&self.model_id, self.clock.clone());
        context.add_secret(&self.api_key);
        context.add_secret(format!("Bearer {}", self.api_key));
        if let Some(token) = self.api_key.split_whitespace().last() {
            context.add_secret(token);
        }
        context
    }

    async fn read_response(
        &self,
        context: &ErrorContext,
        response: reqwest::Response,
    ) -> TranscriptionResult<AudioTranscriptionResponse> {
        let headers = response.headers().clone();
        let context = context.for_response(&headers);
        let status = response.status();
        if status.is_client_error() || status.is_server_error() {
            let body = match response.text().await {
                Ok(body) => body,
                Err(source) => return Err(request_failure(&context, source)),
            };
            return Err(classify_status(&context, status, &headers, body));
        }
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
        let file = match multipart::Part::bytes(request.audio.clone())
            .file_name(request.filename.clone())
            .mime_str(&request.content_type)
        {
            Ok(file) => file,
            Err(source) => {
                return Err(TranscriptionError::with_source(
                    context.core(
                        ErrorKind::UnsupportedInput,
                        format!("unsupported media type `{}`", request.content_type),
                    ),
                    source,
                ));
            }
        };
        let form = multipart::Form::new()
            .text("model", self.model_id.clone())
            .text("response_format", "json")
            .part("file", file);
        let response = match self
            .client
            .post(&self.endpoint)
            .bearer_auth(&self.api_key)
            .multipart(form)
            .timeout(self.timeout)
            .send()
            .await
        {
            Ok(response) => response,
            Err(source) => return Err(request_failure(&context, source)),
        };
        self.read_response(&context, response).await
    }
}

#[derive(Deserialize)]
struct OpenAiTranscriptionResponse {
    text: String,
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
    TranscriptionError::new(core)
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
    TranscriptionError::with_source(context.core(kind, message), source)
}

#[cfg(test)]
#[path = "_tests_/openai_transcription_tests.rs"]
mod openai_transcription_tests;

#[cfg(test)]
#[path = "_tests_/transcription_context_tests.rs"]
mod transcription_context_tests;
