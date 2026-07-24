//! Explicit PKCE authorization orchestration.

use ai_mcp::{McpAuthorizationChallenge, McpAuthorizationFailure};
use secrecy::ExposeSecret;
use url::Url;

use crate::{
    Error, OAuthAuthorizationError, OAuthAuthorizationResponse, OAuthCredentialKey,
    OAuthEndpointKind, OAuthRegistrationRequest, OAuthScopes, OAuthUserAuthorizationRequest,
    Result, pkce::generate_authorization_secrets,
};

use super::{DefaultMcpOAuthManager, DeniedPromptKey, OAuthAuthorizationContext, OAuthConnection};

impl DefaultMcpOAuthManager {
    pub(super) async fn authorize_inner(
        &self,
        challenge: &McpAuthorizationChallenge,
        context: &OAuthAuthorizationContext,
    ) -> Result<OAuthConnection> {
        validate_context(context)?;
        if challenge.failure == McpAuthorizationFailure::Forbidden {
            return Err(Error::AuthorizationForbidden);
        }
        let requested_scopes = context
            .baseline_scopes
            .union(&OAuthScopes::new(challenge.scopes.clone()));
        let denied_key = denied_key(context, &requested_scopes);
        if challenge.failure == McpAuthorizationFailure::InsufficientScope
            && self.denied_prompts.lock().await.contains(&denied_key)
        {
            return Err(Error::UserDenied);
        }
        let discovered = self
            .discovery
            .discover(&context.resource, challenge)
            .await?;
        if !discovered.authorization_server.supports_s256() {
            return Err(Error::PkceS256Unsupported);
        }
        let registration = self
            .registry
            .resolve(&OAuthRegistrationRequest {
                server: discovered.authorization_server.clone(),
                redirect_uri: context.redirect_uri.clone(),
                client_name: context.client_name.clone(),
                configured: context.configured_registration.clone(),
            })
            .await?;
        let secrets = generate_authorization_secrets(self.random.as_ref())?;
        let started_at = self.clock.now_unix_seconds()?;
        let state_lifetime = self.config.state_lifetime.as_secs();
        let state_handle = self
            .states
            .begin(&secrets.state, started_at, state_lifetime)
            .await?;
        let authorization_url = build_authorization_url(
            &discovered.authorization_server.authorization_endpoint,
            &registration.client_id,
            &registration.redirect_uri,
            context.resource.as_str(),
            &requested_scopes,
            secrets.state.expose_secret(),
            &secrets.challenge,
            &self.config,
        )?;
        let response = tokio::time::timeout(
            self.config.user_agent_timeout,
            self.user_agent
                .authorize(OAuthUserAuthorizationRequest::new(
                    authorization_url,
                    started_at.saturating_add(state_lifetime),
                )),
        )
        .await;
        let response = match response {
            Ok(Ok(response)) => response,
            Ok(Err(error)) => {
                self.states.invalidate(&state_handle).await;
                if matches!(error, Error::UserDenied)
                    && challenge.failure == McpAuthorizationFailure::InsufficientScope
                {
                    self.denied_prompts.lock().await.insert(denied_key);
                }
                return Err(error);
            }
            Err(_) => {
                self.states.invalidate(&state_handle).await;
                return Err(Error::CallbackTimeout);
            }
        };
        let code = match response {
            OAuthAuthorizationResponse::Authorized { code, state } => {
                let callback_at = self.clock.now_unix_seconds()?;
                self.states
                    .consume(&state_handle, &secrets.state, state.as_ref(), callback_at)
                    .await?;
                if code.expose_secret().is_empty() {
                    return Err(Error::AuthorizationCodeMissing);
                }
                code
            }
            OAuthAuthorizationResponse::OAuthError {
                error: OAuthAuthorizationError::AccessDenied,
            } => {
                self.states.invalidate(&state_handle).await;
                if challenge.failure == McpAuthorizationFailure::InsufficientScope {
                    self.denied_prompts.lock().await.insert(denied_key);
                }
                return Err(Error::UserDenied);
            }
            OAuthAuthorizationResponse::OAuthError { error } => {
                self.states.invalidate(&state_handle).await;
                return Err(Error::AuthorizationRejected { error });
            }
            OAuthAuthorizationResponse::Cancelled => {
                self.states.invalidate(&state_handle).await;
                return Err(Error::UserCancelled);
            }
        };
        let fields = vec![
            ("grant_type".to_owned(), "authorization_code".to_owned()),
            ("code".to_owned(), code.expose_secret().to_owned()),
            ("client_id".to_owned(), registration.client_id.clone()),
            ("redirect_uri".to_owned(), registration.redirect_uri.clone()),
            (
                "code_verifier".to_owned(),
                secrets.verifier.expose_secret().to_owned(),
            ),
            ("resource".to_owned(), context.resource.to_string()),
        ];
        let tokens = self
            .request_tokens(
                &discovered.authorization_server,
                &fields,
                &requested_scopes,
                None,
            )
            .await?;
        let key = OAuthCredentialKey {
            account_id: context.account_id.clone(),
            resource: context.resource.clone(),
            issuer: discovered.authorization_server.issuer,
            client_id: registration.client_id,
            redirect_uri: registration.redirect_uri,
        };
        self.store.save_tokens(&key, &tokens).await?;
        self.denied_prompts.lock().await.remove(&denied_key);
        Ok(OAuthConnection {
            key,
            scopes: tokens.scopes,
            expires_at: tokens.expires_at,
        })
    }
}

