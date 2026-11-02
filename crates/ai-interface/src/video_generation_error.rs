//! Shared-core access and typed constructors for video generation failures.

use crate::{ErrorCore, ErrorKind, VideoGenerationError};

impl VideoGenerationError {
    /// Builds a classified failure from its shared details.
    pub fn new(core: ErrorCore) -> Self {
        Self::Failure {
            core: Box::new(core),
        }
    }

    /// Returns the shared failure details.
    pub fn core(&self) -> &ErrorCore {
        match self {
            Self::EmptyPrompt { core }
            | Self::UnsupportedMediaType { core, .. }
            | Self::NoVideo { core }
            | Self::Failure { core }
            | Self::Internal { core, .. } => core,
        }
    }

    /// Returns shared details for metadata attachment or secret redaction.
    pub fn core_mut(&mut self) -> &mut ErrorCore {
        match self {
            Self::EmptyPrompt { core }
            | Self::UnsupportedMediaType { core, .. }
            | Self::NoVideo { core }
            | Self::Failure { core }
            | Self::Internal { core, .. } => core,
        }
    }

    /// Builds a blank-prompt error with the selected adapter identity.
    pub fn empty_prompt(provider: impl Into<String>, model_id: impl Into<String>) -> Self {
        Self::EmptyPrompt {
            core: Box::new(ErrorCore::new(
                ErrorKind::InvalidRequest,
                provider,
                model_id,
                "prompt must not be empty",
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

    /// Builds an error for a successful response without a final video.
    pub fn no_video(provider: impl Into<String>, model_id: impl Into<String>) -> Self {
        Self::NoVideo {
            core: Box::new(ErrorCore::new(
                ErrorKind::InvalidOutput,
                provider,
                model_id,
                "no video returned",
            )),
        }
    }

    /// Builds a content-policy refusal.
    pub fn content_policy(
        provider: impl Into<String>,
        model_id: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self::new(ErrorCore::new(
            ErrorKind::ContentPolicy,
            provider,
            model_id,
            message,
        ))
    }

    /// Builds an error for a video job that exceeded its total timeout.
    pub fn timed_out(provider: impl Into<String>, model_id: impl Into<String>) -> Self {
        Self::new(ErrorCore::new(
            ErrorKind::TimedOut,
            provider,
            model_id,
            "generation timed out",
        ))
    }
}
