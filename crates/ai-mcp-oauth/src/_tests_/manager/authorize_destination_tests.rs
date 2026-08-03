//! Authorization user-agent destination preflight regressions.

use std::{
    future,
    net::{IpAddr, Ipv4Addr},
    sync::Arc,
    time::Duration,
};

use ai_mcp::McpAuthorizationFailure;
use async_trait::async_trait;
use unimock::{MockFn, Unimock, matching};

use crate::{
    DefaultMcpOAuthManager, DynOAuthDnsResolver, Error, McpOAuthConfig, McpOAuthDiscoveryMock,
    McpOAuthManager, OAuthAuthorizationResponse, OAuthClientRegistryMock, OAuthConnection,
    OAuthDiscoveryResult, OAuthDnsResolver, OAuthDnsResolverMock, OAuthRandomMock,
    OAuthUnsafeUrlReason, OAuthUrlPolicy, OAuthUserAgentMock, Result,
};

use super::support::{
    challenge, clock, context, discovery_result, manager_with_dns_resolver, registration,
};

#[tokio::test]
async fn unsafe_or_mixed_authorization_resolutions_never_reach_the_user_agent() {
    for addresses in [
        vec![IpAddr::V4(Ipv4Addr::LOCALHOST)],
        vec![
            IpAddr::V4(Ipv4Addr::new(93, 184, 216, 34)),
            IpAddr::V4(Ipv4Addr::new(169, 254, 169, 254)),
        ],
        vec!["100::1".parse().unwrap()],
    ] {
        let oauth = authorization_manager(
            resolver_returning(addresses),
            Unimock::new(()),
            discovery_result(),
            Unimock::new(()),
            McpOAuthConfig::default(),
        );

        let error = authorize(&oauth).await.unwrap_err();

        assert!(matches!(
            error,
            Error::UnsafeUrl {
                reason: OAuthUnsafeUrlReason::Address,
                ..
            }
        ));
    }
}

#[tokio::test]
async fn empty_or_failed_authorization_resolution_stops_before_user_agent() {
    for resolver in [
        resolver_returning(Vec::new()),
        Unimock::new(
            OAuthDnsResolverMock::resolve
                .next_call(matching!("auth.example", 443))
                .returns(Err(Error::Dns)),
        ),
    ] {
        let oauth = authorization_manager(
            resolver,
            Unimock::new(()),
            discovery_result(),
            Unimock::new(()),
            McpOAuthConfig::default(),
        );

        assert!(matches!(authorize(&oauth).await, Err(Error::Dns)));
    }
}

#[tokio::test(start_paused = true)]
async fn stalled_authorization_resolution_stops_at_http_timeout() {
    let http_timeout = Duration::from_secs(5);
    let config = McpOAuthConfig {
        http_timeout,
        ..McpOAuthConfig::default()
    };
    let oauth = authorization_manager_with_resolver(
        Arc::new(StalledResolver),
        Unimock::new(()),
        discovery_result(),
        Unimock::new(()),
        config,
    );
    let started_at = tokio::time::Instant::now();

    let bounded =
        tokio::time::timeout(http_timeout + Duration::from_secs(1), authorize(&oauth)).await;

    assert!(matches!(bounded, Ok(Err(Error::Dns))));
    assert_eq!(started_at.elapsed(), http_timeout);
}

#[tokio::test]
async fn public_authorization_resolution_reaches_the_user_agent() {
    let oauth = authorization_manager(
        resolver_returning(vec![IpAddr::V4(Ipv4Addr::new(93, 184, 216, 34))]),
        cancelling_user_agent(),
        discovery_result(),
        clock(vec![100]),
        McpOAuthConfig::default(),
    );

    assert!(matches!(authorize(&oauth).await, Err(Error::UserCancelled)));
}

#[tokio::test]
async fn development_loopback_resolution_reaches_the_user_agent() {
    let mut discovered = discovery_result();
    discovered.authorization_server.authorization_endpoint =
        "http://api.localhost:8123/authorize".to_owned();
    let config = McpOAuthConfig {
        url_policy: OAuthUrlPolicy::loopback_development(),
        ..McpOAuthConfig::default()
    };
    let oauth = authorization_manager(
        Unimock::new(
            OAuthDnsResolverMock::resolve
                .next_call(matching!("api.localhost", 8123))
                .returns(Ok(vec![IpAddr::V4(Ipv4Addr::LOCALHOST)])),
        ),
        cancelling_user_agent(),
        discovered,
        clock(vec![100]),
        config,
    );

    assert!(matches!(authorize(&oauth).await, Err(Error::UserCancelled)));
}

fn authorization_manager(
    resolver: Unimock,
    user_agent: Unimock,
    discovered: OAuthDiscoveryResult,
    clock: Unimock,
    config: McpOAuthConfig,
) -> DefaultMcpOAuthManager {
    authorization_manager_with_resolver(
        Arc::new(resolver) as DynOAuthDnsResolver,
        user_agent,
        discovered,
        clock,
        config,
    )
}

fn authorization_manager_with_resolver(
    resolver: DynOAuthDnsResolver,
    user_agent: Unimock,
    discovered: OAuthDiscoveryResult,
    clock: Unimock,
    config: McpOAuthConfig,
) -> DefaultMcpOAuthManager {
    manager_with_dns_resolver(
        Unimock::new(
            McpOAuthDiscoveryMock::discover
                .next_call(matching!(_, _))
                .returns(Ok(discovered)),
        ),
        Unimock::new(
            OAuthClientRegistryMock::resolve
                .next_call(matching!(_))
                .returns(Ok(registration())),
        ),
        Unimock::new(()),
        user_agent,
        Unimock::new(()),
        resolver,
        clock,
        random(),
        config,
    )
}

async fn authorize(oauth: &DefaultMcpOAuthManager) -> Result<OAuthConnection> {
    oauth
        .authorize(
            &challenge(McpAuthorizationFailure::AuthorizationRequired, &[]),
            &context(),
        )
        .await
}

fn resolver_returning(addresses: Vec<IpAddr>) -> Unimock {
    Unimock::new(
        OAuthDnsResolverMock::resolve
            .next_call(matching!("auth.example", 443))
            .returns(Ok(addresses)),
    )
}

fn cancelling_user_agent() -> Unimock {
    Unimock::new(
        OAuthUserAgentMock::authorize
            .next_call(matching!(_))
            .returns(Ok(OAuthAuthorizationResponse::Cancelled)),
    )
}

fn random() -> Unimock {
    Unimock::new(
        OAuthRandomMock::bytes
            .each_call(matching!(32))
            .answers(&|_, _| Ok(vec![1; 32])),
    )
}

struct StalledResolver;

#[async_trait]
impl OAuthDnsResolver for StalledResolver {
    async fn resolve(&self, _host: &str, _port: u16) -> Result<Vec<IpAddr>> {
        future::pending().await
    }
}
