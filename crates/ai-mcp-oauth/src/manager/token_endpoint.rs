//! Secret-safe token endpoint request and response handling.

use secrecy::SecretString;
use serde::Deserialize;

use crate::{
    AuthorizationServerMetadata, Error, OAuthEndpointKind, OAuthHttpResponse, OAuthScopes,
    OAuthTokenError, OAuthTokenSet, OAuthTokenType, Result,
};

use super::DefaultMcpOAuthManager;

impl DefaultMcpOAuthManager {
    pub(super) async fn request_tokens(
        &self,
        server: &AuthorizationServerMetadata,
        fields: &[(String, String)],
        fallback_scopes: &OAuthScopes,
        previous_refresh: Option<SecretString>,
    ) -> Result<OAuthTokenSet> {
        let response = self
            .transport
            .post_form(
                &server.token_endpoint,
                OAuthEndpointKind::Token,
                &self.config.url_policy,
                self.config.http_limits(),
                fields,
            )
            .await?;
        if !(200..300).contains(&response.status) {
            return parse_token_response(response, 0, fallback_scopes, previous_refresh);
        }
        let now = self.clock.now_unix_seconds()?;
        parse_token_response(response, now, fallback_scopes, previous_refresh)
    }
}

#[derive(Deserialize)]
struct TokenSuccessWire {
    access_token: String,
    token_type: String,
    refresh_token: Option<String>,
    expires_in: Option<u64>,
    scope: Option<String>,
}

#[derive(Deserialize)]
struct TokenErrorWire {
    error: Option<String>,
}

fn parse_token_response(
    response: OAuthHttpResponse,
    now: u64,
    fallback_scopes: &OAuthScopes,
    previous_refresh: Option<SecretString>,
) -> Result<OAuthTokenSet> {
    if !(200..300).contains(&response.status) {
        let error = decode_token_error(response.body);
        if error == OAuthTokenError::InvalidGrant {
            return Err(Error::InvalidGrant);
        }
        return Err(Error::TokenRejected {
            status: response.status,
            error,
        });
    }
    let wire: TokenSuccessWire = match serde_json::from_value(response.body) {
        Ok(wire) => wire,
        Err(source) => return Err(Error::TokenSchema { source }),
    };
    if wire.access_token.is_empty() || !wire.token_type.eq_ignore_ascii_case("bearer") {
        return Err(Error::TokenSchema {
            source: schema_error("token_type must be Bearer and access_token must be non-empty"),
        });
    }
    let scopes = wire
        .scope
        .as_deref()
        .map(OAuthScopes::parse)
        .unwrap_or_else(|| fallback_scopes.clone());
    let refresh_token = wire
        .refresh_token
        .map(SecretString::from)
        .or(previous_refresh);
    Ok(OAuthTokenSet {
        access_token: SecretString::from(wire.access_token),
        refresh_token,
        token_type: OAuthTokenType::Bearer,
        expires_at: wire.expires_in.map(|seconds| now.saturating_add(seconds)),
        scopes,
    })
}

fn decode_token_error(body: serde_json::Value) -> OAuthTokenError {
    let wire: TokenErrorWire = match serde_json::from_value(body) {
        Ok(wire) => wire,
        Err(_) => return OAuthTokenError::Unrecognized,
    };
    match wire.error.as_deref() {
        Some("invalid_request") => OAuthTokenError::InvalidRequest,
        Some("invalid_client") => OAuthTokenError::InvalidClient,
        Some("invalid_grant") => OAuthTokenError::InvalidGrant,
        Some("unsupported_grant_type") => OAuthTokenError::UnsupportedGrantType,
        Some("invalid_scope") => OAuthTokenError::InvalidScope,
        _ => OAuthTokenError::Unrecognized,
    }
}

fn schema_error(message: &'static str) -> serde_json::Error {
    <serde_json::Error as serde::de::Error>::custom(message)
}

#[cfg(test)]
#[path = "../_tests_/manager/token_endpoint_tests.rs"]
mod token_endpoint_tests;
