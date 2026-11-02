//! Case-insensitive request header precedence and credential names.

use std::collections::BTreeMap;

pub(crate) fn insert(headers: &mut BTreeMap<String, String>, name: String, value: String) {
    headers.retain(|key, _| !key.eq_ignore_ascii_case(&name));
    headers.insert(name, value);
}

pub(crate) fn credential_header(name: &str) -> bool {
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

pub(crate) fn same_origin(left: &str, right: &str) -> bool {
    match (reqwest::Url::parse(left), reqwest::Url::parse(right)) {
        (Ok(left), Ok(right)) => left.origin() == right.origin(),
        _ => false,
    }
}
