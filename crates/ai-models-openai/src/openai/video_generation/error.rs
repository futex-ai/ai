//! OpenAI video HTTP classification with per-request metadata and safe diagnostics.

use ai_interface::{ErrorKind, VideoGenerationError};
use ai_models_core::{ErrorContext, json_http_error_core};
use json_http::JsonHttpHeaders;
use serde::Deserialize;
use serde_json::Value;

#[derive(Deserialize)]
struct ErrorEnvelope {
    error: Option<ErrorBody>,
}

#[derive(Deserialize)]
struct ErrorBody {
    code: Option<String>,
    #[serde(rename = "type")]
    kind: Option<String>,
    message: Option<String>,
}

pub(super) fn classify_status(
    context: &ErrorContext,
    status: u16,
    headers: &JsonHttpHeaders,
    body: &Value,
) -> VideoGenerationError {
    let parsed = serde_json::from_value::<ErrorEnvelope>(body.clone()).ok();
    let details = parsed.as_ref().and_then(|envelope| envelope.error.as_ref());
    let message = details
        .and_then(|error| error.message.as_deref())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| body.to_string());
    let mut core = json_http_error_core(context, status, headers, body);
    core.message = context.redact(&message);
    if !matches!(
        core.kind,
        ErrorKind::RateLimited | ErrorKind::Transient | ErrorKind::ContextLimitExceeded
    ) && details.is_some_and(is_content_policy)
    {
        core.kind = ErrorKind::ContentPolicy;
    }
    VideoGenerationError::new(core)
}

pub(super) fn classify_request_error(
    context: &ErrorContext,
    source: json_http::Error,
) -> VideoGenerationError {
    VideoGenerationError::with_source(context.request_error(&source), source)
}

fn is_content_policy(error: &ErrorBody) -> bool {
    [error.code.as_deref(), error.kind.as_deref()]
        .into_iter()
        .flatten()
        .any(|value| {
            matches!(
                value,
                "content_policy_violation"
                    | "moderation_blocked"
                    | "safety_violation"
                    | "video_generation_safety_violation"
            )
        })
}
