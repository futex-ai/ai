//! Concurrent non-interactive token refresh.

use async_trait::async_trait;
use secrecy::{ExposeSecret, SecretString};
use sha2::{Digest, Sha256};

use crate::{Error, OAuthCredentialKey, OAuthRequestTokenProvider, OAuthTokenSet, Result};

use super::{DefaultMcpOAuthManager, OAuthConnection};

impl DefaultMcpOAuthManager {
    pub(super) async fn explicit_refresh(
        &self,
        key: &OAuthCredentialKey,
    ) -> Result<OAuthConnection> {
        let Some(initial) = self.store.load_tokens(key).await? else {
            return Err(Error::InteractionRequired);
        };
        let initial_fingerprint = token_fingerprint(&initial);
        let lock = self.refresh_lock(key).await;
        let _guard = lock.lock().await;
        let Some(current) = self.store.load_tokens(key).await? else {
            return Err(Error::InteractionRequired);
        };
        if token_fingerprint(&current) != initial_fingerprint {
            return Ok(connection(key, current));
        }
        match self.refresh_current(key, current).await {
            Ok(tokens) => Ok(connection(key, tokens)),
            Err(Error::InvalidGrant) => {
                self.store.delete_tokens(key).await?;
                Err(Error::InteractionRequired)
            }
            Err(error) => Err(error),
        }
    }

    async fn refresh_current(
        &self,
        key: &OAuthCredentialKey,
        current: OAuthTokenSet,
    ) -> Result<OAuthTokenSet> {
        let Some(refresh_token) = current.refresh_token.clone() else {
            return Err(Error::InteractionRequired);
        };
        let server = self.discovery.authorization_server(&key.issuer).await?;
        let fields = vec![
            ("grant_type".to_owned(), "refresh_token".to_owned()),
            (
                "refresh_token".to_owned(),
                refresh_token.expose_secret().to_owned(),
            ),
            ("client_id".to_owned(), key.client_id.clone()),
            ("resource".to_owned(), key.resource.to_string()),
        ];
        let refreshed = self
            .request_tokens(&server, &fields, &current.scopes, Some(refresh_token))
            .await?;
        self.store.save_tokens(key, &refreshed).await?;
        Ok(refreshed)
    }

    async fn request_token_inner(&self, key: &OAuthCredentialKey) -> Result<Option<SecretString>> {
        let Some(initial) = self.store.load_tokens(key).await? else {
            return Ok(None);
        };
        let now = self.clock.now_unix_seconds()?;
        if initial.is_fresh_at(now, self.config.refresh_skew.as_secs()) {
            return Ok(Some(initial.access_token));
        }
        let initial_fingerprint = token_fingerprint(&initial);
        let lock = self.refresh_lock(key).await;
        let _guard = lock.lock().await;
        let Some(current) = self.store.load_tokens(key).await? else {
            return Ok(None);
        };
        let now = self.clock.now_unix_seconds()?;
        if token_fingerprint(&current) != initial_fingerprint
            || current.is_fresh_at(now, self.config.refresh_skew.as_secs())
        {
            return usable_access_token(current, now);
        }
        if current.refresh_token.is_none() {
            return Ok(None);
        }
        match self.refresh_current(key, current).await {
            Ok(refreshed) => {
                let now = self.clock.now_unix_seconds()?;
                usable_access_token(refreshed, now)
            }
            Err(Error::InvalidGrant) => {
                self.store.delete_tokens(key).await?;
                Ok(None)
            }
            Err(error) => Err(error),
        }
    }
}

#[async_trait]
impl OAuthRequestTokenProvider for DefaultMcpOAuthManager {
    async fn token_for_request(&self, key: &OAuthCredentialKey) -> Result<Option<SecretString>> {
        self.request_token_inner(key).await
    }
}

fn connection(key: &OAuthCredentialKey, tokens: OAuthTokenSet) -> OAuthConnection {
    OAuthConnection {
        key: key.clone(),
        scopes: tokens.scopes,
        expires_at: tokens.expires_at,
    }
}

fn usable_access_token(tokens: OAuthTokenSet, now: u64) -> Result<Option<SecretString>> {
    if tokens.is_expired_at(now) {
        return Ok(None);
    }
    Ok(Some(tokens.access_token))
}

fn token_fingerprint(tokens: &OAuthTokenSet) -> [u8; 32] {
    let mut digest = Sha256::new();
    let access = tokens.access_token.expose_secret().as_bytes();
    digest.update(access.len().to_be_bytes());
    digest.update(access);
    if let Some(refresh) = &tokens.refresh_token {
        digest.update([1]);
        let refresh = refresh.expose_secret().as_bytes();
        digest.update(refresh.len().to_be_bytes());
        digest.update(refresh);
    } else {
        digest.update([0]);
    }
    if let Some(expires_at) = tokens.expires_at {
        digest.update([1]);
        digest.update(expires_at.to_be_bytes());
    } else {
        digest.update([0]);
    }
    for scope in tokens.scopes.as_slice() {
        digest.update(scope.len().to_be_bytes());
        digest.update(scope.as_bytes());
    }
    digest.finalize().into()
}
