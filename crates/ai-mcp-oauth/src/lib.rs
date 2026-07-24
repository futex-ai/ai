//! Host-side OAuth discovery and token management for remote MCP servers.

#![warn(unreachable_pub)]

mod clock;
mod config;
mod discovery;
mod error;
mod metadata;
mod random;
mod registration;
mod registration_types;
mod resource;
mod selector;
mod store;
mod token;
mod transport;
mod url_policy;

pub use clock::{DynOAuthClock, OAuthClock, SystemOAuthClock};
pub use config::McpOAuthConfig;
pub use discovery::{
    DefaultMcpOAuthDiscovery, DynMcpOAuthDiscovery, McpOAuthDiscovery, OAuthDiscoveryResult,
};
pub use error::{
    Error, OAuthConfigField, OAuthEndpointKind, OAuthStoreOperation, OAuthUnsafeUrlReason, Result,
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

#[cfg(any(test, doctest, feature = "test-support"))]
pub use clock::OAuthClockMock;
#[cfg(any(test, doctest, feature = "test-support"))]
pub use discovery::McpOAuthDiscoveryMock;
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

#[cfg(test)]
#[path = "_tests_/mod.rs"]
mod tests;
