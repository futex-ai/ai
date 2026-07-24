//! Token response type, expiry, and fallback tests.

use secrecy::{ExposeSecret, SecretString};
use serde_json::json;

use crate::{Error, OAuthHttpResponse, OAuthScopes, OAuthTokenType};

use super::parse_token_response;

#[test]
fn rejects_non_bearer_token_responses() {
    let error = parse_token_response(
        response(json!({
            "access_token": "secret",
            "token_type": "DPoP"
        })),
        500,
        &OAuthScopes::new(["read"]),
        None,
    )
    .unwrap_err();

    assert!(matches!(error, Error::TokenSchema { .. }));
    assert!(!error.to_string().contains("secret"));
}

#[test]
fn retains_requested_scopes_and_refresh_without_inventing_expiry() {
    let tokens = parse_token_response(
        response(json!({
            "access_token": "new-access",
            "token_type": "Bearer"
        })),
        500,
        &OAuthScopes::new(["read", "write"]),
        Some(SecretString::from("old-refresh".to_owned())),
    )
    .unwrap();

    assert_eq!(tokens.token_type, OAuthTokenType::Bearer);
    assert_eq!(tokens.expires_at, None);
    assert_eq!(tokens.scopes.as_slice(), &["read", "write"]);
    assert_eq!(tokens.refresh_token.unwrap().expose_secret(), "old-refresh");
}

fn response(body: serde_json::Value) -> OAuthHttpResponse {
    OAuthHttpResponse {
        status: 200,
        headers: Default::default(),
        body,
    }
}
