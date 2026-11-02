//! Precise shared failure categories for unsuccessful provider HTTP statuses.

use ai_interface::ErrorKind;

/// Returns the precise failure category, or None below HTTP 400.
pub fn classify_http_status(status: u16) -> Option<ErrorKind> {
    match status {
        0..=399 => None,
        401..=403 => Some(ErrorKind::RejectedKey),
        404 => Some(ErrorKind::UnknownModel),
        429 => Some(ErrorKind::RateLimited),
        408 | 409 | 425 | 500..=599 => Some(ErrorKind::Transient),
        _ => Some(ErrorKind::InvalidRequest),
    }
}

#[cfg(test)]
#[path = "_tests_/http_status_tests.rs"]
mod http_status_tests;
