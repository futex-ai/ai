//! OpenAI audio transcription client.

use std::time::Duration;

use ai_interface::{
    AudioTranscriber, AudioTranscriptionRequest, AudioTranscriptionResponse, TranscriptionError,
    TranscriptionResult,
};
use async_trait::async_trait;
use reqwest::{Client, StatusCode, multipart};
use serde::Deserialize;

const OPENAI_AUDIO_TRANSCRIPTIONS_URL: &str = "https://api.openai.com/v1/audio/transcriptions";
const DEFAULT_TRANSCRIPTION_TIMEOUT: Duration = Duration::from_secs(60);
const PROVIDER: &str = "openai";

/// OpenAI-backed `ai_interface::AudioTranscriber` implementation.
#[derive(Clone)]
pub struct OpenAiAudioTranscriber {
    client: Client,
    model_id: String,
    api_key: String,
    endpoint: String,
    timeout: Duration,
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
        }
    }
}

#[async_trait]
impl AudioTranscriber for OpenAiAudioTranscriber {
    async fn transcribe(
        &self,
        request: &AudioTranscriptionRequest,
    ) -> TranscriptionResult<AudioTranscriptionResponse> {
        let file = match multipart::Part::bytes(request.audio.clone())
            .file_name(request.filename.clone())
            .mime_str(&request.content_type)
        {
            Ok(file) => file,
            Err(source) => return Err(TranscriptionError::internal(source)),
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
            Err(source) => {
                return Err(TranscriptionError::transient_provider(
                    PROVIDER,
                    &self.model_id,
                    source.to_string(),
                ));
            }
        };
        let status = response.status();
        if status.is_client_error() || status.is_server_error() {
            let body = match response.text().await {
                Ok(body) => body,
                Err(source) => return Err(TranscriptionError::internal(source)),
            };
            return Err(classify_status(status, &self.model_id, body));
        }
        let response = match response.json::<OpenAiTranscriptionResponse>().await {
            Ok(response) => response,
            Err(source) => return Err(TranscriptionError::internal(source)),
        };
        let text = response.text.trim().to_owned();
        if text.is_empty() {
            return Err(TranscriptionError::EmptyTranscript);
        }
        Ok(AudioTranscriptionResponse { text })
    }
}

#[derive(Deserialize)]
struct OpenAiTranscriptionResponse {
    text: String,
}

fn classify_status(status: StatusCode, model_id: &str, body: String) -> TranscriptionError {
    if status == StatusCode::TOO_MANY_REQUESTS {
        return TranscriptionError::rate_limited(PROVIDER, model_id, body);
    }
    if status.is_server_error() {
        return TranscriptionError::transient_provider(PROVIDER, model_id, body);
    }
    TranscriptionError::provider(PROVIDER, model_id, body)
}

#[cfg(test)]
#[path = "_tests_/openai_transcription_tests.rs"]
mod openai_transcription_tests;
