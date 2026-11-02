//! Errors returned by the judgment boundary.

use std::fmt;

use internal_error::{DefinedAt, InternalError};
use thiserror::Error;

use crate::{ErrorCore, ErrorKind};

use super::{JudgmentAnswerProblem, JudgmentJsonType};

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
#[derive(Error)]
pub enum JudgmentError {
    /// Content used a scalar JSON kind outside the judgment contract.
    #[error(
        "[ai_interface/judgment] unsupported {json_type:?} content; use text, an object, or an array"
    )]
    UnsupportedContent {
        /// Shared failure details.
        core: Box<ErrorCore>,
        /// Unsupported JSON scalar kind.
        json_type: JudgmentJsonType,
    },
    /// Request state was blank or structurally empty.
    #[error("[ai_interface/judgment] state must not be empty")]
    EmptyState {
        /// Shared failure details.
        core: Box<ErrorCore>,
    },
    /// A request did not contain any questions.
    #[error("[ai_interface/judgment] at least one question is required")]
    NoQuestions {
        /// Shared failure details.
        core: Box<ErrorCore>,
    },
    /// A question failed local validation.
    #[error("[ai_interface/judgment] invalid question `{id}`: {problem}")]
    InvalidQuestion {
        /// Shared failure details.
        core: Box<ErrorCore>,
        /// Caller-provided question id.
        id: String,
        /// Specific validation problem.
        problem: JudgmentQuestionProblem,
    },
    /// A provider answer failed shared response postconditions.
    #[error("[ai_interface/judgment] invalid answer `{id}` from `{provider}` model `{model_id}`: {problem}", provider = core.provider, model_id = core.model_id)]
    InvalidAnswer {
        /// Shared failure details.
        core: Box<ErrorCore>,
        /// Caller-provided question id.
        id: String,
        /// Specific answer postcondition failure.
        problem: JudgmentAnswerProblem,
    },
    /// A classified provider or adapter failure.
    #[error("[ai_interface/judgment] {kind:?} for `{provider}` model `{model_id}`: {message}", kind = core.kind, provider = core.provider, model_id = core.model_id, message = core.message)]
    Failure {
        /// Shared failure details.
        core: Box<ErrorCore>,
    },
    /// A classified failure with tracked definition and caller locations.
    #[error("[ai_interface/judgment] {kind:?} for `{provider}` model `{model_id}`: {message}", kind = core.kind, provider = core.provider, model_id = core.model_id, message = core.message)]
    Internal {
        /// Shared failure details.
        core: Box<ErrorCore>,
        /// Original source and its tracked locations.
        source: InternalError,
    },
}

impl fmt::Debug for JudgmentError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("JudgmentError")
            .field("core", self.core())
            .finish()
    }
}

impl JudgmentError {
    /// Builds a question-validation error before an adapter is selected.
    pub fn invalid_question(id: impl Into<String>, problem: JudgmentQuestionProblem) -> Self {
        let id = id.into();
        Self::InvalidQuestion {
            core: Box::new(ErrorCore::local(
                ErrorKind::InvalidRequest,
                "judgment-validation",
                format!("invalid question `{id}`: {problem}"),
            )),
            id,
            problem,
        }
    }

    /// Builds an invalid-answer error with its typed problem and adapter identity.
    pub fn invalid_answer(
        provider: impl Into<String>,
        model_id: impl Into<String>,
        id: impl Into<String>,
        problem: JudgmentAnswerProblem,
    ) -> Self {
        let id = id.into();
        Self::InvalidAnswer {
            core: Box::new(ErrorCore::new(
                ErrorKind::InvalidOutput,
                provider,
                model_id,
                format!("invalid answer `{id}`: {problem}"),
            )),
            id,
            problem,
        }
    }

    /// Wraps an internal failure without losing its source or caller location.
    #[track_caller]
    pub fn internal(
        provider: impl Into<String>,
        model_id: impl Into<String>,
        source: impl std::error::Error + Send + Sync + 'static,
    ) -> Self {
        Self::with_source(
            ErrorCore::new(ErrorKind::Internal, provider, model_id, "internal error"),
            source,
        )
    }

    /// Retains a source and its tracked locations with an explicit failure category.
    #[track_caller]
    pub fn with_source(
        core: ErrorCore,
        source: impl std::error::Error + Send + Sync + 'static,
    ) -> Self {
        const DEFINED_AT: DefinedAt = DefinedAt::new(module_path!(), file!(), line!(), column!());
        Self::Internal {
            core: Box::new(core),
            source: InternalError::from_source(DEFINED_AT, source),
        }
    }
}

/// Result alias for judgment operations.
pub type JudgmentResult<T> = std::result::Result<T, JudgmentError>;

#[cfg(test)]
#[path = "_tests_/error_tests.rs"]
mod error_tests;
