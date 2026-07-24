//! Discovery metadata and issuer validation tests.

use serde_json::json;
use unimock::{MockFn, Unimock, matching};

use crate::{AuthorizationServerSelectorMock, Error, McpOAuthDiscovery};

use super::support::{challenge, discovery, protected_json, resource, response, server_json};

#[tokio::test]
async fn discovers_one_issuer_and_preserves_unknown_metadata() {
    let discovery = discovery(
        vec![
            response(
                json!({
                    "resource": "https://mcp.example/api",
                    "authorization_servers": ["https://auth.example"],
                    "scopes_supported": ["read"],
                    "future_resource_field": {"enabled": true}
                }),
                60,
            ),
            response(
                json!({
                    "issuer": "https://auth.example",
                    "authorization_endpoint": "https://auth.example/authorize",
                    "token_endpoint": "https://auth.example/token",
                    "registration_endpoint": "https://auth.example/register",
                    "grant_types_supported": ["authorization_code", "refresh_token"],
                    "token_endpoint_auth_methods_supported": ["none"],
                    "code_challenge_methods_supported": ["S256"],
                    "future_server_field": 7
                }),
                30,
            ),
        ],
        Unimock::new(()),
        vec![100],
    );
    let result = discovery
        .discover(&resource(), &challenge(None))
        .await
        .unwrap();

    assert_eq!(
        result.resource_metadata_url,
        "https://mcp.example/.well-known/oauth-protected-resource/api"
    );
    assert_eq!(
        result.protected_resource.unknown["future_resource_field"],
        json!({"enabled": true})
    );
    assert_eq!(
        result.authorization_server.unknown["future_server_field"],
        json!(7)
    );
}

#[tokio::test]
async fn multiple_issuers_require_and_validate_host_selection() {
    let selector = Unimock::new(
        AuthorizationServerSelectorMock::select
            .next_call(matching!(_, _))
            .returns(Ok("https://two.example".to_owned())),
    );
    let discovery = discovery(
        vec![
            response(
                json!({
                    "resource": "https://mcp.example/api",
                    "authorization_servers": [
                        "https://one.example",
                        "https://two.example"
                    ]
                }),
                60,
            ),
            response(server_json("https://two.example"), 60),
        ],
        selector,
        vec![100],
    );

    let result = discovery
        .discover(&resource(), &challenge(None))
        .await
        .unwrap();

    assert_eq!(result.authorization_server.issuer, "https://two.example");
}

#[tokio::test]
async fn rejects_resource_and_issuer_mismatches() {
    let resource_mismatch = discovery(
        vec![response(
            json!({
                "resource": "https://other.example/api",
                "authorization_servers": ["https://auth.example"]
            }),
            60,
        )],
        Unimock::new(()),
        vec![100],
    )
    .discover(&resource(), &challenge(None))
    .await
    .unwrap_err();
    assert!(matches!(resource_mismatch, Error::ResourceMismatch { .. }));

    let issuer_mismatch = discovery(
        vec![
            response(
                json!({
                    "resource": "https://mcp.example/api",
                    "authorization_servers": ["https://auth.example"]
                }),
                60,
            ),
            response(server_json("https://other.example"), 60),
        ],
        Unimock::new(()),
        vec![100],
    )
    .discover(&resource(), &challenge(None))
    .await
    .unwrap_err();
    assert!(matches!(issuer_mismatch, Error::IssuerMismatch { .. }));
}

#[tokio::test]
async fn rejects_missing_and_unsafe_required_endpoints() {
    let missing = discovery(
        vec![
            response(protected_json(), 60),
            response(
                json!({
                    "issuer": "https://auth.example",
                    "authorization_endpoint": "https://auth.example/authorize"
                }),
                60,
            ),
        ],
        Unimock::new(()),
        vec![100],
    )
    .discover(&resource(), &challenge(None))
    .await
    .unwrap_err();
    assert!(matches!(missing, Error::MissingEndpoint { .. }));

    let unsafe_endpoint = discovery(
        vec![
            response(protected_json(), 60),
            response(
                json!({
                    "issuer": "https://auth.example",
                    "authorization_endpoint": "http://auth.example/authorize",
                    "token_endpoint": "https://auth.example/token"
                }),
                60,
            ),
        ],
        Unimock::new(()),
        vec![100],
    )
    .discover(&resource(), &challenge(None))
    .await
    .unwrap_err();
    assert!(matches!(unsafe_endpoint, Error::UnsafeUrl { .. }));
}
