//! Public OAuth registration and credential identity types.

use crate::CanonicalMcpResource;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
/// Origin of an OAuth public-client registration.
pub enum OAuthClientRegistrationSource {
    /// Client ID supplied directly by the embedding host.
    Configured,
    /// Client ID created through RFC 7591 dynamic registration.
    Dynamic,
}

#[derive(Clone, Debug, Eq, PartialEq)]
/// Public OAuth client registration used for one exact redirect URI.
pub struct OAuthClientRegistration {
    /// Public client identifier.
    pub client_id: String,
    /// Exact registered callback URI.
    pub redirect_uri: String,
    /// Stable host-visible client name used as a cache-key component.
    pub client_name: String,
    /// Registration origin.
    pub source: OAuthClientRegistrationSource,
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
/// Store key for a dynamic public-client registration.
pub struct OAuthRegistrationKey {
    /// Authorization-server issuer.
    pub issuer: String,
    /// Exact registered callback URI.
    pub redirect_uri: String,
    /// Stable client metadata identity.
    pub client_name: String,
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
/// Store key that prevents OAuth tokens crossing accounts or resources.
pub struct OAuthCredentialKey {
    /// Host account or user identity.
    pub account_id: String,
    /// Exact canonical MCP resource.
    pub resource: CanonicalMcpResource,
    /// Authorization-server issuer.
    pub issuer: String,
    /// Public client identifier.
    pub client_id: String,
    /// Exact registered callback URI.
    pub redirect_uri: String,
}

impl OAuthCredentialKey {
    /// Builds a registration key for this credential identity.
    pub fn registration_key(&self, client_name: impl Into<String>) -> OAuthRegistrationKey {
        OAuthRegistrationKey {
            issuer: self.issuer.clone(),
            redirect_uri: self.redirect_uri.clone(),
            client_name: client_name.into(),
        }
    }
}
