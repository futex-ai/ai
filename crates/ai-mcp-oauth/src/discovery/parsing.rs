//! Strict wire parsing and endpoint validation for OAuth discovery.

use std::collections::BTreeMap;

use serde::Deserialize;
use serde_json::Value;
use url::Url;

use crate::{
    AuthorizationServerMetadata, CanonicalMcpResource, Error, McpOAuthConfig, OAuthEndpointKind,
    OAuthHttpResponse, OAuthScopes, ProtectedResourceMetadata, Result,
};

#[derive(Deserialize)]
struct ProtectedResourceWire {
    resource: String,
    #[serde(default)]
    authorization_servers: Vec<String>,
    #[serde(default)]
    scopes_supported: Vec<String>,
    #[serde(flatten)]
    unknown: BTreeMap<String, Value>,
}

#[derive(Deserialize)]
struct AuthorizationServerWire {
    issuer: String,
    authorization_endpoint: Option<String>,
    token_endpoint: Option<String>,
    registration_endpoint: Option<String>,
    revocation_endpoint: Option<String>,
    #[serde(default)]
    grant_types_supported: Vec<String>,
    #[serde(default)]
    token_endpoint_auth_methods_supported: Vec<String>,
    #[serde(default)]
    code_challenge_methods_supported: Vec<String>,
    #[serde(default)]
    scopes_supported: Vec<String>,
    #[serde(flatten)]
    unknown: BTreeMap<String, Value>,
}

pub(super) fn parse_protected_resource(
    resource: &CanonicalMcpResource,
    body: Value,
) -> Result<ProtectedResourceMetadata> {
    let wire: ProtectedResourceWire =
        decode_metadata(body, OAuthEndpointKind::ProtectedResourceMetadata)?;
    if wire.resource != resource.as_str() {
        return Err(Error::ResourceMismatch {
            expected: resource.to_string(),
            actual: wire.resource,
        });
    }
    if wire.authorization_servers.is_empty() {
        return Err(Error::MissingAuthorizationServer);
    }
    Ok(ProtectedResourceMetadata {
        resource: wire.resource,
        authorization_servers: wire.authorization_servers,
        scopes_supported: OAuthScopes::new(wire.scopes_supported),
        unknown: wire.unknown,
    })
}

pub(super) fn parse_authorization_server(
    issuer: &str,
    body: Value,
    config: &McpOAuthConfig,
) -> Result<AuthorizationServerMetadata> {
    let wire: AuthorizationServerWire =
        decode_metadata(body, OAuthEndpointKind::AuthorizationServerMetadata)?;
    if wire.issuer != issuer {
        return Err(Error::IssuerMismatch {
            expected: issuer.to_owned(),
            actual: wire.issuer,
        });
    }
    let authorization_endpoint = required_endpoint(
        wire.authorization_endpoint,
        OAuthEndpointKind::Authorization,
        config,
    )?;
    let token_endpoint = required_endpoint(wire.token_endpoint, OAuthEndpointKind::Token, config)?;
    validate_optional_endpoint(
        wire.registration_endpoint.as_deref(),
        OAuthEndpointKind::Registration,
        config,
    )?;
    validate_optional_endpoint(
        wire.revocation_endpoint.as_deref(),
        OAuthEndpointKind::Revocation,
        config,
    )?;
    Ok(AuthorizationServerMetadata {
        issuer: wire.issuer,
        authorization_endpoint,
        token_endpoint,
        registration_endpoint: wire.registration_endpoint,
        revocation_endpoint: wire.revocation_endpoint,
        grant_types_supported: wire.grant_types_supported,
        token_endpoint_auth_methods_supported: wire.token_endpoint_auth_methods_supported,
        code_challenge_methods_supported: wire.code_challenge_methods_supported,
        scopes_supported: OAuthScopes::new(wire.scopes_supported),
        unknown: wire.unknown,
    })
}

pub(super) fn require_discovery_response(
    response: &OAuthHttpResponse,
    endpoint: OAuthEndpointKind,
) -> Result<()> {
    if response.status != 200 || !has_json_content_type(response) {
        return Err(Error::DiscoveryStatus {
            endpoint,
            status: response.status,
        });
    }
    Ok(())
}

pub(super) fn authorization_server_metadata_url(issuer: &str) -> Result<String> {
    let mut url = match Url::parse(issuer) {
        Ok(url) => url,
        Err(_) => {
            return Err(Error::InvalidUrl {
                endpoint: OAuthEndpointKind::AuthorizationServerMetadata,
            });
        }
    };
    if url.query().is_some() {
        return Err(Error::InvalidUrl {
            endpoint: OAuthEndpointKind::AuthorizationServerMetadata,
        });
    }
    let issuer_path = url.path().trim_start_matches('/');
    let path = if issuer_path.is_empty() {
        "/.well-known/oauth-authorization-server".to_owned()
    } else {
        format!("/.well-known/oauth-authorization-server/{issuer_path}")
    };
    url.set_path(&path);
    Ok(url.to_string())
}

fn decode_metadata<T>(body: Value, endpoint: OAuthEndpointKind) -> Result<T>
where
    T: for<'de> Deserialize<'de>,
{
    match serde_json::from_value(body) {
        Ok(metadata) => Ok(metadata),
        Err(source) => Err(Error::DiscoverySchema { endpoint, source }),
    }
}

fn required_endpoint(
    endpoint: Option<String>,
    kind: OAuthEndpointKind,
    config: &McpOAuthConfig,
) -> Result<String> {
    let Some(endpoint) = endpoint else {
        return Err(Error::MissingEndpoint { endpoint: kind });
    };
    config.url_policy.parse(&endpoint, kind)?;
    Ok(endpoint)
}

fn validate_optional_endpoint(
    endpoint: Option<&str>,
    kind: OAuthEndpointKind,
    config: &McpOAuthConfig,
) -> Result<()> {
    if let Some(endpoint) = endpoint {
        config.url_policy.parse(endpoint, kind)?;
    }
    Ok(())
}

fn has_json_content_type(response: &OAuthHttpResponse) -> bool {
    response.headers.get("content-type").is_some_and(|values| {
        values.iter().any(|value| {
            value
                .split(';')
                .next()
                .is_some_and(|media| media.trim().eq_ignore_ascii_case("application/json"))
        })
    })
}
