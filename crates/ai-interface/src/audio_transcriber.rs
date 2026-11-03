//! Audio transcription DTOs and transcriber trait.

use std::{fmt, sync::Arc};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{ErrorCore, ErrorKind, SafeErrorSource};

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
#[derive(Error)]
pub enum TranscriptionError {
    /// The client submitted an empty audio payload.
    #[error("[ai_interface/audio_transcriber] audio must not be empty")]
    EmptyAudio {
        /// Shared failure details, including the adapter identity.
        core: Box<ErrorCore>,
    },
    /// The client submitted an unsupported media type.
    #[error("[ai_interface/audio_transcriber] unsupported media type `{content_type}`")]
    UnsupportedMediaType {
        /// Shared failure details, including the adapter identity.
        core: Box<ErrorCore>,
        /// Unsupported MIME media type.
        content_type: String,
    },
    /// The client submitted audio larger than the accepted limit.
    #[error(
        "[ai_interface/audio_transcriber] audio is too large: {actual_bytes} bytes exceeds {max_bytes} bytes"
    )]
    AudioTooLarge {
        /// Shared failure details, including the adapter identity.
        core: Box<ErrorCore>,
        /// Maximum accepted audio size in bytes.
        max_bytes: usize,
        /// Submitted audio size in bytes.
        actual_bytes: usize,
    },
    /// The provider returned no usable transcript text.
    #[error("[ai_interface/audio_transcriber] transcript text was empty")]
    EmptyTranscript {
        /// Shared failure details, including the adapter identity.
        core: Box<ErrorCore>,
    },
    /// A classified provider or adapter failure.
    #[error("[ai_interface/audio_transcriber] {kind:?} for `{provider}` model `{model_id}`: {message}", kind = core.kind, provider = core.provider, model_id = core.model_id, message = core.message)]
    Failure {
        /// Shared failure details.
        core: Box<ErrorCore>,
    },
    /// A classified failure with its original source object.
    #[error("[ai_interface/audio_transcriber] {kind:?} for `{provider}` model `{model_id}`: {message}", kind = core.kind, provider = core.provider, model_id = core.model_id, message = core.message)]
    Internal {
        /// Shared failure details, including the adapter identity.
        core: Box<ErrorCore>,
        /// Original source retained for inspection and downcasting.
        source: SafeErrorSource,
    },
}

impl fmt::Debug for TranscriptionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("TranscriptionError")
            .field("core", self.core())
            .finish()
    }
}

impl TranscriptionError {
    /// Builds a classified failure from its shared details.
    pub fn new(core: ErrorCore) -> Self {
        Self::Failure {
            core: Box::new(core),
        }
    }

    /// Returns the shared failure details.
    pub fn core(&self) -> &ErrorCore {
        match self {
            Self::EmptyAudio { core }
            | Self::UnsupportedMediaType { core, .. }
            | Self::AudioTooLarge { core, .. }
            | Self::EmptyTranscript { core }
            | Self::Failure { core }
            | Self::Internal { core, .. } => core,
        }
    }

    /// Returns shared details for metadata attachment or secret redaction.
    pub fn core_mut(&mut self) -> &mut ErrorCore {
        match self {
            Self::EmptyAudio { core }
            | Self::UnsupportedMediaType { core, .. }
            | Self::AudioTooLarge { core, .. }
            | Self::EmptyTranscript { core }
            | Self::Failure { core }
            | Self::Internal { core, .. } => core,
        }
    }

    /// Builds an empty-audio error with the selected adapter identity.
    pub fn empty_audio(provider: impl Into<String>, model_id: impl Into<String>) -> Self {
        Self::EmptyAudio {
            core: Box::new(ErrorCore::new(
                ErrorKind::InvalidRequest,
                provider,
                model_id,
                "audio must not be empty",
            )),
        }
    }

    /// Builds a media-type error with the selected adapter identity.
    pub fn unsupported_media_type(
        provider: impl Into<String>,
        model_id: impl Into<String>,
        content_type: impl Into<String>,
    ) -> Self {
        let content_type = content_type.into();
        Self::UnsupportedMediaType {
            core: Box::new(ErrorCore::new(
                ErrorKind::UnsupportedInput,
                provider,
                model_id,
                format!("unsupported media type `{content_type}`"),
            )),
            content_type,
        }
    }

    /// Builds an audio-size error with the selected adapter identity.
    pub fn audio_too_large(
        provider: impl Into<String>,
        model_id: impl Into<String>,
        max_bytes: usize,
        actual_bytes: usize,
    ) -> Self {
        Self::AudioTooLarge {
            core: Box::new(ErrorCore::new(
                ErrorKind::UnsupportedInput,
                provider,
                model_id,
                format!("audio is too large: {actual_bytes} bytes exceeds {max_bytes} bytes"),
            )),
            max_bytes,
            actual_bytes,
        }
    }

    /// Builds an empty-output error with the selected adapter identity.
    pub fn empty_transcript(provider: impl Into<String>, model_id: impl Into<String>) -> Self {
        Self::EmptyTranscript {
            core: Box::new(ErrorCore::new(
                ErrorKind::InvalidOutput,
                provider,
                model_id,
                "transcript text was empty",
            )),
        }
    }

    /// Retains an original source with an explicit failure category.
    pub fn with_source(core: ErrorCore, source: SafeErrorSource) -> Self {
        Self::Internal {
            core: Box::new(core),
            source,
        }
    }

    /// Wraps an internal failure with the selected adapter identity.
    pub fn internal(
        provider: impl Into<String>,
        model_id: impl Into<String>,
        source: SafeErrorSource,
    ) -> Self {
        Self::with_source(
            ErrorCore::new(ErrorKind::Internal, provider, model_id, "internal error"),
            source,
        )
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
