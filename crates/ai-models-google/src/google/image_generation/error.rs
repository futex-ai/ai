//! Google image generation error classification.

use ai_interface::ImageGenerationError;
use serde_json::Value;

const PROVIDER: &str = "google";

pub(super) fn classify_status(status: u16, model_id: &str, body: &Value) -> ImageGenerationError {
    let message = error_message(body).unwrap_or_else(|| format!("HTTP {status}"));
    if status == 429 {
        return ImageGenerationError::rate_limited(PROVIDER, model_id, message);
    }
    if matches!(status, 408 | 409 | 425) || (500..=599).contains(&status) {
        return ImageGenerationError::transient_provider(PROVIDER, model_id, message);
    }
    ImageGenerationError::provider(PROVIDER, model_id, message)
}

pub(super) fn classify_request_error(
    source: json_http::Error,
    model_id: &str,
) -> ImageGenerationError {
    match source {
        json_http::Error::Transport { .. } | json_http::Error::Auth { .. } => {
            ImageGenerationError::transient_provider(PROVIDER, model_id, source.to_string())
        }
        json_http::Error::SerializeRequest { .. }
        | json_http::Error::DeserializeResponse { .. } => ImageGenerationError::internal(source),
    }
}

fn error_message(body: &Value) -> Option<String> {
    body.get("error")
        .and_then(|error| error.get("message"))
        .and_then(Value::as_str)
        .or_else(|| body.get("message").and_then(Value::as_str))
        .or_else(|| body.as_str())
        .map(ToOwned::to_owned)
}
