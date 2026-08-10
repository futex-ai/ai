//! Authorization URL extension-parameter retention tests.

use std::collections::BTreeMap;

use url::Url;

use crate::{McpOAuthConfig, OAuthScopes};

use super::{build_authorization_url, validated_authorization_endpoint};

#[test]
fn preserves_non_reserved_and_case_distinct_endpoint_parameters() {
    let endpoint = validated_authorization_endpoint(
        "https://auth.example/authorize?audience=tools&CLIENT_ID=extension",
        &McpOAuthConfig::default(),
    )
    .unwrap();

    let authorization_url = build_authorization_url(
        endpoint,
        "client-id",
        "https://app.example/callback",
        "https://tools.example/mcp",
        &OAuthScopes::new(["read", "write"]),
        "state",
        "challenge",
    );

    let query = Url::parse(&authorization_url)
        .unwrap()
        .query_pairs()
        .map(|(name, value)| (name.into_owned(), value.into_owned()))
        .collect::<BTreeMap<_, _>>();
    assert_eq!(query["audience"], "tools");
    assert_eq!(query["CLIENT_ID"], "extension");
    assert_eq!(query["response_type"], "code");
    assert_eq!(query["client_id"], "client-id");
    assert_eq!(query["redirect_uri"], "https://app.example/callback");
    assert_eq!(query["code_challenge"], "challenge");
    assert_eq!(query["code_challenge_method"], "S256");
    assert_eq!(query["resource"], "https://tools.example/mcp");
    assert_eq!(query["state"], "state");
    assert_eq!(query["scope"], "read write");
}
