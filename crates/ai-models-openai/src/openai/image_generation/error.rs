//! OpenAI image generation error classification.

use ai_interface::ImageGenerationError;
use reqwest::StatusCode;
use serde::Deserialize;

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

pub(super) fn classify_status(
    status: StatusCode,
    model_id: &str,
    body: &str,
) -> ImageGenerationError {
    let parsed = serde_json::from_str::<ErrorEnvelope>(body).ok();
    let details = parsed.as_ref().and_then(|envelope| envelope.error.as_ref());
    let message = details
        .and_then(|error| error.message.as_deref())
        .unwrap_or(body)
        .to_owned();
    if status == StatusCode::TOO_MANY_REQUESTS {
        return ImageGenerationError::rate_limited(PROVIDER, model_id, message);
    }
    if is_transient_status(status) {
        return ImageGenerationError::transient_provider(PROVIDER, model_id, message);
    }
    if details.is_some_and(is_content_policy) {
        return ImageGenerationError::content_policy(PROVIDER, model_id, message);
    }
    ImageGenerationError::provider(PROVIDER, model_id, message)
}

pub(super) fn request_error(model_id: &str, message: impl Into<String>) -> ImageGenerationError {
    ImageGenerationError::transient_provider(PROVIDER, model_id, message)
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
                    | "image_generation_safety_violation"
            )
        })
}

fn is_transient_status(status: StatusCode) -> bool {
    matches!(status, StatusCode::REQUEST_TIMEOUT | StatusCode::CONFLICT)
        || status.as_u16() == 425
        || status.is_server_error()
}
