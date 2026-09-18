//! Boundary-neutral HTTP status classification tests.

use super::{HttpFailureClass, classify_http_status};

#[test]
fn statuses_below_failure_range_are_not_failures() {
    for status in [200, 204, 399] {
        assert_eq!(classify_http_status(status), None, "HTTP {status}");
    }
}

#[test]
fn rate_limit_status_has_its_own_class() {
    assert_eq!(
        classify_http_status(429),
        Some(HttpFailureClass::RateLimited)
    );
}

#[test]
fn retryable_statuses_are_transient() {
    for status in [408, 409, 425, 500, 502, 503, 529, 599] {
        assert_eq!(
            classify_http_status(status),
            Some(HttpFailureClass::Transient),
            "HTTP {status}"
        );
    }
}

#[test]
fn remaining_failure_statuses_are_terminal() {
    for status in [400, 401, 403, 404, 422, 600] {
        assert_eq!(
            classify_http_status(status),
            Some(HttpFailureClass::Terminal),
            "HTTP {status}"
        );
    }
}
