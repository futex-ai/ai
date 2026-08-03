//! Shared authorization/refresh concurrency fixtures.

use std::{net::IpAddr, sync::Arc};

use ai_mcp::McpAuthorizationFailure;
use serde_json::json;
use unimock::{MockFn, Unimock, matching};
use url::Url;

use crate::{
    DefaultMcpOAuthManager, McpOAuthConfig, McpOAuthDiscoveryMock, McpOAuthManager,
    OAuthAuthorizationResponse, OAuthClientRegistryMock, OAuthClockMock, OAuthConnection,
    OAuthDnsResolverMock, OAuthHttpResponse, OAuthRandomMock, OAuthTokenSet, OAuthUserAgentMock,
    OAuthUserAuthorizationRequest, Result,
};

use super::super::support::{
    challenge, context, discovery_result, key, manager, registration, server_metadata,
};

pub(super) fn concurrent_oauth(
    store: Unimock,
    transport: Unimock,
    refresh_discovers_server: bool,
) -> Arc<DefaultMcpOAuthManager> {
    let discovery = if refresh_discovers_server {
        Unimock::new((
            McpOAuthDiscoveryMock::discover
                .each_call(matching!(_, _))
                .answers(&|_, _, _| Ok(discovery_result())),
            McpOAuthDiscoveryMock::authorization_server
                .each_call(matching!(_))
                .answers(&|_, _| Ok(server_metadata())),
        ))
    } else {
        Unimock::new(
            McpOAuthDiscoveryMock::discover
                .each_call(matching!(_, _))
                .answers(&|_, _, _| Ok(discovery_result())),
        )
    };
    let registry = Unimock::new(
        OAuthClientRegistryMock::resolve
            .each_call(matching!(_))
            .answers(&|_, _| Ok(registration())),
    );
    let user_agent = Unimock::new(
        OAuthUserAgentMock::authorize
            .next_call(matching!(_))
            .answers(&|_, request: OAuthUserAuthorizationRequest| {
                let url = Url::parse(request.authorization_url()).unwrap();
                let state = url
                    .query_pairs()
                    .find(|(name, _)| name == "state")
                    .map(|(_, value)| value.into_owned())
                    .unwrap();
                Ok(OAuthAuthorizationResponse::authorized("code", Some(state)))
            }),
    );
    let dns = Unimock::new(
        OAuthDnsResolverMock::resolve
            .next_call(matching!("auth.example", 443))
            .returns(Ok(vec!["93.184.216.34".parse::<IpAddr>().unwrap()])),
    );
    let clock = Unimock::new(
        OAuthClockMock::now_unix_seconds
            .each_call(matching!())
            .answers(&|_| Ok(1_000)),
    );
    let random = Unimock::new(
        OAuthRandomMock::bytes
            .each_call(matching!(32))
            .answers(&|_, _| Ok(vec![7; 32])),
    );
    Arc::new(manager(
        discovery,
        registry,
        store,
        user_agent,
        transport,
        dns,
        clock,
        random,
        McpOAuthConfig::default(),
    ))
}

pub(super) fn spawn_authorize(
    oauth: Arc<DefaultMcpOAuthManager>,
) -> tokio::task::JoinHandle<Result<OAuthConnection>> {
    tokio::spawn(async move {
        oauth
            .authorize(
                &challenge(McpAuthorizationFailure::InsufficientScope, &["write"]),
                &context(),
            )
            .await
    })
}

pub(super) fn spawn_refresh(
    oauth: Arc<DefaultMcpOAuthManager>,
) -> tokio::task::JoinHandle<Result<OAuthConnection>> {
    tokio::spawn(async move { oauth.refresh(&key("account")).await })
}

pub(super) fn authorized_response() -> OAuthHttpResponse {
    OAuthHttpResponse {
        status: 200,
        headers: Default::default(),
        body: json!({
            "access_token": "authorized-access",
            "refresh_token": "authorized-refresh",
            "token_type": "Bearer",
            "scope": "read write"
        }),
    }
}

pub(super) fn refreshed_response() -> OAuthHttpResponse {
    OAuthHttpResponse {
        status: 200,
        headers: Default::default(),
        body: json!({
            "access_token": "refreshed-old-access",
            "refresh_token": "refreshed-old-refresh",
            "token_type": "Bearer",
            "scope": "read"
        }),
    }
}

pub(super) fn invalid_grant_response() -> OAuthHttpResponse {
    OAuthHttpResponse {
        status: 400,
        headers: Default::default(),
        body: json!({"error": "invalid_grant"}),
    }
}

pub(super) fn grant_type(fields: &[(String, String)]) -> Option<&str> {
    fields
        .iter()
        .find(|(name, _)| name == "grant_type")
        .map(|(_, value)| value.as_str())
}

pub(super) fn assert_authorized(stored: &Arc<std::sync::Mutex<Option<OAuthTokenSet>>>) {
    let stored = stored.lock().unwrap().clone().unwrap();
    assert_eq!(
        secrecy::ExposeSecret::expose_secret(&stored.access_token),
        "authorized-access"
    );
    assert_eq!(stored.scopes.as_slice(), &["read", "write"]);
}
