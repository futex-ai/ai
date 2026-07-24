//! Successful authorization URL, token exchange, and persistence test.

use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
};

use ai_mcp::McpAuthorizationFailure;
use secrecy::ExposeSecret;
use serde_json::json;
use unimock::{MockFn, Unimock, matching};
use url::Url;

use crate::{
    McpOAuthConfig, McpOAuthDiscoveryMock, McpOAuthManager, OAuthAuthorizationResponse,
    OAuthClientRegistryMock, OAuthCredentialStoreMock, OAuthHttpResponse, OAuthHttpTransportMock,
    OAuthRandomMock, OAuthTokenSet, OAuthUserAgentMock, OAuthUserAuthorizationRequest,
};

use super::support::{challenge, clock, context, discovery_result, manager, registration};

#[tokio::test]
async fn authorization_uses_pkce_resource_minimum_scopes_and_atomic_storage() {
    let authorization_url = Arc::new(Mutex::new(None::<String>));
    let token_form = Arc::new(Mutex::new(None::<Vec<(String, String)>>));
    let saved = Arc::new(Mutex::new(None::<OAuthTokenSet>));
    let discovery = Unimock::new(
        McpOAuthDiscoveryMock::discover
            .next_call(matching!(_, _))
            .returns(Ok(discovery_result())),
    );
    let registry = Unimock::new(
        OAuthClientRegistryMock::resolve
            .next_call(matching!(_))
            .returns(Ok(registration())),
    );
    let user_agent = Unimock::new(
        OAuthUserAgentMock::authorize
            .next_call(matching!(_))
            .answers_arc({
                let authorization_url = authorization_url.clone();
                Arc::new(move |_, request: OAuthUserAuthorizationRequest| {
                    let url = Url::parse(request.authorization_url()).unwrap();
                    let state = query(&url)["state"].clone();
                    *authorization_url.lock().unwrap() = Some(url.to_string());
                    Ok(OAuthAuthorizationResponse::authorized(
                        "one-time-code",
                        Some(state),
                    ))
                })
            }),
    );
    let transport = Unimock::new(
        OAuthHttpTransportMock::post_form
            .next_call(matching!(_, _, _, _, _))
            .answers_arc({
                let token_form = token_form.clone();
                Arc::new(move |_, _, _, _, _, fields: &[(String, String)]| {
                    *token_form.lock().unwrap() = Some(fields.to_vec());
                    Ok(OAuthHttpResponse {
                        status: 200,
                        headers: BTreeMap::new(),
                        body: json!({
                            "access_token": "access-secret",
                            "refresh_token": "refresh-secret",
                            "token_type": "Bearer"
                        }),
                    })
                })
            }),
    );
    let store = Unimock::new(
        OAuthCredentialStoreMock::save_tokens
            .next_call(matching!(_, _))
            .answers_arc({
                let saved = saved.clone();
                Arc::new(move |_, _, tokens: &OAuthTokenSet| {
                    *saved.lock().unwrap() = Some(tokens.clone());
                    Ok(())
                })
            }),
    );
    let random = Unimock::new((
        OAuthRandomMock::bytes
            .next_call(matching!(32))
            .returns(Ok(vec![1; 32])),
        OAuthRandomMock::bytes
            .next_call(matching!(32))
            .returns(Ok(vec![2; 32])),
    ));
    let oauth = manager(
        discovery,
        registry,
        store,
        user_agent,
        transport,
        clock(vec![100, 101, 102]),
        random,
        McpOAuthConfig::default(),
    );

    let connection = oauth
        .authorize(
            &challenge(
                McpAuthorizationFailure::AuthorizationRequired,
                &["write", "read"],
            ),
            &context(),
        )
        .await
        .unwrap();

    let url = Url::parse(authorization_url.lock().unwrap().as_ref().unwrap()).unwrap();
    let query = query(&url);
    assert_eq!(query["response_type"], "code");
    assert_eq!(query["client_id"], "client-id");
    assert_eq!(query["redirect_uri"], "https://app.example/callback");
    assert_eq!(query["resource"], "https://mcp.example/api");
    assert_eq!(query["scope"], "read write");
    assert_eq!(query["code_challenge_method"], "S256");
    assert_eq!(query["state"].len(), 43);
    assert_eq!(query["code_challenge"].len(), 43);

    let form = token_form.lock().unwrap().clone().unwrap();
    assert_eq!(field(&form, "grant_type"), "authorization_code");
    assert_eq!(field(&form, "code"), "one-time-code");
    assert_eq!(field(&form, "client_id"), "client-id");
    assert_eq!(field(&form, "redirect_uri"), "https://app.example/callback");
    assert_eq!(field(&form, "resource"), "https://mcp.example/api");
    assert_eq!(field(&form, "code_verifier").len(), 43);

    let saved = saved.lock().unwrap().clone().unwrap();
    assert_eq!(saved.access_token.expose_secret(), "access-secret");
    assert_eq!(
        saved.refresh_token.unwrap().expose_secret(),
        "refresh-secret"
    );
    assert_eq!(saved.expires_at, None);
    assert_eq!(saved.scopes.as_slice(), &["read", "write"]);
    assert_eq!(connection.scopes.as_slice(), &["read", "write"]);
}

fn query(url: &Url) -> BTreeMap<String, String> {
    url.query_pairs()
        .map(|(name, value)| (name.into_owned(), value.into_owned()))
        .collect()
}

fn field<'a>(fields: &'a [(String, String)], name: &str) -> &'a str {
    fields
        .iter()
        .find(|(candidate, _)| candidate == name)
        .map(|(_, value)| value.as_str())
        .unwrap()
}
