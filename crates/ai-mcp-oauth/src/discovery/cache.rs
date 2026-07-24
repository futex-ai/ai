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
    for directive in values.iter().flat_map(|value| value.split(',')) {
        let directive = directive.trim();
        if directive.eq_ignore_ascii_case("no-store") {
            return Some(0);
        }
        if let Some((name, value)) = directive.split_once('=')
            && name.trim().eq_ignore_ascii_case("max-age")
            && let Ok(seconds) = value.trim().trim_matches('"').parse()
        {
            return Some(seconds);
        }
    }
    None
}
