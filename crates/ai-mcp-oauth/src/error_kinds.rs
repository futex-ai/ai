//! Public typed categories embedded in the OAuth error contract.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
/// Configuration field whose value failed validation.
pub enum OAuthConfigField {
    /// HTTP request timeout.
    HttpTimeout,
    /// User-agent callback timeout.
    UserAgentTimeout,
    /// Authorization state lifetime.
    StateLifetime,
    /// HTTP response byte limit.
    ResponseLimit,
    /// HTTP redirect count limit.
    RedirectLimit,
    /// Metadata cache lifetime.
    MetadataCacheAge,
    /// Access-token refresh skew.
    RefreshSkew,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
/// OAuth endpoint being validated or requested.
pub enum OAuthEndpointKind {
    /// Protected-resource metadata.
    ProtectedResourceMetadata,
    /// Authorization-server metadata.
    AuthorizationServerMetadata,
    /// Browser authorization endpoint.
    Authorization,
    /// Token endpoint.
    Token,
    /// Dynamic client registration endpoint.
    Registration,
    /// Token revocation endpoint.
    Revocation,
    /// Registered callback endpoint.
    Redirect,
    /// Canonical MCP resource.
    Resource,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
/// Security reason an OAuth URL was rejected.
pub enum OAuthUnsafeUrlReason {
    /// URL scheme is not allowed.
    Scheme,
    /// URL contains user information.
    UserInfo,
    /// URL contains a fragment.
    Fragment,
    /// URL has no host.
    MissingHost,
    /// URL uses a blocked port.
    Port,
    /// URL targets a private, reserved, local, or otherwise unsafe address.
    Address,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
/// Secure-store operation that failed.
pub enum OAuthStoreOperation {
    /// Load a client registration.
    LoadRegistration,
    /// Save a client registration.
    SaveRegistration,
    /// Delete a client registration.
    DeleteRegistration,
    /// Load tokens.
    LoadTokens,
    /// Save tokens.
    SaveTokens,
    /// Delete tokens.
    DeleteTokens,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
/// Standard OAuth authorization callback error.
pub enum OAuthAuthorizationError {
    /// Resource owner denied the request.
    AccessDenied,
    /// Authorization request itself was invalid.
    InvalidRequest,
    /// Client is not permitted to use this flow.
    UnauthorizedClient,
    /// Requested response type is unsupported.
    UnsupportedResponseType,
    /// Requested scope is invalid.
    InvalidScope,
    /// Authorization server encountered a temporary failure.
    TemporarilyUnavailable,
    /// Authorization server encountered another server failure.
    ServerError,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
/// Standard OAuth token endpoint error.
pub enum OAuthTokenError {
    /// Request was malformed.
    InvalidRequest,
    /// Public client was not accepted.
    InvalidClient,
    /// Authorization code or refresh token is invalid.
    InvalidGrant,
    /// Grant type is unsupported.
    UnsupportedGrantType,
    /// Requested scope is invalid.
    InvalidScope,
    /// Token endpoint returned an unrecognized error code.
    Unrecognized,
}
