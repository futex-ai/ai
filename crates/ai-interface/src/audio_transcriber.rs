//! Audio transcription DTOs and transcriber trait.

use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Provider-agnostic audio transcription request.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct AudioTranscriptionRequest {
    /// Audio bytes to transcribe.
    pub audio: Vec<u8>,
    /// Filename supplied to the provider for media type inference.
    pub filename: String,
    /// MIME content type for the uploaded audio.
    pub content_type: String,
}

/// Provider-agnostic audio transcription response.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct AudioTranscriptionResponse {
    /// Transcribed plain text.
    pub text: String,
}

/// Errors returned by the audio transcription boundary.
#[derive(Debug, Error)]
pub enum TranscriptionError {
    /// The client submitted an empty audio payload.
    #[error("[ai_interface/audio_transcriber] audio must not be empty")]
    EmptyAudio,
    /// The client submitted audio in an unsupported media type.
    #[error("[ai_interface/audio_transcriber] unsupported media type `{content_type}`")]
    UnsupportedMediaType {
        /// Unsupported MIME media type.
        content_type: String,
    },
    /// The client submitted audio larger than the accepted limit.
    #[error(
        "[ai_interface/audio_transcriber] audio is too large: {actual_bytes} bytes exceeds {max_bytes} bytes"
    )]
    AudioTooLarge {
        /// Maximum accepted audio size in bytes.
        max_bytes: usize,
        /// Submitted audio size in bytes.
        actual_bytes: usize,
    },
    /// The upstream provider returned no usable transcript text.
    #[error("[ai_interface/audio_transcriber] transcript text was empty")]
    EmptyTranscript,
    /// The upstream provider rejected the request due to a rate limit.
    #[error(
        "[ai_interface/audio_transcriber] provider rate limit for `{provider}` model `{model_id}`: {message}"
    )]
    RateLimited {
        /// Provider that returned the rate-limit response.
        provider: String,
        /// Model identifier requested from the provider.
        model_id: String,
        /// Provider-supplied failure details.
        message: String,
    },
    /// The upstream provider returned a transient failure that may succeed later.
    #[error(
        "[ai_interface/audio_transcriber] transient provider failure for `{provider}` model `{model_id}`: {message}"
    )]
    TransientProvider {
        /// Provider that returned the transient failure.
        provider: String,
        /// Model identifier requested from the provider.
        model_id: String,
        /// Provider-supplied failure details.
        message: String,
    },
    /// The upstream provider returned a non-retryable failure.
    #[error(
        "[ai_interface/audio_transcriber] provider failure for `{provider}` model `{model_id}`: {message}"
    )]
    Provider {
        /// Provider that returned the failure.
        provider: String,
        /// Model identifier requested from the provider.
        model_id: String,
        /// Provider-supplied failure details.
        message: String,
    },
    /// Unhandled transcription-boundary failure.
    #[error("[ai_interface/audio_transcriber] internal error: {source}")]
    Internal {
        /// Underlying transcription failure.
        source: Box<dyn std::error::Error + Send + Sync>,
    },
}

impl TranscriptionError {
    /// Builds an unsupported media type error.
    pub fn unsupported_media_type(content_type: impl Into<String>) -> Self {
        Self::UnsupportedMediaType {
            content_type: content_type.into(),
        }
    }

    /// Builds an audio-too-large error.
    pub fn audio_too_large(max_bytes: usize, actual_bytes: usize) -> Self {
        Self::AudioTooLarge {
            max_bytes,
            actual_bytes,
        }
    }

    /// Builds a rate-limited provider error.
    pub fn rate_limited(
        provider: impl Into<String>,
        model_id: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self::RateLimited {
            provider: provider.into(),
            model_id: model_id.into(),
            message: message.into(),
        }
    }

    /// Builds a transient provider error.
    pub fn transient_provider(
        provider: impl Into<String>,
        model_id: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self::TransientProvider {
            provider: provider.into(),
            model_id: model_id.into(),
            message: message.into(),
        }
    }

    /// Builds a non-retryable provider error.
    pub fn provider(
        provider: impl Into<String>,
        model_id: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self::Provider {
            provider: provider.into(),
            model_id: model_id.into(),
            message: message.into(),
        }
    }

    /// Wraps an internal transcription-boundary error.
    pub fn internal(source: impl std::error::Error + Send + Sync + 'static) -> Self {
        Self::Internal {
            source: Box::new(source),
        }
    }
}

/// Result alias for transcription operations.
pub type TranscriptionResult<T> = std::result::Result<T, TranscriptionError>;

#[cfg_attr(
    any(test, doctest, feature = "test-support"),
    unimock::unimock(api = AudioTranscriberMock)
)]
#[async_trait]
/// Provider-agnostic audio transcription boundary.
pub trait AudioTranscriber: Send + Sync {
    /// Transcribes a completed audio recording to plain text.
    async fn transcribe(
        &self,
        request: &AudioTranscriptionRequest,
    ) -> TranscriptionResult<AudioTranscriptionResponse>;
}

/// Shared dynamic audio transcriber alias.
pub type DynAudioTranscriber = Arc<dyn AudioTranscriber>;
