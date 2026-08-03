//! Host-side OAuth discovery and token management for remote MCP servers.

#![warn(unreachable_pub)]

mod auth_hook;
mod clock;
mod config;
mod discovery;
mod error;
mod error_kinds;
mod manager;
mod metadata;
mod pkce;
mod random;
mod registration;
mod registration_types;
mod resource;
mod selector;
mod state;
mod store;
mod token;
mod transport;
mod url_policy;
mod user_agent;

pub use auth_hook::RefreshingMcpAuth;
pub use clock::{DynOAuthClock, OAuthClock, SystemOAuthClock};
pub use config::McpOAuthConfig;
pub use discovery::{
    DefaultMcpOAuthDiscovery, DynMcpOAuthDiscovery, McpOAuthDiscovery, OAuthDiscoveryResult,
};
pub use error::{Error, Result};
pub use error_kinds::{
    OAuthAuthorizationError, OAuthConfigField, OAuthEndpointKind, OAuthStoreOperation,
    OAuthTokenError, OAuthUnsafeUrlReason,
};
pub use manager::{
    DefaultMcpOAuthManager, DynMcpOAuthManager, DynOAuthRequestTokenProvider, McpOAuthManager,
    OAuthAuthorizationContext, OAuthConnection, OAuthRequestTokenProvider,
};
pub use metadata::{AuthorizationServerMetadata, ProtectedResourceMetadata};
pub use random::{DynOAuthRandom, OAuthRandom, SystemOAuthRandom};
pub use registration::{
    DefaultOAuthClientRegistry, DynOAuthClientRegistry, OAuthClientRegistry,
    OAuthRegistrationRequest,
};
pub use registration_types::{
    OAuthClientRegistration, OAuthClientRegistrationSource, OAuthCredentialKey,
    OAuthRegistrationKey,
};
pub use resource::{CanonicalMcpResource, OAuthScopes};
pub use selector::{AuthorizationServerSelector, DynAuthorizationServerSelector};
pub use store::{DynOAuthCredentialStore, OAuthCredentialStore};
pub use token::{OAuthTokenSet, OAuthTokenType};
pub use transport::{
    DynOAuthDnsResolver, DynOAuthHttpTransport, OAuthDnsResolver, OAuthHttpLimits,
    OAuthHttpResponse, OAuthHttpTransport, ReqwestOAuthHttpTransport, SystemOAuthDnsResolver,
};
pub use url_policy::OAuthUrlPolicy;
pub use user_agent::{
    DynOAuthUserAgent, OAuthAuthorizationResponse, OAuthUserAgent, OAuthUserAuthorizationRequest,
};

#[cfg(any(test, doctest, feature = "test-support"))]
pub use clock::OAuthClockMock;
#[cfg(any(test, doctest, feature = "test-support"))]
pub use discovery::McpOAuthDiscoveryMock;
#[cfg(any(test, doctest, feature = "test-support"))]
pub use manager::{McpOAuthManagerMock, OAuthRequestTokenProviderMock};
#[cfg(any(test, doctest, feature = "test-support"))]
pub use random::OAuthRandomMock;
#[cfg(any(test, doctest, feature = "test-support"))]
pub use registration::OAuthClientRegistryMock;
#[cfg(any(test, doctest, feature = "test-support"))]
pub use selector::AuthorizationServerSelectorMock;
#[cfg(any(test, doctest, feature = "test-support"))]
pub use store::OAuthCredentialStoreMock;
#[cfg(any(test, doctest, feature = "test-support"))]
pub use transport::{OAuthDnsResolverMock, OAuthHttpTransportMock};
#[cfg(any(test, doctest, feature = "test-support"))]
pub use user_agent::OAuthUserAgentMock;

#[cfg(test)]
#[path = "_tests_/mod.rs"]
mod tests;
