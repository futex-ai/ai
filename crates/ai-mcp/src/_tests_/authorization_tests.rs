//! Bearer challenge parsing tests.

use crate::{McpAuthorizationFailure, authorization::authorization_challenge};

#[test]
fn combines_repeated_bearer_fields_without_splitting_quoted_commas() {
    let fields = vec![
        "Basic realm=\"legacy\", Bearer error=\"invalid_token\", error_description=\"expired, retry\", scope=\"read write\"".to_owned(),
        "Bearer scope=\"write admin\", resource_metadata=\"https://example.com/.well-known/oauth-protected-resource/mcp\"".to_owned(),
    ];

    let challenge = authorization_challenge(401, &fields);

    assert_eq!(challenge.failure, McpAuthorizationFailure::InvalidToken);
    assert_eq!(
        challenge.error_description.as_deref(),
        Some("expired, retry")
    );
    assert_eq!(challenge.scopes, ["read", "write", "admin"]);
    assert_eq!(
        challenge.resource_metadata_url.as_deref(),
        Some("https://example.com/.well-known/oauth-protected-resource/mcp")
    );
    assert_eq!(challenge.raw_www_authenticate, fields);
}

#[test]
fn conflicting_or_malformed_resource_metadata_is_ignored() {
    let conflicting = authorization_challenge(
        403,
        &[
            "Bearer error=\"insufficient_scope\", resource_metadata=\"https://a.example/meta\""
                .to_owned(),
            "Bearer resource_metadata=\"https://b.example/meta\"".to_owned(),
        ],
    );
    let malformed =
        authorization_challenge(401, &["Bearer resource_metadata=\"not a url\"".to_owned()]);

    assert_eq!(
        conflicting.failure,
        McpAuthorizationFailure::InsufficientScope
    );
    assert_eq!(conflicting.resource_metadata_url, None);
    assert_eq!(malformed.resource_metadata_url, None);
}

#[test]
fn defaults_failure_from_status_when_bearer_error_is_absent() {
    assert_eq!(
        authorization_challenge(401, &[]).failure,
        McpAuthorizationFailure::AuthorizationRequired
    );
    assert_eq!(
        authorization_challenge(403, &[]).failure,
        McpAuthorizationFailure::Forbidden
    );
}
