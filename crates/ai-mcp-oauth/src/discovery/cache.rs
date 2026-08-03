//! HTTP cache lifetime parsing for combined discovery documents.

use std::collections::BTreeMap;

pub(super) fn cache_age_seconds(
    protected_headers: &BTreeMap<String, Vec<String>>,
    server_headers: &BTreeMap<String, Vec<String>>,
    maximum: u64,
) -> u64 {
    maximum
        .min(response_cache_age(protected_headers).unwrap_or(maximum))
        .min(response_cache_age(server_headers).unwrap_or(maximum))
}

fn response_cache_age(headers: &BTreeMap<String, Vec<String>>) -> Option<u64> {
    let values = headers.get("cache-control")?;
    if values.is_empty() {
        return Some(0);
    }
    let mut minimum = None;
    let mut force_zero = false;
    for directive in values.iter().flat_map(|value| value.split(',')) {
        let directive = directive.trim();
        let (name, value) = match directive.split_once('=') {
            Some((name, value)) => (name.trim(), Some(value.trim())),
            None => (directive, None),
        };
        if name.eq_ignore_ascii_case("no-store") || name.eq_ignore_ascii_case("no-cache") {
            force_zero = true;
            continue;
        }
        if name.eq_ignore_ascii_case("max-age") {
            let Some(seconds) = value.and_then(parse_delta_seconds) else {
                force_zero = true;
                continue;
            };
            minimum = Some(minimum.map_or(seconds, |current: u64| current.min(seconds)));
        }
    }
    if force_zero { Some(0) } else { minimum }
}

fn parse_delta_seconds(value: &str) -> Option<u64> {
    let value = if value.starts_with('"') && value.ends_with('"') && value.len() >= 2 {
        &value[1..value.len() - 1]
    } else if value.contains('"') {
        return None;
    } else {
        value
    };
    if value.is_empty() || !value.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    value.parse().ok()
}

#[cfg(test)]
#[path = "_tests_/cache_tests.rs"]
mod cache_tests;
