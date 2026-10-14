//! Credential redaction for provider diagnostics.

use std::collections::BTreeMap;

const REDACTED: &str = "[redacted]";

/// Replaces every non-empty secret occurrence in one diagnostic message.
///
/// Secrets are applied longest first so a shorter secret that is a substring
/// of a longer one cannot leave the longer credential's remainder exposed.
pub fn redact_secrets(message: &str, secrets: &[String]) -> String {
    let mut ordered = secrets
        .iter()
        .filter(|secret| !secret.is_empty())
        .collect::<Vec<_>>();
    ordered.sort_by(|left, right| right.len().cmp(&left.len()).then_with(|| left.cmp(right)));
    ordered.dedup();
    ordered
        .into_iter()
        .fold(message.to_owned(), |message, secret| {
            message.replace(secret.as_str(), REDACTED)
        })
}

/// Collects explicit and transmitted header secrets for diagnostic redaction.
///
/// Every non-empty header value is a secret. For scheme-prefixed values such
/// as `Bearer <token>`, the token after the last whitespace is also a secret
/// so a provider body that echoes the bare token is still redacted.
pub fn secrets_from_headers(
    explicit: &[String],
    headers: &BTreeMap<String, String>,
) -> Vec<String> {
    let mut secrets = explicit
        .iter()
        .filter(|secret| !secret.is_empty())
        .cloned()
        .collect::<Vec<_>>();
    for value in headers.values().filter(|value| !value.is_empty()) {
        secrets.push(value.clone());
        if let Some((_, suffix)) = value.rsplit_once(char::is_whitespace)
            && !suffix.is_empty()
        {
            secrets.push(suffix.to_owned());
        }
    }
    secrets
}

#[cfg(test)]
#[path = "_tests_/redaction_tests.rs"]
mod redaction_tests;
