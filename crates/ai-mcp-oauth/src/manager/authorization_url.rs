//! Authorization-endpoint validation and client-owned query construction.

use url::Url;

use crate::{Error, McpOAuthConfig, OAuthEndpointKind, OAuthScopes, Result};

pub(super) fn validated_authorization_endpoint(
    endpoint: &str,
    config: &McpOAuthConfig,
) -> Result<Url> {
    let url = config
        .url_policy
        .parse(endpoint, OAuthEndpointKind::Authorization)?;
    reject_reserved_parameters(&url)?;
    Ok(url)
}

pub(super) fn build_authorization_url(
    mut url: Url,
    client_id: &str,
    redirect_uri: &str,
    resource: &str,
    scopes: &OAuthScopes,
    state: &str,
    challenge: &str,
) -> String {
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
    url.to_string()
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

#[cfg(test)]
#[path = "../_tests_/manager/authorization_url_tests.rs"]
mod authorization_url_tests;
