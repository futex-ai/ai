//! Google video generation HTTP error classification.

use ai_interface::VideoGenerationError;
use serde_json::Value;

const PROVIDER: &str = "google";

pub(super) fn classify_status(status: u16, model_id: &str, body: &Value) -> VideoGenerationError {
    let message = body
        .get("error")
        .and_then(|error| error.get("message"))
        .and_then(Value::as_str)
        .or_else(|| body.as_str())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| format!("HTTP {status}"));
    if status == 429 {
        return VideoGenerationError::rate_limited(PROVIDER, model_id, message);
    }
    if matches!(status, 408 | 409 | 425) || (500..=599).contains(&status) {
        return VideoGenerationError::transient_provider(PROVIDER, model_id, message);
    }
    VideoGenerationError::provider(PROVIDER, model_id, message)
}

pub(super) fn classify_request_error(
    source: json_http::Error,
    model_id: &str,
) -> VideoGenerationError {
    match source {
        json_http::Error::Transport { .. }
        | json_http::Error::ReqwestTransport { .. }
        | json_http::Error::Auth { .. } => {
            VideoGenerationError::transient_provider(PROVIDER, model_id, source.to_string())
        }
        json_http::Error::SerializeRequest { .. }
        | json_http::Error::DeserializeResponse { .. }
        | json_http::Error::ClientInitialization { .. }
        | json_http::Error::SseUnsupported
        | json_http::Error::HttpStatus { .. }
        | json_http::Error::InvalidSseContentType { .. }
        | json_http::Error::IdleTimeout { .. }
        | json_http::Error::DeadlineExceeded { .. }
        | json_http::Error::SseTransport { .. }
        | json_http::Error::SseDecode { .. } => VideoGenerationError::internal(source),
    }
}
