//! Token freshness and redaction tests.

use secrecy::SecretString;

use crate::{OAuthScopes, OAuthTokenSet, OAuthTokenType};

#[test]
fn applies_default_and_configurable_refresh_skew() {
    let value = token(Some(200));

    assert!(value.is_fresh_at(139, 60));
    assert!(!value.is_fresh_at(140, 60));
    assert!(value.is_fresh_at(180, 10));
    assert!(token(Some(200)).is_expired_at(200));
}

#[test]
fn absent_expiry_remains_fresh_without_inventing_one() {
    let token = token(None);

    assert!(token.is_fresh_at(u64::MAX, 60));
    assert!(!token.is_expired_at(u64::MAX));
}

#[test]
fn token_debug_output_never_contains_secrets() {
    let token = token(Some(200));
    let rendered = format!("{token:?}");

    assert!(!rendered.contains("access-secret"));
    assert!(!rendered.contains("refresh-secret"));
    assert!(rendered.contains("[REDACTED]"));
}

fn token(expires_at: Option<u64>) -> OAuthTokenSet {
    OAuthTokenSet {
        access_token: SecretString::from("access-secret".to_owned()),
        refresh_token: Some(SecretString::from("refresh-secret".to_owned())),
        token_type: OAuthTokenType::Bearer,
        expires_at,
        scopes: OAuthScopes::new(["read"]),
    }
}
