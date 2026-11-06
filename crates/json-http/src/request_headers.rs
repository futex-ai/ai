//! Case-insensitive request header precedence and credential names.

use std::collections::BTreeMap;

pub(crate) fn insert(headers: &mut BTreeMap<String, String>, name: String, value: String) {
    headers.retain(|key, _| !key.eq_ignore_ascii_case(&name));
    headers.insert(name, value);
}

/// Identifies standard credential headers for dispatch, redirects, capture, and recordings.
pub fn credential_header(name: &str) -> bool {
    matches!(
        name.to_ascii_lowercase().as_str(),
        "authorization"
            | "cookie"
            | "cookie2"
            | "proxy-authorization"
            | "www-authenticate"
            | "x-api-key"
            | "x-goog-api-key"
            | "api-key"
    )
}

/// Derives full credential values and long component words without hiding short common words.
pub fn credential_values(value: &str) -> impl Iterator<Item = &str> {
    std::iter::once(value)
        .filter(|value| !value.is_empty())
        .chain(
            value
                .split_whitespace()
                .next_back()
                .filter(move |word| word.len() >= 6 && *word != value),
        )
}

pub(crate) fn same_origin(left: &str, right: &str) -> bool {
    match (reqwest::Url::parse(left), reqwest::Url::parse(right)) {
        (Ok(left), Ok(right)) => left.origin() == right.origin(),
        _ => false,
    }
}
