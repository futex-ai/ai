//! Typed, secret-safe OAuth error contract.

use serde_json::Value;
use thiserror::Error;

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

#[derive(Debug, Error)]
/// Errors returned by MCP OAuth discovery and token management.
pub enum Error {
    /// One pure configuration bound is invalid.
    #[error("[ai_mcp_oauth/config] invalid {field:?}")]
    InvalidConfig {
        /// Invalid configuration field.
        field: OAuthConfigField,
    },
    /// A URL could not be parsed as an absolute URI.
    #[error("[ai_mcp_oauth/url] invalid {endpoint:?} URL")]
    InvalidUrl {
        /// Kind of endpoint being parsed.
        endpoint: OAuthEndpointKind,
    },
    /// A URL violates the configured security policy.
    #[error("[ai_mcp_oauth/url] unsafe {endpoint:?} URL: {reason:?}")]
    UnsafeUrl {
        /// Kind of endpoint being validated.
        endpoint: OAuthEndpointKind,
        /// Policy reason for rejection.
        reason: OAuthUnsafeUrlReason,
    },
    /// Protected-resource metadata names a different resource.
    #[error("[ai_mcp_oauth/discovery] protected resource mismatch")]
    ResourceMismatch {
        /// Canonical resource requested by the client.
        expected: String,
        /// Resource returned by metadata.
        actual: String,
    },
    /// Authorization-server metadata names a different issuer.
    #[error("[ai_mcp_oauth/discovery] authorization issuer mismatch")]
    IssuerMismatch {
        /// Issuer used to form the metadata URL.
        expected: String,
        /// Issuer returned by metadata.
        actual: String,
    },
    /// A discovery endpoint returned an unsuccessful status.
    #[error("[ai_mcp_oauth/discovery] {endpoint:?} returned HTTP {status}")]
    DiscoveryStatus {
        /// Endpoint that failed.
        endpoint: OAuthEndpointKind,
        /// HTTP response status.
        status: u16,
    },
    /// A discovery response did not match its required JSON shape.
    #[error("[ai_mcp_oauth/discovery] invalid {endpoint:?} response: {source}")]
    DiscoverySchema {
        /// Endpoint whose response failed.
        endpoint: OAuthEndpointKind,
        /// Underlying JSON decode failure.
        source: serde_json::Error,
    },
    /// Protected-resource metadata advertised no authorization server.
    #[error("[ai_mcp_oauth/discovery] no authorization server advertised")]
    MissingAuthorizationServer,
    /// Required authorization-server metadata is absent.
    #[error("[ai_mcp_oauth/discovery] missing {endpoint:?}")]
    MissingEndpoint {
        /// Missing endpoint.
        endpoint: OAuthEndpointKind,
    },
    /// Host cancelled selection among multiple authorization servers.
    #[error("[ai_mcp_oauth/discovery] authorization server selection cancelled")]
    IssuerSelectionCancelled,
    /// Host selected an issuer that was not advertised.
    #[error("[ai_mcp_oauth/discovery] selected authorization server was not advertised")]
    InvalidIssuerSelection,
    /// No configured, cached, or dynamic public client is available.
    #[error("[ai_mcp_oauth/registration] public client registration required")]
    ClientRegistrationRequired {
        /// Authorization server requiring registration.
        issuer: String,
        /// Redirect URI requiring registration.
        redirect_uri: String,
    },
    /// Authorization server does not support unauthenticated public clients.
    #[error("[ai_mcp_oauth/registration] public clients are unsupported")]
    PublicClientUnsupported,
    /// Dynamic registration was rejected.
    #[error("[ai_mcp_oauth/registration] registration rejected with HTTP {status}")]
    RegistrationRejected {
        /// HTTP response status.
        status: u16,
        /// Structured non-secret response body.
        body: Value,
    },
    /// Dynamic registration response was malformed.
    #[error("[ai_mcp_oauth/registration] invalid registration response: {source}")]
    RegistrationSchema {
        /// Underlying JSON decode failure.
        source: serde_json::Error,
    },
    /// A configured registration does not match the approved callback identity.
    #[error("[ai_mcp_oauth/registration] configured registration identity mismatch")]
    RegistrationMismatch,
    /// A secure credential-store operation failed.
    #[error("[ai_mcp_oauth/store] {operation:?} failed")]
    Store {
        /// Store operation that failed.
        operation: OAuthStoreOperation,
    },
    /// DNS lookup failed without exposing request secrets.
    #[error("[ai_mcp_oauth/transport] DNS resolution failed")]
    Dns,
    /// HTTP transport failed without exposing request secrets.
    #[error("[ai_mcp_oauth/transport] request failed")]
    Transport,
    /// Response body exceeded the configured limit.
    #[error("[ai_mcp_oauth/transport] response exceeded {limit_bytes} bytes")]
    ResponseTooLarge {
        /// Configured maximum response bytes.
        limit_bytes: usize,
    },
    /// Too many validated redirects were followed.
    #[error("[ai_mcp_oauth/transport] redirect limit exceeded")]
    TooManyRedirects,
    /// Redirect response omitted or malformed its location.
    #[error("[ai_mcp_oauth/transport] invalid redirect location")]
    InvalidRedirect,
    /// Response body was not valid JSON.
    #[error("[ai_mcp_oauth/transport] response body was not JSON")]
    InvalidJsonResponse,
    /// Random byte generation failed.
    #[error("[ai_mcp_oauth/random] secure random generation failed")]
    Random,
    /// System clock could not produce a valid UNIX timestamp.
    #[error("[ai_mcp_oauth/clock] system time precedes the UNIX epoch")]
    Clock,
    /// An implementation detail failed without a caller-actionable category.
    #[error("[ai_mcp_oauth/internal] internal failure")]
    Internal,
}

/// Result alias for MCP OAuth operations.
pub type Result<T> = std::result::Result<T, Error>;
