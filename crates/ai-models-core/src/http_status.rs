//! Boundary-neutral HTTP failure status classification.

/// Retry class of one failing HTTP status.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HttpFailureClass {
    /// The provider rejected the request because its rate limit was reached.
    RateLimited,
    /// The provider failure can be retried.
    Transient,
    /// The provider failure should not be retried.
    Terminal,
}

/// Classifies one HTTP status for retry handling.
///
/// Returns `None` below 400. Status 429 is rate limited; 408, 409, 425, and
/// every status from 500 through 599 are transient; every other failure
/// status is terminal.
pub fn classify_http_status(status: u16) -> Option<HttpFailureClass> {
    if status < 400 {
        return None;
    }

    match status {
        429 => Some(HttpFailureClass::RateLimited),
        408 | 409 | 425 | 500..=599 => Some(HttpFailureClass::Transient),
        _ => Some(HttpFailureClass::Terminal),
    }
}

#[cfg(test)]
#[path = "_tests_/http_status_tests.rs"]
mod http_status_tests;
