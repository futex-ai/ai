//! Pure OAuth client configuration and defaults.

use std::time::Duration;

use crate::{Error, OAuthConfigField, OAuthHttpLimits, OAuthUrlPolicy, Result};

const DEFAULT_HTTP_TIMEOUT: Duration = Duration::from_secs(30);
const DEFAULT_USER_AGENT_TIMEOUT: Duration = Duration::from_secs(10 * 60);
const DEFAULT_STATE_LIFETIME: Duration = Duration::from_secs(10 * 60);
const DEFAULT_MAX_RESPONSE_BYTES: usize = 1024 * 1024;
const DEFAULT_MAX_REDIRECTS: usize = 3;
const DEFAULT_METADATA_CACHE_AGE: Duration = Duration::from_secs(60 * 60);
const DEFAULT_REFRESH_SKEW: Duration = Duration::from_secs(60);

#[derive(Clone, Debug, Eq, PartialEq)]
/// Host-independent limits and policy for MCP OAuth operations.
pub struct McpOAuthConfig {
    /// Timeout for discovery, registration, token, and revocation requests.
    pub http_timeout: Duration,
    /// Maximum time allowed for one explicit user-agent authorization.
    pub user_agent_timeout: Duration,
    /// Maximum valid lifetime of one authorization state value.
    pub state_lifetime: Duration,
    /// Maximum response bytes accepted from one OAuth endpoint.
    pub max_response_bytes: usize,
    /// Maximum number of manually validated HTTP redirects.
    pub max_redirects: usize,
    /// Upper bound for cached discovery metadata.
    pub max_metadata_cache_age: Duration,
    /// Time before expiry at which an access token should be refreshed.
    pub refresh_skew: Duration,
    /// URL and destination policy applied to every OAuth endpoint.
    pub url_policy: OAuthUrlPolicy,
}

impl Default for McpOAuthConfig {
    fn default() -> Self {
        Self {
            http_timeout: DEFAULT_HTTP_TIMEOUT,
            user_agent_timeout: DEFAULT_USER_AGENT_TIMEOUT,
            state_lifetime: DEFAULT_STATE_LIFETIME,
            max_response_bytes: DEFAULT_MAX_RESPONSE_BYTES,
            max_redirects: DEFAULT_MAX_REDIRECTS,
            max_metadata_cache_age: DEFAULT_METADATA_CACHE_AGE,
            refresh_skew: DEFAULT_REFRESH_SKEW,
            url_policy: OAuthUrlPolicy::default(),
        }
    }
}

impl McpOAuthConfig {
    /// Validates all positive bounds before side effects occur.
    pub fn validate(&self) -> Result<()> {
        if self.http_timeout.is_zero() {
            return Err(Error::InvalidConfig {
                field: OAuthConfigField::HttpTimeout,
            });
        }
        if self.user_agent_timeout.is_zero() {
            return Err(Error::InvalidConfig {
                field: OAuthConfigField::UserAgentTimeout,
            });
        }
        if self.state_lifetime.is_zero() {
            return Err(Error::InvalidConfig {
                field: OAuthConfigField::StateLifetime,
            });
        }
        if self.max_response_bytes == 0 {
            return Err(Error::InvalidConfig {
                field: OAuthConfigField::ResponseLimit,
            });
        }
        if self.max_redirects == 0 {
            return Err(Error::InvalidConfig {
                field: OAuthConfigField::RedirectLimit,
            });
        }
        if self.max_metadata_cache_age.is_zero() {
            return Err(Error::InvalidConfig {
                field: OAuthConfigField::MetadataCacheAge,
            });
        }
        if self.refresh_skew.is_zero() {
            return Err(Error::InvalidConfig {
                field: OAuthConfigField::RefreshSkew,
            });
        }
        Ok(())
    }

    pub(crate) fn http_limits(&self) -> OAuthHttpLimits {
        OAuthHttpLimits {
            timeout: self.http_timeout,
            max_response_bytes: self.max_response_bytes,
            max_redirects: self.max_redirects,
        }
    }
}
