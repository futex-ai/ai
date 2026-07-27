//! HTTP response media-type classification.

use std::collections::BTreeMap;

pub(crate) const APPLICATION_JSON: &str = "application/json";
pub(crate) const EVENT_STREAM: &str = "text/event-stream";

pub(crate) fn first(headers: &BTreeMap<String, Vec<String>>) -> Option<&str> {
    headers
        .get("content-type")
        .and_then(|values| values.first())
        .map(String::as_str)
}

pub(crate) fn matches(headers: &BTreeMap<String, Vec<String>>, expected: &str) -> bool {
    first(headers)
        .and_then(|value| value.split(';').next())
        .is_some_and(|essence| essence.trim().eq_ignore_ascii_case(expected))
}
