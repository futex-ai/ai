//! Explicit OAuth manager and non-interactive request-token boundary.

use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

use ai_mcp::McpAuthorizationChallenge;
use async_trait::async_trait;
use secrecy::SecretString;
use tokio::sync::Mutex;

use crate::{
    DynMcpOAuthDiscovery, DynOAuthClientRegistry, DynOAuthClock, DynOAuthCredentialStore,
    DynOAuthHttpTransport, DynOAuthRandom, DynOAuthUserAgent, McpOAuthConfig, OAuthCredentialKey,
    OAuthScopes, Result, state::AuthorizationStateTracker,
};

pub use types::{OAuthAuthorizationContext, OAuthConnection};

mod authorize;
mod disconnect;
mod refresh;
mod token_endpoint;
mod types;

/// Shared explicit OAuth manager.
pub type DynMcpOAuthManager = Arc<dyn McpOAuthManager>;

/// Shared non-interactive request-token provider.
pub type DynOAuthRequestTokenProvider = Arc<dyn OAuthRequestTokenProvider>;

#[cfg_attr(
    any(test, doctest, feature = "test-support"),
    unimock::unimock(api = McpOAuthManagerMock)
)]
#[async_trait]
/// Performs explicit authorization, forced refresh, and disconnect operations.
pub trait McpOAuthManager: Send + Sync {
    /// Performs one explicit host-approved user-agent authorization.
    async fn authorize(
        &self,
        challenge: &McpAuthorizationChallenge,
        context: &OAuthAuthorizationContext,
    ) -> Result<OAuthConnection>;

    /// Forces one non-interactive refresh or requests user interaction.
    async fn refresh(&self, key: &OAuthCredentialKey) -> Result<OAuthConnection>;

    /// Best-effort revokes then unconditionally removes local tokens.
    async fn disconnect(&self, key: &OAuthCredentialKey) -> Result<()>;
}

#[cfg_attr(
    any(test, doctest, feature = "test-support"),
    unimock::unimock(api = OAuthRequestTokenProviderMock)
)]
#[async_trait]
/// Supplies a fresh request token without invoking a user agent.
pub trait OAuthRequestTokenProvider: Send + Sync {
    /// Returns a fresh resource-bound token, or `None` when interaction is needed.
    async fn token_for_request(&self, key: &OAuthCredentialKey) -> Result<Option<SecretString>>;
}

/// Default OAuth manager over fully injected host and protocol boundaries.
pub struct DefaultMcpOAuthManager {
    pub(super) discovery: DynMcpOAuthDiscovery,
    pub(super) registry: DynOAuthClientRegistry,
    pub(super) store: DynOAuthCredentialStore,
    pub(super) user_agent: DynOAuthUserAgent,
    pub(super) transport: DynOAuthHttpTransport,
    pub(super) clock: DynOAuthClock,
    pub(super) random: DynOAuthRandom,
    pub(super) config: McpOAuthConfig,
    pub(super) states: AuthorizationStateTracker,
    pub(super) refresh_locks: Mutex<BTreeMap<OAuthCredentialKey, Arc<Mutex<()>>>>,
    pub(super) denied_prompts: Mutex<BTreeSet<DeniedPromptKey>>,
}

impl DefaultMcpOAuthManager {
    /// Builds a validated manager from injected host and protocol services.
    #[expect(
        clippy::too_many_arguments,
        reason = "composition root injects each independent impure boundary"
    )]
    pub fn new(
        discovery: DynMcpOAuthDiscovery,
        registry: DynOAuthClientRegistry,
        store: DynOAuthCredentialStore,
        user_agent: DynOAuthUserAgent,
        transport: DynOAuthHttpTransport,
        clock: DynOAuthClock,
        random: DynOAuthRandom,
        config: McpOAuthConfig,
    ) -> Result<Self> {
        config.validate()?;
        Ok(Self {
            discovery,
            registry,
            store,
            user_agent,
            transport,
            clock,
            random,
            config,
            states: AuthorizationStateTracker::new(),
            refresh_locks: Mutex::new(BTreeMap::new()),
            denied_prompts: Mutex::new(BTreeSet::new()),
        })
    }

    pub(super) async fn refresh_lock(&self, key: &OAuthCredentialKey) -> Arc<Mutex<()>> {
        self.refresh_locks
            .lock()
            .await
            .entry(key.clone())
            .or_insert_with(|| Arc::new(Mutex::new(())))
            .clone()
    }
}

#[async_trait]
impl McpOAuthManager for DefaultMcpOAuthManager {
    async fn authorize(
        &self,
        challenge: &McpAuthorizationChallenge,
        context: &OAuthAuthorizationContext,
    ) -> Result<OAuthConnection> {
        self.authorize_inner(challenge, context).await
    }

    async fn refresh(&self, key: &OAuthCredentialKey) -> Result<OAuthConnection> {
        self.explicit_refresh(key).await
    }

    async fn disconnect(&self, key: &OAuthCredentialKey) -> Result<()> {
        self.disconnect_inner(key).await
    }
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(super) struct DeniedPromptKey {
    pub(super) attempt_id: String,
    pub(super) account_id: String,
    pub(super) resource: crate::CanonicalMcpResource,
    pub(super) scopes: OAuthScopes,
}
