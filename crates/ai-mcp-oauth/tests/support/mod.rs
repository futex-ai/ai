//! Shared host and in-process server support for OAuth integration tests.

use std::{
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use ai_mcp::{McpServerConfig, ReqwestMcpHttpTransport, StreamableHttpMcpClient};
use ai_mcp_oauth::{
    CanonicalMcpResource, DefaultMcpOAuthDiscovery, DefaultMcpOAuthManager,
    DefaultOAuthClientRegistry, DynAuthorizationServerSelector, DynMcpOAuthDiscovery,
    DynOAuthClientRegistry, DynOAuthClock, DynOAuthCredentialStore, DynOAuthHttpTransport,
    DynOAuthRandom, DynOAuthRequestTokenProvider, DynOAuthUserAgent, McpOAuthConfig,
    OAuthAuthorizationContext, OAuthClientRegistration, OAuthClientRegistrationSource,
    OAuthCredentialKey, OAuthScopes, OAuthTokenSet, OAuthTokenType, OAuthUrlPolicy,
    RefreshingMcpAuth, ReqwestOAuthHttpTransport, SystemOAuthClock, SystemOAuthRandom,
};
use json_http::{DynJsonHttpAuth, StaticHeaderAuth};
use secrecy::SecretString;

mod server;
mod store;
mod user_agent;

pub(crate) use server::FakeOAuthMcpServer;
pub(crate) use store::MemoryCredentialStore;
pub(crate) use user_agent::LoopbackUserAgent;

pub(crate) struct OAuthHarness {
    pub(crate) manager: Arc<DefaultMcpOAuthManager>,
    pub(crate) store: Arc<MemoryCredentialStore>,
    pub(crate) user_agent: Arc<LoopbackUserAgent>,
    pub(crate) resource: CanonicalMcpResource,
    pub(crate) context: OAuthAuthorizationContext,
}

pub(crate) fn harness(server: &FakeOAuthMcpServer) -> OAuthHarness {
    let config = McpOAuthConfig {
        url_policy: OAuthUrlPolicy::loopback_development(),
        ..McpOAuthConfig::default()
    };
    let transport = Arc::new(ReqwestOAuthHttpTransport::new()) as DynOAuthHttpTransport;
    let store = Arc::new(MemoryCredentialStore::default());
    let user_agent = Arc::new(LoopbackUserAgent::default());
    let discovery = Arc::new(
        DefaultMcpOAuthDiscovery::new(
            transport.clone(),
            Arc::new(user_agent::FirstIssuerSelector) as DynAuthorizationServerSelector,
            Arc::new(SystemOAuthClock) as DynOAuthClock,
            config.clone(),
        )
        .unwrap(),
    ) as DynMcpOAuthDiscovery;
    let registry = Arc::new(
        DefaultOAuthClientRegistry::new(
            transport.clone(),
            store.clone() as DynOAuthCredentialStore,
            config.clone(),
        )
        .unwrap(),
    ) as DynOAuthClientRegistry;
    let manager = Arc::new(
        DefaultMcpOAuthManager::new(
            discovery,
            registry,
            store.clone() as DynOAuthCredentialStore,
            user_agent.clone() as DynOAuthUserAgent,
            transport,
            Arc::new(SystemOAuthClock) as DynOAuthClock,
            Arc::new(SystemOAuthRandom) as DynOAuthRandom,
            config.clone(),
        )
        .unwrap(),
    );
    let resource = CanonicalMcpResource::parse(&server.mcp_url, &config.url_policy).unwrap();
    let context = OAuthAuthorizationContext {
        account_id: "account".to_owned(),
        resource: resource.clone(),
        redirect_uri: "http://127.0.0.1/callback".to_owned(),
        client_name: "OAuth integration test".to_owned(),
        baseline_scopes: OAuthScopes::new(["read"]),
        configured_registration: None,
        authorization_attempt_id: "attempt-1".to_owned(),
    };
    OAuthHarness {
        manager,
        store,
        user_agent,
        resource,
        context,
    }
}

pub(crate) fn unauthenticated_client(server: &FakeOAuthMcpServer) -> StreamableHttpMcpClient {
    mcp_client(server, Arc::new(StaticHeaderAuth::default()))
}

pub(crate) fn authenticated_client(
    server: &FakeOAuthMcpServer,
    harness: &OAuthHarness,
    key: OAuthCredentialKey,
) -> StreamableHttpMcpClient {
    let provider = harness.manager.clone() as DynOAuthRequestTokenProvider;
    let auth = Arc::new(RefreshingMcpAuth::new(harness.resource.clone(), key, provider).unwrap())
        as DynJsonHttpAuth;
    mcp_client(server, auth)
}

pub(crate) fn credential_key(server: &FakeOAuthMcpServer) -> OAuthCredentialKey {
    let resource =
        CanonicalMcpResource::parse(&server.mcp_url, &OAuthUrlPolicy::loopback_development())
            .unwrap();
    OAuthCredentialKey {
        account_id: "account".to_owned(),
        resource,
        issuer: server.base_url.clone(),
        client_id: "client-id".to_owned(),
        redirect_uri: "http://127.0.0.1/callback".to_owned(),
    }
}

pub(crate) fn configured_registration() -> OAuthClientRegistration {
    OAuthClientRegistration {
        client_id: "client-id".to_owned(),
        redirect_uri: "http://127.0.0.1/callback".to_owned(),
        client_name: "OAuth integration test".to_owned(),
        source: OAuthClientRegistrationSource::Configured,
    }
}

pub(crate) fn tokens(
    access: &str,
    refresh: Option<&str>,
    expires_at: Option<u64>,
    scopes: &[&str],
) -> OAuthTokenSet {
    OAuthTokenSet {
        access_token: SecretString::from(access.to_owned()),
        refresh_token: refresh.map(|value| SecretString::from(value.to_owned())),
        token_type: OAuthTokenType::Bearer,
        expires_at,
        scopes: OAuthScopes::new(scopes.iter().copied()),
    }
}

pub(crate) fn fresh_expiry() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
        .saturating_add(3600)
}

fn mcp_client(server: &FakeOAuthMcpServer, auth: DynJsonHttpAuth) -> StreamableHttpMcpClient {
    StreamableHttpMcpClient::new(
        Arc::new(ReqwestMcpHttpTransport::new()),
        auth,
        McpServerConfig::new("oauth", &server.mcp_url),
    )
    .unwrap()
}
