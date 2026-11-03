//! Model failures with shared error details and earlier fallback attempts.

use std::fmt;

use thiserror::Error;

use crate::{ErrorCore, ErrorKind, ModelAttempt, ModelControl, SafeErrorSource};

#[derive(Error)]
/// Errors returned by the model boundary.
pub enum ModelError {
    /// A classified provider or adapter failure.
    #[error("[ai_interface/model] {kind:?} for `{provider}` model `{model_id}`: {message}", kind = core.kind, provider = core.provider, model_id = core.model_id, message = core.message)]
    Failure {
        /// Shared failure details.
        core: Box<ErrorCore>,
        /// Earlier failed fallback attempts, excluding this failure.
        attempts: Vec<ModelAttempt>,
    },
    /// A classified failure with its original source object.
    #[error("[ai_interface/model] {kind:?} for `{provider}` model `{model_id}`: {message}", kind = core.kind, provider = core.provider, model_id = core.model_id, message = core.message)]
    Internal {
        /// Shared failure details.
        core: Box<ErrorCore>,
        /// Earlier failed fallback attempts, excluding this failure.
        attempts: Vec<ModelAttempt>,
        /// Original source retained for inspection and downcasting.
        source: SafeErrorSource,
    },
}

impl fmt::Debug for ModelError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ModelError")
            .field("core", self.core())
            .field("attempts", &self.attempts())
            .finish()
    }
}

impl ModelError {
    /// Builds a classified failure from one shared core.
    pub fn new(core: ErrorCore) -> Self {
        Self::Failure {
            core: Box::new(core),
            attempts: Vec::new(),
        }
    }

    /// Builds a failure with an explicit category and adapter identity.
    pub fn failure(
        kind: ErrorKind,
        provider: impl Into<String>,
        model_id: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self::new(ErrorCore::new(kind, provider, model_id, message))
    }

    /// Returns the shared failure details.
    pub fn core(&self) -> &ErrorCore {
        match self {
            Self::Failure { core, .. } | Self::Internal { core, .. } => core,
        }
    }

    /// Returns the shared details for metadata attachment or secret redaction.
    pub fn core_mut(&mut self) -> &mut ErrorCore {
        match self {
            Self::Failure { core, .. } | Self::Internal { core, .. } => core,
        }
    }

    /// Returns earlier failed attempts in chronological order.
    pub fn attempts(&self) -> &[ModelAttempt] {
        match self {
            Self::Failure { attempts, .. } | Self::Internal { attempts, .. } => attempts,
        }
    }

    /// Removes and returns the earlier failed attempts.
    pub fn take_attempts(&mut self) -> Vec<ModelAttempt> {
        match self {
            Self::Failure { attempts, .. } | Self::Internal { attempts, .. } => {
                std::mem::take(attempts)
            }
        }
    }

    /// Replaces the earlier attempts while preserving this failure and its source.
    pub fn with_attempts(mut self, earlier: Vec<ModelAttempt>) -> Self {
        match &mut self {
            Self::Failure { attempts, .. } | Self::Internal { attempts, .. } => *attempts = earlier,
        }
        self
    }

    /// Adds outer fallback history before any existing inner fallback history.
    pub fn prepend_attempts(mut self, mut earlier: Vec<ModelAttempt>) -> Self {
        earlier.extend(self.take_attempts());
        self.with_attempts(earlier)
    }

    /// Snapshots only this failed attempt, excluding its earlier attempts.
    pub fn attempt(&self, emitted_events: bool) -> ModelAttempt {
        let core = self.core();
        ModelAttempt {
            provider: core.provider.clone(),
            model_id: core.model_id.clone(),
            kind: core.kind,
            message: core.message.clone(),
            emitted_events,
        }
    }

    /// Reconstructs a recorded failure without its original source or response metadata.
    pub fn from_attempt(attempt: ModelAttempt) -> Self {
        Self::failure(
            attempt.kind,
            attempt.provider,
            attempt.model_id,
            attempt.message,
        )
    }

    /// Wraps an internal failure with the identity of the adapter that raised it.
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

    /// Retains an original source with an explicit failure category.
    pub fn with_source(core: ErrorCore, source: SafeErrorSource) -> Self {
        Self::Internal {
            core: Box::new(core),
            attempts: Vec::new(),
            source,
        }
    }

    pub(crate) fn fold_error(source: impl std::error::Error + Send + Sync + 'static) -> Self {
        Self::internal(
            "local",
            "model-response-fold",
            SafeErrorSource::redacted(source),
        )
    }

    /// Builds a rate limit.
    pub fn rate_limited(
        provider: impl Into<String>,
        model_id: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self::failure(ErrorKind::RateLimited, provider, model_id, message)
    }

    /// Builds a failure before provider progress.
    pub fn transient(
        provider: impl Into<String>,
        model_id: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self::failure(ErrorKind::Transient, provider, model_id, message)
    }

    /// Builds a stream failure after provider progress.
    pub fn interrupted(
        provider: impl Into<String>,
        model_id: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self::failure(ErrorKind::Interrupted, provider, model_id, message)
    }

    /// Builds a request that breaks the input contract.
    pub fn invalid_request(
        provider: impl Into<String>,
        model_id: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self::failure(ErrorKind::InvalidRequest, provider, model_id, message)
    }

    /// Builds provider output that breaks the response contract.
    pub fn invalid_output(
        provider: impl Into<String>,
        model_id: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self::failure(ErrorKind::InvalidOutput, provider, model_id, message)
    }

    /// Builds an input above the model context limit.
    pub fn context_limit_exceeded(
        provider: impl Into<String>,
        model_id: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self::failure(ErrorKind::ContextLimitExceeded, provider, model_id, message)
    }

    /// Builds a failure for an explicit control that the model cannot honor.
    pub fn unsupported_control(
        provider: impl Into<String>,
        model_id: impl Into<String>,
        control: ModelControl,
    ) -> Self {
        Self::failure(
            ErrorKind::UnsupportedControl(control),
            provider,
            model_id,
            format!("unsupported {control:?} control"),
        )
    }
}