fn validate_context(context: &OAuthAuthorizationContext) -> Result<()> {
    if context.account_id.is_empty()
        || context.redirect_uri.is_empty()
        || context.client_name.is_empty()
        || context.authorization_attempt_id.is_empty()
    {
        return Err(Error::InvalidAuthorizationContext);
    }
    Ok(())
}

fn denied_key(context: &OAuthAuthorizationContext, scopes: &OAuthScopes) -> DeniedPromptKey {
    DeniedPromptKey {
        attempt_id: context.authorization_attempt_id.clone(),
        account_id: context.account_id.clone(),
        resource: context.resource.clone(),
        scopes: scopes.clone(),
    }
}

#[expect(
    clippy::too_many_arguments,
    reason = "each OAuth authorization parameter is an independent protocol value"
)]
fn build_authorization_url(
    endpoint: &str,
    client_id: &str,
    redirect_uri: &str,
    resource: &str,
    scopes: &OAuthScopes,
    state: &str,
    challenge: &str,
    config: &crate::McpOAuthConfig,
) -> Result<String> {
    let mut url = config
        .url_policy
        .parse(endpoint, OAuthEndpointKind::Authorization)?;
    reject_reserved_parameters(&url)?;
    {
        let mut query = url.query_pairs_mut();
        query
            .append_pair("response_type", "code")
            .append_pair("client_id", client_id)
            .append_pair("redirect_uri", redirect_uri)
            .append_pair("code_challenge", challenge)
            .append_pair("code_challenge_method", "S256")
            .append_pair("resource", resource)
            .append_pair("state", state);
        if !scopes.is_empty() {
            query.append_pair("scope", &scopes.to_parameter());
        }
    }
    Ok(url.to_string())
}

fn reject_reserved_parameters(url: &Url) -> Result<()> {
    const RESERVED: [&str; 8] = [
        "response_type",
        "client_id",
        "redirect_uri",
        "code_challenge",
        "code_challenge_method",
        "resource",
        "state",
        "scope",
    ];
    if url
        .query_pairs()
        .any(|(name, _)| RESERVED.contains(&name.as_ref()))
    {
        return Err(Error::InvalidUrl {
            endpoint: OAuthEndpointKind::Authorization,
        });
    }
    Ok(())
}
