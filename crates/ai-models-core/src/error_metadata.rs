//! Optional provider response metadata independent of a modality.

use std::time::{Duration, SystemTime};

use ai_interface::ErrorCore;
use json_http::JsonHttpHeaders;

use crate::Clock;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
/// Retry hints and request identity captured from one actual response.
pub struct ErrorMetadata {
    /// Wait requested by the response's Retry-After header.
    pub retry_after: Option<Duration>,
    /// Request id selected from adapter-owned header names.
    pub provider_request_id: Option<String>,
}

/// Parses a delta-seconds or HTTP-date Retry-After value against explicit time.
pub fn parse_retry_after(value: &str, now: SystemTime) -> Option<Duration> {
    let value = value.trim();
    if value.bytes().all(|byte| byte.is_ascii_digit()) && !value.is_empty() {
        return value.parse::<u64>().ok().map(Duration::from_secs);
    }
    httpdate::parse_http_date(value)
        .ok()
        .map(|date| date.duration_since(now).unwrap_or(Duration::ZERO))
}

impl ErrorMetadata {
    /// Captures optional metadata, consulting the clock only for a valid date.
    pub fn from_headers(
        headers: &JsonHttpHeaders,
        request_id_headers: &[&str],
        clock: &dyn Clock,
    ) -> Self {
        let retry_after = headers
            .get_all("retry-after")
            .iter()
            .filter_map(|value| value.to_str().ok())
            .find_map(|value| {
                let value = value.trim();
                if !value.is_empty() && value.bytes().all(|byte| byte.is_ascii_digit()) {
                    return value.parse::<u64>().ok().map(Duration::from_secs);
                }
                httpdate::parse_http_date(value)
                    .ok()
                    .map(|date| date.duration_since(clock.now()).unwrap_or(Duration::ZERO))
            });
        let provider_request_id = request_id_headers.iter().find_map(|name| {
            headers
                .get_all(*name)
                .iter()
                .filter_map(|value| value.to_str().ok())
                .map(str::trim)
                .find(|value| !value.is_empty())
                .map(ToOwned::to_owned)
        });
        Self {
            retry_after,
            provider_request_id,
        }
    }

    /// Applies this response's metadata without changing classification or identity.
    pub fn apply(&self, core: &mut ErrorCore) {
        core.retry_after = self.retry_after;
        core.provider_request_id = self.provider_request_id.clone();
    }
}
