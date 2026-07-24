//! Typed, secret-safe OAuth error contract.

use serde_json::Value;
use thiserror::Error;

use crate::{
    OAuthAuthorizationError, OAuthConfigField, OAuthEndpointKind, OAuthStoreOperation,
    OAuthTokenError, OAuthUnsafeUrlReason,
};

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
    /// Selected authorization server does not advertise S256 PKCE.
    #[error("[ai_mcp_oauth/authorization] S256 PKCE is unsupported")]
    PkceS256Unsupported,
    /// The supplied MCP challenge does not permit interactive authorization.
    #[error("[ai_mcp_oauth/authorization] challenge does not permit authorization")]
    AuthorizationForbidden,
    /// The resource owner denied the authorization request.
    #[error("[ai_mcp_oauth/authorization] user denied authorization")]
    UserDenied,
    /// The host cancelled the external user-agent interaction.
    #[error("[ai_mcp_oauth/authorization] user-agent interaction cancelled")]
    UserCancelled,
    /// The external user agent did not return before the configured deadline.
    #[error("[ai_mcp_oauth/authorization] callback timed out")]
    CallbackTimeout,
    /// Authorization server returned a typed callback error.
    #[error("[ai_mcp_oauth/authorization] callback returned {error:?}")]
    AuthorizationRejected {
        /// Standard authorization error.
        error: OAuthAuthorizationError,
    },
    /// Authorization callback omitted its state value.
    #[error("[ai_mcp_oauth/state] callback state is missing")]
    StateMissing,
    /// Authorization callback arrived after state expiry.
    #[error("[ai_mcp_oauth/state] callback state expired")]
    StateExpired,
    /// Authorization state was consumed previously.
    #[error("[ai_mcp_oauth/state] callback state was already used")]
    StateReused,
    /// Authorization callback state did not match the request.
    #[error("[ai_mcp_oauth/state] callback state mismatch")]
    StateMismatch,
    /// Secure randomness produced a duplicate live authorization state.
    #[error("[ai_mcp_oauth/state] duplicate authorization state")]
    StateCollision,
    /// Token endpoint returned a typed OAuth failure.
    #[error("[ai_mcp_oauth/token] token request rejected with HTTP {status}: {error:?}")]
    TokenRejected {
        /// HTTP response status.
        status: u16,
        /// Standard token error.
        error: OAuthTokenError,
    },
    /// Token endpoint response was malformed.
    #[error("[ai_mcp_oauth/token] invalid token response: {source}")]
    TokenSchema {
        /// Underlying JSON decode failure.
        source: serde_json::Error,
    },
    /// Refresh token is no longer accepted by the authorization server.
    #[error("[ai_mcp_oauth/token] refresh token is invalid")]
    InvalidGrant,
    /// User interaction is required before a usable token can be obtained.
    #[error("[ai_mcp_oauth/token] user interaction required")]
    InteractionRequired,
    /// Auth hook and credential key name different protected resources.
    #[error("[ai_mcp_oauth/auth_hook] credential resource mismatch")]
    CredentialResourceMismatch,
    /// Best-effort token revocation failed after local credentials were removed.
    #[error("[ai_mcp_oauth/disconnect] remote revocation failed")]
    RevocationFailed,
    /// Local credential deletion failed during disconnect.
    #[error(
        "[ai_mcp_oauth/disconnect] local token deletion failed (revocation_failed={revocation_failed})"
    )]
    LocalTokenDeletionFailed {
        /// Whether remote revocation also failed.
        revocation_failed: bool,
    },
    /// Required host authorization context is empty or inconsistent.
    #[error("[ai_mcp_oauth/authorization] invalid host authorization context")]
    InvalidAuthorizationContext,
    /// Successful callback omitted a usable authorization code.
    #[error("[ai_mcp_oauth/authorization] callback code is missing")]
    AuthorizationCodeMissing,
}

/// Result alias for MCP OAuth operations.
pub type Result<T> = std::result::Result<T, Error>;
