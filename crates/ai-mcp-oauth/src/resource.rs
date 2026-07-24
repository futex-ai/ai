//! Canonical MCP resource identities and ordered OAuth scopes.

use std::collections::BTreeSet;

use url::Url;

use crate::{Error, OAuthEndpointKind, OAuthUrlPolicy, Result};

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
/// Canonical, absolute identity of one protected MCP HTTP resource.
pub struct CanonicalMcpResource(String);

impl CanonicalMcpResource {
    /// Parses and canonicalizes one MCP endpoint under the supplied URL policy.
    pub fn parse(value: &str, policy: &OAuthUrlPolicy) -> Result<Self> {
        let url = policy.parse(value, OAuthEndpointKind::Resource)?;
        Ok(Self(canonical_url_string(&url)))
    }

    /// Returns the exact resource string used for OAuth and credential keys.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Derives the RFC 9728 well-known protected-resource metadata URL.
    pub fn protected_resource_metadata_url(&self) -> Result<String> {
        let mut url = match Url::parse(&self.0) {
            Ok(url) => url,
            Err(_) => {
                return Err(Error::InvalidUrl {
                    endpoint: OAuthEndpointKind::Resource,
                });
            }
        };
        let resource_path = url.path().trim_start_matches('/');
        let metadata_path = if resource_path.is_empty() {
            "/.well-known/oauth-protected-resource".to_owned()
        } else {
            format!("/.well-known/oauth-protected-resource/{resource_path}")
        };
        url.set_path(&metadata_path);
        Ok(url.to_string())
    }
}

impl AsRef<str> for CanonicalMcpResource {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl std::fmt::Display for CanonicalMcpResource {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Clone, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
/// Deduplicated OAuth scopes retained in first-seen order.
pub struct OAuthScopes(Vec<String>);

impl OAuthScopes {
    /// Builds an ordered scope set from individual scope names.
    pub fn new<I, S>(scopes: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let mut seen = BTreeSet::new();
        let values = scopes
            .into_iter()
            .map(Into::into)
            .filter(|scope| !scope.is_empty() && seen.insert(scope.clone()))
            .collect();
        Self(values)
    }

    /// Parses an OAuth space-delimited scope response.
    pub fn parse(value: &str) -> Self {
        Self::new(value.split_ascii_whitespace())
    }

    /// Returns scopes in stable first-seen order.
    pub fn as_slice(&self) -> &[String] {
        &self.0
    }

    /// Returns whether no scopes are present.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Combines scopes while preserving the first occurrence of each value.
    pub fn union(&self, additional: &Self) -> Self {
        Self::new(self.0.iter().chain(&additional.0).cloned())
    }

    /// Serializes scopes for OAuth query and form parameters.
    pub fn to_parameter(&self) -> String {
        self.0.join(" ")
    }
}

fn canonical_url_string(url: &Url) -> String {
    let mut canonical = url.to_string();
    if url.path() == "/" && url.query().is_none() && canonical.ends_with('/') {
        canonical.pop();
    }
    canonical
}
