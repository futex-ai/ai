//! Shared OAuth manager fixtures.

use std::{
    collections::BTreeMap,
    net::{IpAddr, Ipv4Addr},
    sync::Arc,
};

use ai_mcp::{McpAuthorizationChallenge, McpAuthorizationFailure};
use secrecy::SecretString;
use serde_json::json;
use unimock::{MockFn, Unimock, matching};

use crate::{
    AuthorizationServerMetadata, CanonicalMcpResource, DefaultMcpOAuthManager,
    DynMcpOAuthDiscovery, DynOAuthClientRegistry, DynOAuthClock, DynOAuthCredentialStore,
    DynOAuthDnsResolver, DynOAuthHttpTransport, DynOAuthRandom, DynOAuthUserAgent, McpOAuthConfig,
    McpOAuthDiscoveryMock, OAuthAuthorizationContext, OAuthClientRegistration,
    OAuthClientRegistrationSource, OAuthClockMock, OAuthCredentialKey, OAuthDiscoveryResult,
    OAuthDnsResolverMock, OAuthHttpResponse, OAuthHttpTransportMock, OAuthScopes, OAuthTokenSet,
    OAuthTokenType, OAuthUrlPolicy, ProtectedResourceMetadata,
};

#[expect(
    clippy::too_many_arguments,
    reason = "test fixture mirrors the production composition root"
)]
pub(super) fn manager(
    discovery: Unimock,
    registry: Unimock,
    store: Unimock,
    user_agent: Unimock,
    transport: Unimock,
    dns_resolver: Unimock,
    clock: Unimock,
    random: Unimock,
    config: McpOAuthConfig,
) -> DefaultMcpOAuthManager {
    manager_with_boundaries(
        discovery,
        registry,
        store,
        Arc::new(user_agent) as DynOAuthUserAgent,
        transport,
        Arc::new(dns_resolver) as DynOAuthDnsResolver,
        clock,
        random,
        config,
    )
}

#[expect(
    clippy::too_many_arguments,
    reason = "test fixture mirrors the production composition root"
)]
pub(super) fn manager_with_user_agent(
    discovery: Unimock,
    registry: Unimock,
    store: Unimock,
    user_agent: DynOAuthUserAgent,
    transport: Unimock,
    dns_resolver: Unimock,
    clock: Unimock,
    random: Unimock,
    config: McpOAuthConfig,
) -> DefaultMcpOAuthManager {
    manager_with_boundaries(
        discovery,
        registry,
        store,
        user_agent,
        transport,
        Arc::new(dns_resolver) as DynOAuthDnsResolver,
        clock,
        random,
        config,
    )
}

#[expect(
    clippy::too_many_arguments,
    reason = "test fixture mirrors the production composition root"
)]
pub(super) fn manager_with_dns_resolver(
    discovery: Unimock,
    registry: Unimock,
    store: Unimock,
    user_agent: Unimock,
    transport: Unimock,
    dns_resolver: DynOAuthDnsResolver,
    clock: Unimock,
    random: Unimock,
    config: McpOAuthConfig,
) -> DefaultMcpOAuthManager {
    manager_with_boundaries(
        discovery,
        registry,
        store,
        Arc::new(user_agent) as DynOAuthUserAgent,
        transport,
        dns_resolver,
        clock,
        random,
        config,
    )
}

#[expect(
    clippy::too_many_arguments,
    reason = "test fixture mirrors the production composition root"
)]
fn manager_with_boundaries(
    discovery: Unimock,
    registry: Unimock,
    store: Unimock,
    user_agent: DynOAuthUserAgent,
    transport: Unimock,
    dns_resolver: DynOAuthDnsResolver,
    clock: Unimock,
    random: Unimock,
    config: McpOAuthConfig,
) -> DefaultMcpOAuthManager {
    DefaultMcpOAuthManager::new(
        Arc::new(discovery) as DynMcpOAuthDiscovery,
        Arc::new(registry) as DynOAuthClientRegistry,
        Arc::new(store) as DynOAuthCredentialStore,
        user_agent,
        Arc::new(transport) as DynOAuthHttpTransport,
        dns_resolver,
        Arc::new(clock) as DynOAuthClock,
        Arc::new(random) as DynOAuthRandom,
        config,
    )
    .unwrap()
}

pub(super) fn public_dns_resolver() -> Unimock {
    Unimock::new(
        OAuthDnsResolverMock::resolve
            .next_call(matching!("auth.example", 443))
            .returns(Ok(vec![IpAddr::V4(Ipv4Addr::new(93, 184, 216, 34))])),
    )
}

