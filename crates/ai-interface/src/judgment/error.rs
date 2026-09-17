//! Errors returned by the judgment boundary.

use std::fmt;

use internal_error::InternalError;
use thiserror::Error;

use super::JudgmentJsonType;

/// Typed local validation problem for one judgment question.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum JudgmentQuestionProblem {
    /// The caller-provided question map key is blank.
    BlankId,
    /// A choice question has no options.
    NoOptions,
    /// A choice question has a blank option label.
    BlankOptionLabel,
    /// A score question has fewer than two ordered levels.
    TooFewLevels {
        /// Number of levels supplied by the caller.
        levels: usize,
    },
}

impl fmt::Display for JudgmentQuestionProblem {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BlankId => formatter.write_str("question id must not be blank"),
            Self::NoOptions => formatter.write_str("choice must include at least one option"),
            Self::BlankOptionLabel => formatter.write_str("choice option labels must not be blank"),
            Self::TooFewLevels { levels } => {
                write!(
                    formatter,
                    "score must include at least two levels, got {levels}"
                )
            }
        }
    }
}

/// Errors returned by judgment request validation and providers.
#[derive(Debug, Error, internal_error::ErrorContract)]
pub enum JudgmentError {
    /// Content used a scalar JSON kind outside the judgment contract.
    #[error(
        "[ai_interface/judgment] unsupported {json_type:?} content; use text, an object, or an array"
    )]
    UnsupportedContent {
        /// Unsupported JSON scalar kind.
        json_type: JudgmentJsonType,
    },
    /// Request state was blank or structurally empty.
    #[error("[ai_interface/judgment] state must not be empty")]
    EmptyState,
    /// A request did not contain any questions.
    #[error("[ai_interface/judgment] at least one question is required")]
    NoQuestions,
    /// A question failed local validation.
    #[error("[ai_interface/judgment] invalid question `{id}`: {problem}")]
    InvalidQuestion {
        /// Caller-provided question id.
        id: String,
        /// Specific validation problem.
        problem: JudgmentQuestionProblem,
    },
    /// The provider rejected the request due to a rate limit.
    #[error(
        "[ai_interface/judgment] provider rate limit for `{provider}` model `{model_id}`: {message}"
    )]
    RateLimited {
        /// Provider that returned the rate limit.
        provider: String,
        /// Configured provider model identifier.
        model_id: String,
        /// Provider-supplied failure details.
        message: String,
    },
    /// The provider returned a transient failure that may succeed later.
    #[error(
        "[ai_interface/judgment] transient provider failure for `{provider}` model `{model_id}`: {message}"
    )]
    TransientProvider {
        /// Provider that returned the transient failure.
        provider: String,
        /// Configured provider model identifier.
        model_id: String,
        /// Provider-supplied failure details.
        message: String,
    },
    /// The provider returned a non-retryable failure.
    #[error(
        "[ai_interface/judgment] provider failure for `{provider}` model `{model_id}`: {message}"
    )]
    Provider {
        /// Provider that returned the failure.
        provider: String,
        /// Configured provider model identifier.
        model_id: String,
        /// Provider-supplied failure details.
        message: String,
    },
    /// Unhandled judgment-boundary failure.
    #[error("[ai_interface/judgment] internal error")]
    Internal(#[from] InternalError),
}

impl JudgmentError {
    /// Builds an invalid-question error.
    pub fn invalid_question(id: impl Into<String>, problem: JudgmentQuestionProblem) -> Self {
        Self::InvalidQuestion {
            id: id.into(),
            problem,
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
}

/// Result alias for judgment operations.
pub type JudgmentResult<T> = std::result::Result<T, JudgmentError>;
