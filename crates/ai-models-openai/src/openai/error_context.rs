//! OpenAI-owned response header names and native provider error code meanings.

use std::sync::Arc;

use ai_interface::ErrorKind;
use ai_models_core::{Clock, ErrorContext};

pub(super) fn new(model_id: &str, clock: Arc<dyn Clock>) -> ErrorContext {
    ErrorContext::new("openai", model_id, &["x-request-id", "request-id"], clock)
}

pub(super) fn native_kind(code: Option<&str>) -> Option<ErrorKind> {
    match code {
        Some("rate_limit_error" | "rate_limit_exceeded" | "slow_down" | "too_many_requests") => {
            Some(ErrorKind::RateLimited)
        }
        Some(
            "internal_error"
            | "server_error"
            | "overloaded"
            | "temporarily_unavailable"
            | "service_unavailable_error"
            | "server_is_overloaded"
            | "timeout_error"
            | "vector_store_timeout",
        ) => Some(ErrorKind::Transient),
        Some(
            "authentication_error"
            | "invalid_api_key"
            | "permission_error"
            | "permission_denied"
            | "insufficient_quota"
            | "billing_error"
            | "billing_not_active"
            | "billing_hard_limit_reached"
            | "account_deactivated"
            | "organization_spend_limit_exceeded"
            | "project_spend_limit_exceeded"
            | "organization_usage_limit_exceeded",
        ) => Some(ErrorKind::RejectedKey),
        Some("model_not_found" | "unknown_model") => Some(ErrorKind::UnknownModel),
        Some(
            "context_length_exceeded"
            | "model_context_window_exceeded"
            | "input_too_long"
            | "too_many_tokens",
        ) => Some(ErrorKind::ContextLimitExceeded),
        Some("unsupported_image_media_type") => Some(ErrorKind::UnsupportedInput),
        Some(
            "invalid_request_error"
            | "invalid_prompt"
            | "invalid_parameter"
            | "invalid_image"
            | "invalid_image_format"
            | "invalid_base64_image"
            | "invalid_image_url"
            | "image_too_large"
            | "image_too_small"
            | "image_parse_error"
            | "invalid_image_mode"
            | "image_file_too_large"
            | "empty_image_file"
            | "failed_to_download_image"
            | "image_file_not_found"
            | "data_residency_mismatch",
        ) => Some(ErrorKind::InvalidRequest),
        _ => None,
    }
}
