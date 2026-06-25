//! Built-in mock audio transcriber for development and tests.

use async_trait::async_trait;

use crate::{
    AudioTranscriber, AudioTranscriptionRequest, AudioTranscriptionResponse, TranscriptionError,
    TranscriptionResult,
};

/// Simple deterministic mock transcriber used by development and tests.
#[derive(Clone, Debug)]
pub struct MockAudioTranscriber {
    text: String,
}

impl MockAudioTranscriber {
    /// Builds a mock transcriber that returns the provided transcript.
    pub fn new(text: impl Into<String>) -> Self {
        Self { text: text.into() }
    }
}

impl Default for MockAudioTranscriber {
    fn default() -> Self {
        Self::new("local voice transcript")
    }
}

#[async_trait]
impl AudioTranscriber for MockAudioTranscriber {
    async fn transcribe(
        &self,
        _request: &AudioTranscriptionRequest,
    ) -> TranscriptionResult<AudioTranscriptionResponse> {
        if self.text.trim().is_empty() {
            return Err(TranscriptionError::EmptyTranscript);
        }
        Ok(AudioTranscriptionResponse {
            text: self.text.clone(),
        })
    }
}
