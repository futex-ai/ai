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

#[test]
fn accepts_bad_whitespace_around_bearer_parameter_equals() {
    let metadata_url = "https://example.com/.well-known/oauth-protected-resource";
    let challenge = authorization_challenge(
        401,
        &[format!(
            "Bearer error = \"invalid_token\", resource_metadata = \"{metadata_url}\", scope = \"read write\", error_description = \"expired, retry\""
        )],
    );

    assert_eq!(challenge.failure, McpAuthorizationFailure::InvalidToken);
    assert_eq!(
        challenge.resource_metadata_url.as_deref(),
        Some(metadata_url)
    );
    assert_eq!(challenge.scopes, ["read", "write"]);
    assert_eq!(
        challenge.error_description.as_deref(),
        Some("expired, retry")
    );
}

#[test]
fn whitespace_parameter_does_not_end_the_bearer_challenge() {
    let metadata_url = "https://example.com/meta";
    let challenge = authorization_challenge(
        401,
        &[format!(
            "Bearer scope=\"read\", resource_metadata\t= \"{metadata_url}\", error=\"invalid_token\""
        )],
    );

    assert_eq!(challenge.failure, McpAuthorizationFailure::InvalidToken);
    assert_eq!(
        challenge.resource_metadata_url.as_deref(),
        Some(metadata_url)
    );
    assert_eq!(challenge.scopes, ["read"]);
}

#[test]
fn empty_list_elements_do_not_end_the_bearer_challenge() {
    let challenge = authorization_challenge(
        401,
        &[
            "Bearer error=\"invalid_token\",, scope=\"read\", , error_description=\"expired\""
                .to_owned(),
        ],
    );

    assert_eq!(challenge.failure, McpAuthorizationFailure::InvalidToken);
    assert_eq!(challenge.scopes, ["read"]);
    assert_eq!(challenge.error_description.as_deref(), Some("expired"));
}

#[test]
fn later_schemes_do_not_bleed_parameters_into_bearer() {
    let challenge = authorization_challenge(
        401,
        &[
            "Bearer scope=\"read\", Basic realm = \"legacy\", scope=\"admin\"".to_owned(),
            "Bearer error=\"invalid_token\", Negotiate YWJjZGVmZw==, scope=\"ignored\"".to_owned(),
        ],
    );

    assert_eq!(challenge.failure, McpAuthorizationFailure::InvalidToken);
    assert_eq!(challenge.scopes, ["read"]);
}
