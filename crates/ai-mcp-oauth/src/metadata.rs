//! Typed OAuth protected-resource and authorization-server metadata.

use std::collections::BTreeMap;

use serde_json::Value;

use crate::OAuthScopes;

#[derive(Clone, Debug, PartialEq)]
/// Validated RFC 9728 protected-resource metadata.
pub struct ProtectedResourceMetadata {
    /// Exact canonical MCP resource identifier.
    pub resource: String,
    /// Advertised authorization-server issuers in wire order.
    pub authorization_servers: Vec<String>,
    /// Optional advertised scope catalog.
    pub scopes_supported: OAuthScopes,
    /// Unrecognized metadata retained at the wire boundary.
    pub unknown: BTreeMap<String, Value>,
}

#[derive(Clone, Debug, PartialEq)]
/// Validated RFC 8414 authorization-server metadata used by MCP OAuth.
pub struct AuthorizationServerMetadata {
    /// Exact selected authorization-server issuer.
    pub issuer: String,
    /// Browser authorization endpoint.
    pub authorization_endpoint: String,
    /// Authorization-code and refresh token endpoint.
    pub token_endpoint: String,
    /// Optional RFC 7591 dynamic registration endpoint.
    pub registration_endpoint: Option<String>,
    /// Optional RFC 7009 token revocation endpoint.
    pub revocation_endpoint: Option<String>,
    /// Advertised grant types.
    pub grant_types_supported: Vec<String>,
    /// Advertised token endpoint authentication methods.
    pub token_endpoint_auth_methods_supported: Vec<String>,
    /// Advertised PKCE challenge methods.
    pub code_challenge_methods_supported: Vec<String>,
    /// Optional advertised scope catalog.
    pub scopes_supported: OAuthScopes,
    /// Unrecognized metadata retained at the wire boundary.
    pub unknown: BTreeMap<String, Value>,
}

impl AuthorizationServerMetadata {
    /// Returns whether the server supports an unauthenticated public client.
    pub fn supports_public_clients(&self) -> bool {
        self.token_endpoint_auth_methods_supported
            .iter()
            .any(|method| method == "none")
    }

    /// Returns whether the server advertises refresh-token grants.
    pub fn supports_refresh_tokens(&self) -> bool {
        self.grant_types_supported
            .iter()
            .any(|grant| grant == "refresh_token")
    }

    /// Returns whether the required S256 PKCE method is supported.
    pub fn supports_s256(&self) -> bool {
        self.code_challenge_methods_supported
            .iter()
            .any(|method| method == "S256")
    }
}
