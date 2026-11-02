//! Shared-core access and pre-adapter identities for judgment validation.

use crate::{ErrorCore, ErrorKind, JudgmentError, JudgmentJsonType};

impl JudgmentError {
    /// Builds a classified failure from its shared details.
    pub fn new(core: ErrorCore) -> Self {
        Self::Failure {
            core: Box::new(core),
        }
    }

    /// Returns the shared failure details.
    pub fn core(&self) -> &ErrorCore {
        match self {
            Self::UnsupportedContent { core, .. }
            | Self::EmptyState { core }
            | Self::NoQuestions { core }
            | Self::InvalidQuestion { core, .. }
            | Self::InvalidAnswer { core, .. }
            | Self::Failure { core }
            | Self::Internal { core, .. } => core,
        }
    }

    /// Returns shared details for metadata attachment or secret redaction.
    pub fn core_mut(&mut self) -> &mut ErrorCore {
        match self {
            Self::UnsupportedContent { core, .. }
            | Self::EmptyState { core }
            | Self::NoQuestions { core }
            | Self::InvalidQuestion { core, .. }
            | Self::InvalidAnswer { core, .. }
            | Self::Failure { core }
            | Self::Internal { core, .. } => core,
        }
    }

    /// Attaches the selected adapter identity to an earlier local validation failure.
    pub fn with_identity(
        mut self,
        provider: impl Into<String>,
        model_id: impl Into<String>,
    ) -> Self {
        let core = self.core_mut();
        core.provider = provider.into();
        core.model_id = model_id.into();
        self
    }

    /// Builds a content-validation error before an adapter is selected.
    pub fn unsupported_content(json_type: JudgmentJsonType) -> Self {
        Self::UnsupportedContent {
            core: Box::new(ErrorCore::local(
                ErrorKind::InvalidRequest,
                "judgment-validation",
                format!("unsupported {json_type:?} content; use text, an object, or an array"),
            )),
            json_type,
        }
    }

    /// Builds an empty-state error before an adapter is selected.
    pub fn empty_state() -> Self {
        Self::EmptyState {
            core: Box::new(ErrorCore::local(
                ErrorKind::InvalidRequest,
                "judgment-validation",
                "state must not be empty",
            )),
        }
    }

    /// Builds a missing-questions error before an adapter is selected.
    pub fn no_questions() -> Self {
        Self::NoQuestions {
            core: Box::new(ErrorCore::local(
                ErrorKind::InvalidRequest,
                "judgment-validation",
                "at least one question is required",
            )),
        }
    }
}
