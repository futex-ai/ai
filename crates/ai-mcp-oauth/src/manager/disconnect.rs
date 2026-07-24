//! Best-effort RFC 7009 revocation followed by local deletion.

use secrecy::ExposeSecret;

use crate::{Error, OAuthCredentialKey, OAuthEndpointKind, OAuthTokenSet, Result};

use super::DefaultMcpOAuthManager;

impl DefaultMcpOAuthManager {
    pub(super) async fn disconnect_inner(&self, key: &OAuthCredentialKey) -> Result<()> {
        let tokens = self.store.load_tokens(key).await?;
        let revocation_failed = if let Some(tokens) = tokens {
            self.revoke_if_advertised(key, &tokens).await.is_err()
        } else {
            false
        };
        if self.store.delete_tokens(key).await.is_err() {
            return Err(Error::LocalTokenDeletionFailed { revocation_failed });
        }
        if revocation_failed {
            return Err(Error::RevocationFailed);
        }
        Ok(())
    }

    async fn revoke_if_advertised(
        &self,
        key: &OAuthCredentialKey,
        tokens: &OAuthTokenSet,
    ) -> Result<()> {
        let server = self.discovery.authorization_server(&key.issuer).await?;
        let Some(endpoint) = server.revocation_endpoint else {
            return Ok(());
        };
        let (token, hint) = if let Some(refresh_token) = &tokens.refresh_token {
            (refresh_token.expose_secret(), "refresh_token")
        } else {
            (tokens.access_token.expose_secret(), "access_token")
        };
        let response = self
            .transport
            .post_form(
                &endpoint,
                OAuthEndpointKind::Revocation,
                &self.config.url_policy,
                self.config.http_limits(),
                &[
                    ("token".to_owned(), token.to_owned()),
                    ("token_type_hint".to_owned(), hint.to_owned()),
                    ("client_id".to_owned(), key.client_id.clone()),
                ],
            )
            .await?;
        if !(200..300).contains(&response.status) {
            return Err(Error::RevocationFailed);
        }
        Ok(())
    }
}
