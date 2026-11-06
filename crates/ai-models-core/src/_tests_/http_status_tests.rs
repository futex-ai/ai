//! Boundary-neutral HTTP status classification tests.

use ai_interface::ErrorKind;

use super::classify_http_status;

#[test]
fn statuses_below_failure_range_are_not_failures() {
    for status in [200, 204, 399] {
        assert_eq!(classify_http_status(status), None, "HTTP {status}");
    }
}

#[test]
fn rate_limit_status_has_its_own_class() {
    assert_eq!(classify_http_status(429), Some(ErrorKind::RateLimited));
}

#[test]
fn retryable_statuses_are_transient() {
    for status in [408, 409, 425, 500, 502, 503, 529, 599] {
        assert_eq!(
            classify_http_status(status),
            Some(ErrorKind::Transient),
            "HTTP {status}"
        );
    }
}

#[test]
fn remaining_failure_statuses_are_terminal() {
    for (status, expected) in [
        (400, ErrorKind::InvalidRequest),
        (401, ErrorKind::RejectedKey),
        (402, ErrorKind::RejectedKey),
        (403, ErrorKind::RejectedKey),
        (404, ErrorKind::UnknownModel),
        (413, ErrorKind::InvalidRequest),
        (422, ErrorKind::InvalidRequest),
        (499, ErrorKind::InvalidRequest),
        (600, ErrorKind::InvalidRequest),
    ] {
        assert_eq!(
            classify_http_status(status),
            Some(expected),
            "HTTP {status}"
        );
    }
}
