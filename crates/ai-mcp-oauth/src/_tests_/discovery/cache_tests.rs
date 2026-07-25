//! Discovery cache expiry and challenge invalidation tests.

use serde_json::json;
use unimock::{MockFn, Unimock, matching};

use crate::{AuthorizationServerSelectorMock, McpOAuthDiscovery};

use super::support::{
    challenge, discovery, protected_json, resource, response, response_with_cache_control,
    server_json,
};

#[tokio::test]
async fn cache_expires_and_changed_challenge_url_invalidates_it() {
    let responses = vec![
        response(protected_json(), 10),
        response(server_json("https://auth.example"), 10),
        response(protected_json(), 10),
        response(server_json("https://auth.example"), 10),
        response(protected_json(), 10),
        response(server_json("https://auth.example"), 10),
    ];
    let discovery = discovery(responses, Unimock::new(()), vec![100, 105, 111]);

    discovery
        .discover(&resource(), &challenge(None))
        .await
        .unwrap();
    discovery
        .discover(&resource(), &challenge(None))
        .await
        .unwrap();
    discovery
        .discover(
            &resource(),
            &challenge(Some("https://mcp.example/metadata-v2")),
        )
        .await
        .unwrap();
}

#[tokio::test]
async fn no_store_after_max_age_forces_discovery_refetch() {
    let second_issuer = "https://auth-two.example";
    let responses = vec![
        response_with_cache_control(protected_json(), "max-age=3600, no-store"),
        response(server_json("https://auth.example"), 3600),
        response(
            json!({
                "resource": "https://mcp.example/api",
                "authorization_servers": [second_issuer]
            }),
            3600,
        ),
        response(server_json(second_issuer), 3600),
    ];
    let discovery = discovery(responses, Unimock::new(()), vec![100, 101]);

    let first = discovery
        .discover(&resource(), &challenge(None))
        .await
        .unwrap();
    let second = discovery
        .discover(&resource(), &challenge(None))
        .await
        .unwrap();

    assert_eq!(first.authorization_server.issuer, "https://auth.example");
    assert_eq!(second.authorization_server.issuer, second_issuer);
}

#[tokio::test]
async fn cached_discovery_retains_one_multi_issuer_selection() {
    let selected_issuer = "https://two.example";
    let selector = Unimock::new(
        AuthorizationServerSelectorMock::select
            .next_call(matching!(_, _))
            .returns(Ok(selected_issuer.to_owned())),
    );
    let discovery = discovery(
        vec![
            response(
                json!({
                    "resource": "https://mcp.example/api",
                    "authorization_servers": [
                        "https://one.example",
                        selected_issuer
                    ]
                }),
                60,
            ),
            response(server_json(selected_issuer), 60),
        ],
        selector,
        vec![100, 101],
    );

    let first = discovery
        .discover(&resource(), &challenge(None))
        .await
        .unwrap();
    let cached = discovery
        .discover(&resource(), &challenge(None))
        .await
        .unwrap();

    assert_eq!(first.authorization_server.issuer, selected_issuer);
    assert_eq!(cached.authorization_server.issuer, selected_issuer);
}
