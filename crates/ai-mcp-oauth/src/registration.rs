//! Configured, cached, and RFC 7591 public-client registration.

use std::sync::Arc;

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::{
    AuthorizationServerMetadata, DynOAuthCredentialStore, DynOAuthHttpTransport, Error,
    McpOAuthConfig, OAuthClientRegistration, OAuthClientRegistrationSource, OAuthEndpointKind,
    OAuthRegistrationKey, Result,
};

/// Shared public-client registration resolver.
pub type DynOAuthClientRegistry = Arc<dyn OAuthClientRegistry>;

#[derive(Clone, Debug)]
/// Inputs needed to resolve one exact public-client registration.
pub struct OAuthRegistrationRequest {
    /// Validated authorization-server metadata.
    pub server: AuthorizationServerMetadata,
    /// Host-approved callback URI.
    pub redirect_uri: String,
    /// Stable public client name.
    pub client_name: String,
    /// Optional host-configured registration for this issuer.
    pub configured: Option<OAuthClientRegistration>,
}

impl OAuthRegistrationRequest {
    /// Returns the exact local registration cache key.
    pub fn key(&self) -> OAuthRegistrationKey {
        OAuthRegistrationKey {
            issuer: self.server.issuer.clone(),
            redirect_uri: self.redirect_uri.clone(),
            client_name: self.client_name.clone(),
        }
    }
}

#[cfg_attr(
    any(test, doctest, feature = "test-support"),
    unimock::unimock(api = OAuthClientRegistryMock)
)]
#[async_trait]
/// Resolves a host-configured, cached, or dynamically registered public client.
pub trait OAuthClientRegistry: Send + Sync {
    /// Resolves a public client before interactive authorization begins.
    async fn resolve(&self, request: &OAuthRegistrationRequest) -> Result<OAuthClientRegistration>;
}

/// Default public-client resolver over injected transport and secure storage.
pub struct DefaultOAuthClientRegistry {
    transport: DynOAuthHttpTransport,
    store: DynOAuthCredentialStore,
    config: McpOAuthConfig,
}

impl DefaultOAuthClientRegistry {
    /// Builds a validated public-client resolver.
    pub fn new(
        transport: DynOAuthHttpTransport,
        store: DynOAuthCredentialStore,
        config: McpOAuthConfig,
    ) -> Result<Self> {
        config.validate()?;
        Ok(Self {
            transport,
            store,
            config,
        })
    }

    async fn dynamically_register(
        &self,
        request: &OAuthRegistrationRequest,
        endpoint: &str,
    ) -> Result<OAuthClientRegistration> {
        let mut grant_types = vec!["authorization_code"];
        if request.server.supports_refresh_tokens() {
            grant_types.push("refresh_token");
        }
        let body = json!({
            "redirect_uris": [&request.redirect_uri],
            "response_types": ["code"],
            "grant_types": grant_types,
            "token_endpoint_auth_method": "none",
            "client_name": &request.client_name,
        });
        let response = self
            .transport
            .post_json(
                endpoint,
                OAuthEndpointKind::Registration,
                &self.config.url_policy,
                self.config.http_limits(),
                &body,
            )
            .await?;
        if !matches!(response.status, 200 | 201) {
            return Err(Error::RegistrationRejected {
                status: response.status,
                body: sanitized_error_body(response.body),
            });
        }
        let wire: RegistrationResponseWire = match serde_json::from_value(response.body) {
            Ok(wire) => wire,
            Err(source) => return Err(Error::RegistrationSchema { source }),
        };
        if wire.client_id.is_empty() {
            return Err(Error::RegistrationSchema {
                source: schema_error("client_id must not be empty"),
            });
        }
        let registration = OAuthClientRegistration {
            client_id: wire.client_id,
            redirect_uri: request.redirect_uri.clone(),
            client_name: request.client_name.clone(),
            source: OAuthClientRegistrationSource::Dynamic,
        };
        self.store
            .save_registration(&request.key(), &registration)
            .await?;
        Ok(registration)
    }
}

#[async_trait]
impl OAuthClientRegistry for DefaultOAuthClientRegistry {
    async fn resolve(&self, request: &OAuthRegistrationRequest) -> Result<OAuthClientRegistration> {
        self.config
            .url_policy
            .parse(&request.redirect_uri, OAuthEndpointKind::Redirect)?;
        if !request.server.supports_public_clients() {
            return Err(Error::PublicClientUnsupported);
        }
        if let Some(configured) = &request.configured {
            if configured.redirect_uri != request.redirect_uri
                || configured.client_name != request.client_name
                || configured.client_id.is_empty()
            {
                return Err(Error::RegistrationMismatch);
            }
            let mut configured = configured.clone();
            configured.source = OAuthClientRegistrationSource::Configured;
            return Ok(configured);
        }
        let key = request.key();
        if let Some(cached) = self.store.load_registration(&key).await? {
            if cached.redirect_uri != request.redirect_uri
                || cached.client_name != request.client_name
            {
                return Err(Error::RegistrationMismatch);
            }
            return Ok(cached);
        }
        let Some(endpoint) = request.server.registration_endpoint.as_deref() else {
            return Err(Error::ClientRegistrationRequired {
                issuer: request.server.issuer.clone(),
                redirect_uri: request.redirect_uri.clone(),
            });
        };
        self.dynamically_register(request, endpoint).await
    }
}

#[derive(Deserialize)]
struct RegistrationResponseWire {
    client_id: String,
    #[serde(default)]
    #[expect(
        dead_code,
        reason = "public clients deliberately ignore returned secrets"
    )]
    client_secret: Option<String>,
}

fn schema_error(message: &'static str) -> serde_json::Error {
    <serde_json::Error as serde::de::Error>::custom(message)
}

fn sanitized_error_body(mut body: Value) -> Value {
    redact_secret_fields(&mut body);
    body
}

fn redact_secret_fields(value: &mut Value) {
    match value {
        Value::Object(object) => {
            for (name, value) in object {
                if is_secret_field(name) {
                    *value = Value::String("[REDACTED]".to_owned());
                } else {
                    redact_secret_fields(value);
                }
            }
        }
        Value::Array(values) => {
            for value in values {
                redact_secret_fields(value);
            }
        }
        _ => {}
    }
}

fn is_secret_field(name: &str) -> bool {
    [
        "access_token",
        "refresh_token",
        "client_secret",
        "client_assertion",
        "code",
        "code_verifier",
        "state",
    ]
    .iter()
    .any(|secret| name.eq_ignore_ascii_case(secret))
}
