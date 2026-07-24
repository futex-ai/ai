//! Discovery cache expiry and challenge invalidation tests.

use unimock::Unimock;

use crate::McpOAuthDiscovery;

use super::support::{challenge, discovery, protected_json, resource, response, server_json};

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
