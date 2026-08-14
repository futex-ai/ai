//! OpenAI video generation error classification.

use ai_interface::VideoGenerationError;
use serde::Deserialize;
use serde_json::Value;

const PROVIDER: &str = "openai";

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

pub(super) fn classify_status(status: u16, model_id: &str, body: &Value) -> VideoGenerationError {
    let parsed = serde_json::from_value::<ErrorEnvelope>(body.clone()).ok();
    let details = parsed.as_ref().and_then(|envelope| envelope.error.as_ref());
    let message = details
        .and_then(|error| error.message.as_deref())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| body.to_string());
    if status == 429 {
        return VideoGenerationError::rate_limited(PROVIDER, model_id, message);
    }
    if matches!(status, 408 | 409 | 425) || (500..=599).contains(&status) {
        return VideoGenerationError::transient_provider(PROVIDER, model_id, message);
    }
    if details.is_some_and(is_content_policy) {
        return VideoGenerationError::content_policy(PROVIDER, model_id, message);
    }
    VideoGenerationError::provider(PROVIDER, model_id, message)
}

pub(super) fn classify_request_error(
    source: json_http::Error,
    model_id: &str,
) -> VideoGenerationError {
    match source {
        json_http::Error::Transport { .. } | json_http::Error::Auth { .. } => {
            VideoGenerationError::transient_provider(PROVIDER, model_id, source.to_string())
        }
        json_http::Error::SerializeRequest { .. }
        | json_http::Error::DeserializeResponse { .. } => VideoGenerationError::internal(source),
    }
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