pub(super) fn clock(times: Vec<u64>) -> Unimock {
    let times = Arc::new(std::sync::Mutex::new(
        times.into_iter().collect::<std::collections::VecDeque<_>>(),
    ));
    Unimock::new(
        OAuthClockMock::now_unix_seconds
            .each_call(matching!())
            .answers_arc(Arc::new(move |_| {
                Ok(times
                    .lock()
                    .unwrap()
                    .pop_front()
                    .expect("unexpected clock read"))
            })),
    )
}

pub(super) fn resource() -> CanonicalMcpResource {
    CanonicalMcpResource::parse("https://mcp.example/api", &OAuthUrlPolicy::default()).unwrap()
}

pub(super) fn challenge(
    failure: McpAuthorizationFailure,
    scopes: &[&str],
) -> McpAuthorizationChallenge {
    McpAuthorizationChallenge {
        failure,
        resource_metadata_url: None,
        scopes: scopes.iter().map(|scope| (*scope).to_owned()).collect(),
        error_description: None,
        raw_www_authenticate: Vec::new(),
    }
}

pub(super) fn context() -> OAuthAuthorizationContext {
    OAuthAuthorizationContext {
        account_id: "account".to_owned(),
        resource: resource(),
        redirect_uri: "https://app.example/callback".to_owned(),
        client_name: "Montgomery".to_owned(),
        baseline_scopes: OAuthScopes::new(["read"]),
        configured_registration: None,
        authorization_attempt_id: "attempt-1".to_owned(),
    }
}

pub(super) fn discovery_result() -> OAuthDiscoveryResult {
    OAuthDiscoveryResult {
        resource_metadata_url: "https://mcp.example/.well-known/oauth-protected-resource/api"
            .to_owned(),
        protected_resource: ProtectedResourceMetadata {
            resource: resource().to_string(),
            authorization_servers: vec!["https://auth.example".to_owned()],
            scopes_supported: OAuthScopes::new(["read", "write"]),
            unknown: BTreeMap::new(),
        },
        authorization_server: server_metadata(),
    }
}

pub(super) fn server_metadata() -> AuthorizationServerMetadata {
    AuthorizationServerMetadata {
        issuer: "https://auth.example".to_owned(),
        authorization_endpoint: "https://auth.example/authorize".to_owned(),
        token_endpoint: "https://auth.example/token".to_owned(),
        registration_endpoint: Some("https://auth.example/register".to_owned()),
        revocation_endpoint: Some("https://auth.example/revoke".to_owned()),
        grant_types_supported: vec!["authorization_code".to_owned(), "refresh_token".to_owned()],
        token_endpoint_auth_methods_supported: vec!["none".to_owned()],
        code_challenge_methods_supported: vec!["S256".to_owned()],
        scopes_supported: OAuthScopes::new(["read", "write"]),
        unknown: BTreeMap::new(),
    }
}

pub(super) fn registration() -> OAuthClientRegistration {
    OAuthClientRegistration {
        client_id: "client-id".to_owned(),
        redirect_uri: "https://app.example/callback".to_owned(),
        client_name: "Montgomery".to_owned(),
        source: OAuthClientRegistrationSource::Dynamic,
    }
}

pub(super) fn key(account_id: &str) -> OAuthCredentialKey {
    OAuthCredentialKey {
        account_id: account_id.to_owned(),
        resource: resource(),
        issuer: "https://auth.example".to_owned(),
        client_id: "client-id".to_owned(),
        redirect_uri: "https://app.example/callback".to_owned(),
    }
}

pub(super) fn tokens(
    access: &str,
    refresh: Option<&str>,
    expires_at: Option<u64>,
) -> OAuthTokenSet {
    OAuthTokenSet {
        access_token: SecretString::from(access.to_owned()),
        refresh_token: refresh.map(|token| SecretString::from(token.to_owned())),
        token_type: OAuthTokenType::Bearer,
        expires_at,
        scopes: OAuthScopes::new(["read"]),
    }
}

pub(super) fn refresh_manager(
    store: Unimock,
    transport: Unimock,
    clock: Unimock,
) -> DefaultMcpOAuthManager {
    manager(
        Unimock::new(
            McpOAuthDiscoveryMock::authorization_server
                .next_call(matching!(_))
                .returns(Ok(server_metadata())),
        ),
        Unimock::new(()),
        store,
        Unimock::new(()),
        transport,
        Unimock::new(()),
        clock,
        Unimock::new(()),
        McpOAuthConfig::default(),
    )
}

pub(super) fn successful_refresh_transport(access_token: &str) -> Unimock {
    Unimock::new(
        OAuthHttpTransportMock::post_form
            .next_call(matching!(_, _, _, _, _))
            .returns(Ok(OAuthHttpResponse {
                status: 200,
                headers: Default::default(),
                body: json!({
                    "access_token": access_token,
                    "token_type": "Bearer",
                    "expires_in": 300
                }),
            })),
    )
}
