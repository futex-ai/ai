//! Decisions errors with response metadata and redacted diagnostics.

use ai_interface::{ErrorCore, ErrorKind, JudgmentAnswerProblem, JudgmentError, JudgmentResult};
use ai_models_core::{ErrorContext, classify_http_status};
use serde::Deserialize;
use serde_json::Value;

use crate::openai::error_context::native_kind;

#[derive(Deserialize)]
struct ErrorEnvelope {
    error: ErrorBody,
}

#[derive(Deserialize)]
struct ErrorBody {
    code: Option<Value>,
    message: Option<Value>,
}

pub(super) fn classify_response(context: &ErrorContext, status: u16, body: &[u8]) -> JudgmentError {
    JudgmentError::new(status_core(context, status, &String::from_utf8_lossy(body)))
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
            match body.as_str() {
                Some(text) => status_core(&context, *status, text),
                None => status_core(&context, *status, &body.to_string()),
            }
        }
        _ => context.request_error(&source),
    };
    JudgmentError::with_source(core, context.source(source))
}

fn status_core(context: &ErrorContext, status: u16, body: &str) -> ErrorCore {
    let envelope = serde_json::from_str::<ErrorEnvelope>(body).ok();
    let details = envelope.as_ref().map(|envelope| &envelope.error);
    let code = details
        .and_then(|error| error.code.as_ref())
        .and_then(Value::as_str);
    let kind = native_kind(code)
        .or_else(|| classify_http_status(status))
        .unwrap_or(ErrorKind::InvalidOutput);
    let message = details
        .and_then(|error| error.message.as_ref())
        .and_then(Value::as_str)
        .unwrap_or(body);
    context.core(kind, message)
}

pub(super) fn decode<'a, T: Deserialize<'a>>(
    context: &ErrorContext,
    body: &'a [u8],
) -> JudgmentResult<T> {
    match serde_json::from_slice(body) {
        Ok(parsed) => Ok(parsed),
        Err(source) => Err(JudgmentError::with_source(
            context.core(
                ErrorKind::InvalidOutput,
                format!("malformed provider payload: {source}"),
            ),
            context.source(source),
        )),
    }
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
