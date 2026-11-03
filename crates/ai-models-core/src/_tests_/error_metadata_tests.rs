//! Retry-After parsing and response metadata requirements.

use std::time::{Duration, SystemTime};

use json_http::JsonHttpHeaders;
use unimock::{MockFn, Unimock, matching};

use crate::{ClockMock, ErrorMetadata, parse_retry_after};

#[test]
fn retry_after_handles_seconds_dates_and_invalid_optional_values() {
    let now = SystemTime::UNIX_EPOCH + Duration::from_secs(5);
    for (value, expected) in [
        ("0", Some(Duration::ZERO)),
        (" 7 ", Some(Duration::from_secs(7))),
        ("18446744073709551615", Some(Duration::from_secs(u64::MAX))),
        ("18446744073709551616", None),
        (
            "Thu, 01 Jan 1970 00:00:10 GMT",
            Some(Duration::from_secs(5)),
        ),
        ("Thu, 01 Jan 1970 00:00:05 GMT", Some(Duration::ZERO)),
        ("Thu, 01 Jan 1970 00:00:01 GMT", Some(Duration::ZERO)),
        ("-1", None),
        ("1.5", None),
        ("", None),
        ("invalid", None),
    ] {
        assert_eq!(parse_retry_after(value, now), expected, "{value}");
    }
}

#[test]
fn metadata_uses_adapter_header_order_and_reads_clock_only_for_valid_dates() {
    let mut headers = JsonHttpHeaders::new();
    headers
        .as_raw_mut()
        .insert("retry-after", "7".parse().unwrap());
    headers
        .as_raw_mut()
        .insert("primary-id", "first".parse().unwrap());
    headers
        .as_raw_mut()
        .insert("fallback-id", "second".parse().unwrap());
    let metadata =
        ErrorMetadata::from_headers(&headers, &["Primary-Id", "fallback-id"], &Unimock::new(()));
    assert_eq!(metadata.retry_after, Some(Duration::from_secs(7)));
    assert_eq!(metadata.provider_request_id.as_deref(), Some("first"));
    headers
        .as_raw_mut()
        .insert("primary-id", "  ".parse().unwrap());
    headers.as_raw_mut().insert(
        "retry-after",
        "Thu, 01 Jan 1970 00:00:10 GMT".parse().unwrap(),
    );
    let clock = Unimock::new(
        ClockMock::now
            .some_call(matching!())
            .returns(SystemTime::UNIX_EPOCH + Duration::from_secs(5)),
    );
    let metadata = ErrorMetadata::from_headers(&headers, &["primary-id", "fallback-id"], &clock);
    assert_eq!(metadata.retry_after, Some(Duration::from_secs(5)));
    assert_eq!(metadata.provider_request_id.as_deref(), Some("second"));
    headers
        .as_raw_mut()
        .insert("retry-after", "invalid".parse().unwrap());
    assert_eq!(
        ErrorMetadata::from_headers(&headers, &[], &Unimock::new(())).retry_after,
        None
    );
}
