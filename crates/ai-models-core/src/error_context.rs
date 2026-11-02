//! Per-call identity, secret capture, and immutable response error context.

use std::{
    fmt,
    sync::{Arc, Mutex},
};

use ai_interface::{ErrorCore, ErrorKind, JudgmentAnswerProblem, JudgmentError, ModelError};
use json_http::{DynJsonHttpAuth, JsonHttpHeaders};

use crate::{Clock, ErrorMetadata, capturing_auth::capture, errors::request_error_core};

#[derive(Clone)]
/// Error construction context scoped to one logical model or modality call.
pub struct ErrorContext {
    provider: String,
    model_id: String,
    request_id_headers: Vec<String>,
    clock: Arc<dyn Clock>,
    secrets: Arc<Mutex<Vec<String>>>,
    response_headers: Option<Arc<JsonHttpHeaders>>,
}

impl fmt::Debug for ErrorContext {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ErrorContext")
            .field("provider", &self.provider)
            .field("model_id", &self.model_id)
            .finish_non_exhaustive()
    }
}

impl ErrorContext {
    /// Starts a response-free context with adapter identity and an injected wall clock.
    pub fn new(
        provider: impl Into<String>,
        model_id: impl Into<String>,
        request_id_headers: &[&str],
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            provider: provider.into(),
            model_id: model_id.into(),
            request_id_headers: request_id_headers
                .iter()
                .map(|name| (*name).to_owned())
                .collect(),
            clock,
            secrets: Arc::new(Mutex::new(Vec::new())),
            response_headers: None,
        }
    }

    /// Clones the call context with facts of this response only.
    pub fn for_response(&self, headers: &JsonHttpHeaders) -> Self {
        let mut context = self.clone();
        context.response_headers = Some(Arc::new(headers.clone()));
        context
    }

    /// Captures values from the actual auth hook without applying it a second time.
    pub fn capture_auth(&self, inner: DynJsonHttpAuth) -> DynJsonHttpAuth {
        capture(self.clone(), inner)
    }

    /// Adds an explicit credential to this call's secret collection.
    pub fn add_secret(&self, secret: impl Into<String>) {
        let secret = secret.into();
        if secret.is_empty() {
            return;
        }
        let mut secrets = self
            .secrets
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        if !secrets.contains(&secret) {
            secrets.push(secret);
        }
    }

    /// Redacts known credentials, longest values first.
    pub fn redact(&self, text: &str) -> String {
        let mut secrets = self
            .secrets
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .clone();
        secrets.sort_by_key(|secret| std::cmp::Reverse(secret.len()));
        secrets.into_iter().fold(text.to_owned(), |text, secret| {
            text.replace(&secret, "[redacted]")
        })
    }

    /// Applies actual adapter identity, response metadata, and credential redaction.
    pub fn apply(&self, core: &mut ErrorCore) {
        core.provider = self.provider.clone();
        core.model_id = self.model_id.clone();
        if let Some(headers) = &self.response_headers {
            let names = self
                .request_id_headers
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>();
            ErrorMetadata::from_headers(headers, &names, self.clock.as_ref()).apply(core);
        }
        core.message = self.redact(&core.message);
        core.provider_request_id = core
            .provider_request_id
            .as_deref()
            .map(|id| self.redact(id));
    }

    /// Builds safe shared details with this context's identity and response facts.
    pub fn core(&self, kind: ErrorKind, message: impl Into<String>) -> ErrorCore {
        let mut core = ErrorCore::new(kind, &self.provider, &self.model_id, message);
        self.apply(&mut core);
        core
    }

    /// Builds a classified model failure with this context.
    pub fn model_error(&self, kind: ErrorKind, message: impl Into<String>) -> ModelError {
        ModelError::new(self.core(kind, message))
    }

    /// Finalizes a model error without replacing its kind, source, or earlier history.
    pub fn apply_model_error(&self, mut error: ModelError) -> ModelError {
        self.apply(error.core_mut());
        let attempts = error
            .take_attempts()
            .into_iter()
            .map(|mut attempt| {
                attempt.message = self.redact(&attempt.message);
                attempt
            })
            .collect();
        error.with_attempts(attempts)
    }

    /// Finalizes judgment diagnostics and its provider-controlled typed labels.
    pub fn apply_judgment_error(&self, mut error: JudgmentError) -> JudgmentError {
        if let JudgmentError::InvalidAnswer {
            problem:
                JudgmentAnswerProblem::UnknownOption { label }
                | JudgmentAnswerProblem::MissingOption { label },
            ..
        } = &mut error
        {
            *label = self.redact(label);
        }
        self.apply(error.core_mut());
        error
    }

    /// Classifies a buffered request failure using only that failed response's headers.
    pub fn request_error(&self, source: &json_http::Error) -> ErrorCore {
        let mut context = self.clone();
        context.response_headers = None;
        if let Some(headers) = source.response_headers() {
            context = context.for_response(headers);
        }
        request_error_core(&context, source)
    }
}
