//! Credential redaction for TypeSafe provider diagnostics.

use std::collections::BTreeMap;

use json_http::DynJsonHttpAuth;

const REDACTED: &str = "[redacted]";

/// Replaces every non-empty secret occurrence in one diagnostic message.
pub(super) fn redact_secrets(message: &str, secrets: &[String]) -> String {
    secrets
        .iter()
        .filter(|secret| !secret.is_empty())
        .fold(message.to_owned(), |message, secret| {
            message.replace(secret, REDACTED)
        })
}

/// Collects header values applied by an auth hook for diagnostic redaction.
pub(super) async fn applied_header_values(auth: &DynJsonHttpAuth) -> Vec<String> {
    let mut headers = BTreeMap::new();
    if auth.apply_headers(&mut headers).await.is_err() {
        return Vec::new();
    }
    headers.into_values().collect()
}
