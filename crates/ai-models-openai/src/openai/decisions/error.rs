//! Decisions errors with response metadata and redacted diagnostics.

use std::borrow::Cow;

use ai_interface::{
    ErrorCore, ErrorKind, JudgmentAnswerProblem, JudgmentError, JudgmentResult, SafeErrorSource,
};
use ai_models_core::{ErrorContext, classify_http_status};
use serde::Deserialize;
use serde_json::Value;

use crate::openai::error_context::native_kind;

pub(super) fn classify_response(context: &ErrorContext, status: u16, body: &[u8]) -> JudgmentError {
    JudgmentError::new(status_core(context, status, || {
        serde_json::from_slice(body).ok().map(Cow::Owned)
    }))
}

pub(super) fn classify_request_error(
    context: &ErrorContext,
    source: json_http::Error,
) -> JudgmentError {
    let core = match source.cause() {
        json_http::Error::HttpStatus {
            status,
            headers,
            body,
        } => {
            let context = context.for_response(source.response_headers().unwrap_or(headers));
            status_core(&context, *status, || Some(Cow::Borrowed(body)))
        }
        json_http::Error::DeserializeResponse {
            source: decoder, ..
        } => {
            let mut core = context.request_error(&source);
            core.message = decoder_diagnostic(decoder);
            return JudgmentError::with_source(core, SafeErrorSource::redacted(source));
        }
        _ => context.request_error(&source),
    };
    JudgmentError::with_source(core, context.source(source))
}

fn status_core<'a>(
    context: &ErrorContext,
    status: u16,
    body: impl FnOnce() -> Option<Cow<'a, Value>>,
) -> ErrorCore {
    let status_kind = classify_http_status(status).unwrap_or(ErrorKind::InvalidOutput);
    if (300..400).contains(&status) {
        return context.core(status_kind, "unexpected HTTP redirect");
    }
    let body = body();
    let details = body.as_deref().and_then(|body| body.get("error"));
    let code = details
        .and_then(|error| error.get("code"))
        .and_then(Value::as_str);
    let kind = native_kind(code).unwrap_or(status_kind);
    let message = details
        .and_then(|error| error.get("message"))
        .and_then(Value::as_str);
    match message {
        Some(message) => context.core(kind, message),
        None => context.core(kind, format!("HTTP {status}")),
    }
}

pub(super) fn decode<'a, T: Deserialize<'a>>(
    context: &ErrorContext,
    body: &'a [u8],
) -> JudgmentResult<T> {
    match serde_json::from_slice(body) {
        Ok(parsed) => Ok(parsed),
        Err(source) => Err(JudgmentError::with_source(
            context.core(ErrorKind::InvalidOutput, decoder_diagnostic(&source)),
            SafeErrorSource::redacted(source),
        )),
    }
}

fn decoder_diagnostic(source: &serde_json::Error) -> String {
    format!(
        "malformed provider payload: {:?} at line {} column {}",
        source.classify(),
        source.line(),
        source.column()
    )
}

pub(super) fn invalid_output(context: &ErrorContext, message: &str) -> JudgmentError {
    JudgmentError::new(context.core(ErrorKind::InvalidOutput, message))
}

pub(super) fn redact_answer_error(
    context: &ErrorContext,
    mut error: JudgmentError,
) -> JudgmentError {
    if let JudgmentError::InvalidAnswer {
        problem: JudgmentAnswerProblem::SelectedNotMaximal { selected, maximal },
        ..
    } = &mut error
    {
        *selected = context.redact(selected);
        *maximal = context.redact(maximal);
    }
    context.apply_judgment_error(error)
}
