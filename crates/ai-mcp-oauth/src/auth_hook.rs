//! Resource-bound non-interactive MCP request authentication.

use std::{collections::BTreeMap, sync::Arc};

use async_trait::async_trait;
use json_http::JsonHttpAuth;
use secrecy::ExposeSecret;

use crate::{
    CanonicalMcpResource, DynOAuthRequestTokenProvider, Error, OAuthCredentialKey, Result,
};

/// OAuth auth hook that loads or refreshes one exact resource credential.
pub struct RefreshingMcpAuth {
    resource: CanonicalMcpResource,
    key: OAuthCredentialKey,
    provider: DynOAuthRequestTokenProvider,
}

impl RefreshingMcpAuth {
    /// Binds one token provider to an exact MCP resource and credential key.
    pub fn new(
        resource: CanonicalMcpResource,
        key: OAuthCredentialKey,
        provider: DynOAuthRequestTokenProvider,
    ) -> Result<Self> {
        if key.resource != resource {
            return Err(Error::CredentialResourceMismatch);
        }
        Ok(Self {
            resource,
            key,
            provider,
        })
    }

    /// Returns the exact canonical MCP resource authenticated by this hook.
    pub fn resource(&self) -> &CanonicalMcpResource {
        &self.resource
    }
}

#[async_trait]
impl JsonHttpAuth for RefreshingMcpAuth {
    async fn apply_headers(&self, headers: &mut BTreeMap<String, String>) -> json_http::Result<()> {
        let token = match self.provider.token_for_request(&self.key).await {
            Ok(token) => token,
            Err(_) => {
                return Err(json_http::Error::auth(
                    "MCP OAuth credential refresh failed",
                ));
            }
        };
        if let Some(token) = token {
            headers.insert(
                "Authorization".to_owned(),
                format!("Bearer {}", token.expose_secret()),
            );
        }
        Ok(())
    }
}

impl From<RefreshingMcpAuth> for Arc<dyn JsonHttpAuth> {
    fn from(auth: RefreshingMcpAuth) -> Self {
        Arc::new(auth)
    }
}
