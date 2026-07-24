//! Secret-safe OAuth token types.

use secrecy::SecretString;

use crate::OAuthScopes;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
/// Supported OAuth access-token type.
pub enum OAuthTokenType {
    /// RFC 6750 Bearer access token.
    Bearer,
}

#[derive(Clone)]
/// Stored OAuth credentials and their granted scope set.
pub struct OAuthTokenSet {
    /// Secret Bearer access token.
    pub access_token: SecretString,
    /// Optional secret refresh token.
    pub refresh_token: Option<SecretString>,
    /// Required access-token type.
    pub token_type: OAuthTokenType,
    /// Absolute UNIX expiry time, or `None` when the server omitted expiry.
    pub expires_at: Option<u64>,
    /// Granted scopes retained for refresh and incremental consent.
    pub scopes: OAuthScopes,
}

impl std::fmt::Debug for OAuthTokenSet {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("OAuthTokenSet")
            .field("access_token", &"[REDACTED]")
            .field(
                "refresh_token",
                &self.refresh_token.as_ref().map(|_| "[REDACTED]"),
            )
            .field("token_type", &self.token_type)
            .field("expires_at", &self.expires_at)
            .field("scopes", &self.scopes)
            .finish()
    }
}

impl OAuthTokenSet {
    /// Returns whether the token can be used beyond the supplied refresh skew.
    pub fn is_fresh_at(&self, now: u64, refresh_skew_seconds: u64) -> bool {
        self.expires_at
            .is_none_or(|expiry| expiry.saturating_sub(refresh_skew_seconds) > now)
    }
}
