//! Host-owned external user-agent authorization boundary.

use std::sync::Arc;

use async_trait::async_trait;
use secrecy::{ExposeSecret, SecretString};

use crate::{OAuthAuthorizationError, Result};

/// Shared external OAuth user agent.
pub type DynOAuthUserAgent = Arc<dyn OAuthUserAgent>;

/// One validated browser authorization request supplied to the host.
pub struct OAuthUserAuthorizationRequest {
    authorization_url: SecretString,
    expires_at: u64,
}

impl OAuthUserAuthorizationRequest {
    pub(crate) fn new(authorization_url: String, expires_at: u64) -> Self {
        Self {
            authorization_url: SecretString::from(authorization_url),
            expires_at,
        }
    }

    /// Returns the validated URL the host should open with platform APIs.
    pub fn authorization_url(&self) -> &str {
        self.authorization_url.expose_secret()
    }

    /// Returns the UNIX deadline for this one authorization interaction.
    pub fn expires_at(&self) -> u64 {
        self.expires_at
    }
}

impl std::fmt::Debug for OAuthUserAuthorizationRequest {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("OAuthUserAuthorizationRequest")
            .field("authorization_url", &"[REDACTED]")
            .field("expires_at", &self.expires_at)
            .finish()
    }
}

/// Typed result returned by a host's callback or cancellation UI.
pub enum OAuthAuthorizationResponse {
    /// Authorization server returned a code and optional state value.
    Authorized {
        /// Secret one-time authorization code.
        code: SecretString,
        /// Secret callback state, or `None` when omitted.
        state: Option<SecretString>,
    },
    /// Authorization server returned a standard OAuth error.
    OAuthError {
        /// Typed callback error.
        error: OAuthAuthorizationError,
    },
    /// User or host cancelled before an OAuth callback completed.
    Cancelled,
}

impl OAuthAuthorizationResponse {
    /// Builds a successful callback response while wrapping secrets safely.
    pub fn authorized(code: impl Into<String>, state: Option<impl Into<String>>) -> Self {
        Self::Authorized {
            code: SecretString::from(code.into()),
            state: state.map(|state| SecretString::from(state.into())),
        }
    }
}

impl std::fmt::Debug for OAuthAuthorizationResponse {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Authorized { state, .. } => formatter
                .debug_struct("Authorized")
                .field("code", &"[REDACTED]")
                .field("state", &state.as_ref().map(|_| "[REDACTED]"))
                .finish(),
            Self::OAuthError { error } => formatter
                .debug_struct("OAuthError")
                .field("error", error)
                .finish(),
            Self::Cancelled => formatter.write_str("Cancelled"),
        }
    }
}

#[cfg_attr(
    any(test, doctest, feature = "test-support"),
    unimock::unimock(api = OAuthUserAgentMock)
)]
#[async_trait]
/// Opens a validated authorization URL and returns a typed callback result.
pub trait OAuthUserAgent: Send + Sync {
    /// Performs one explicit host-approved external user-agent interaction.
    async fn authorize(
        &self,
        request: OAuthUserAuthorizationRequest,
    ) -> Result<OAuthAuthorizationResponse>;
}
