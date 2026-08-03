//! Response-header normalization regressions.

use reqwest::header::{CACHE_CONTROL, CONTENT_TYPE, HeaderMap, HeaderName, HeaderValue};

use super::normalized_headers;

#[test]
fn undecodable_repeated_values_poison_the_whole_header() {
    for (name, valid, undecodable) in [
        (
            CACHE_CONTROL,
            HeaderValue::from_static("max-age=60"),
            HeaderValue::from_bytes(b"no-store\xff").unwrap(),
        ),
        (
            CONTENT_TYPE,
            HeaderValue::from_static("application/json"),
            HeaderValue::from_bytes(b"application/json\xff").unwrap(),
        ),
    ] {
        let headers = repeated_headers(name.clone(), valid, undecodable);

        assert_eq!(
            normalized_headers(&headers).get(name.as_str()),
            Some(&Vec::new())
        );
    }
}

#[test]
fn valid_repeated_values_retain_wire_order() {
    let headers = repeated_headers(
        CACHE_CONTROL,
        HeaderValue::from_static("max-age=60"),
        HeaderValue::from_static("no-cache"),
    );

    assert_eq!(
        normalized_headers(&headers).get("cache-control"),
        Some(&vec!["max-age=60".to_owned(), "no-cache".to_owned()])
    );
}

fn repeated_headers(name: HeaderName, first: HeaderValue, second: HeaderValue) -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.append(name.clone(), first);
    headers.append(name, second);
    headers
}
