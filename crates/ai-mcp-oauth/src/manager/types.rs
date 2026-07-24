//! Public authorization inputs and secret-free connection summaries.

use crate::{CanonicalMcpResource, OAuthClientRegistration, OAuthCredentialKey, OAuthScopes};

#[derive(Clone, Debug)]
/// Host-approved context for one explicit OAuth authorization interaction.
pub struct OAuthAuthorizationContext {
    /// Host account or user identity used in the credential key.
    pub account_id: String,
    /// Exact canonical MCP endpoint being authorized.
    pub resource: CanonicalMcpResource,
    /// Exact host callback URI.
    pub redirect_uri: String,
    /// Stable public client name.
    pub client_name: String,
    /// Host-approved scopes requested for every authorization.
    pub baseline_scopes: OAuthScopes,
    /// Optional preconfigured public client for the discovered issuer.
    pub configured_registration: Option<OAuthClientRegistration>,
    /// Host-defined identity for one connection/consent attempt.
    pub authorization_attempt_id: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
/// Secret-free summary of a stored OAuth connection.
pub struct OAuthConnection {
    /// Exact credential identity under which tokens were saved.
    pub key: OAuthCredentialKey,
    /// Scopes granted by the authorization server.
    pub scopes: OAuthScopes,
    /// Absolute token expiry when supplied by the server.
    pub expires_at: Option<u64>,
}
