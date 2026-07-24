//! RFC 9728 and RFC 8414 metadata discovery with bounded caching.

use std::{collections::BTreeMap, sync::Arc};

use ai_mcp::McpAuthorizationChallenge;
use async_trait::async_trait;
use tokio::sync::Mutex;

use crate::{
    AuthorizationServerMetadata, CanonicalMcpResource, DynAuthorizationServerSelector,
    DynOAuthClock, DynOAuthHttpTransport, Error, McpOAuthConfig, OAuthEndpointKind,
    ProtectedResourceMetadata, Result,
};

use self::{
    cache::cache_age_seconds,
    parsing::{
        authorization_server_metadata_url, parse_authorization_server, parse_protected_resource,
        require_discovery_response,
    },
};

mod cache;
mod parsing;

/// Shared MCP OAuth discovery service.
pub type DynMcpOAuthDiscovery = Arc<dyn McpOAuthDiscovery>;

#[derive(Clone, Debug, PartialEq)]
/// Validated metadata needed to register and authorize one MCP resource.
pub struct OAuthDiscoveryResult {
    /// Metadata URL used for RFC 9728 discovery.
    pub resource_metadata_url: String,
    /// Validated protected-resource metadata.
    pub protected_resource: ProtectedResourceMetadata,
    /// Selected and validated authorization-server metadata.
    pub authorization_server: AuthorizationServerMetadata,
}

#[cfg_attr(
    any(test, doctest, feature = "test-support"),
    unimock::unimock(api = McpOAuthDiscoveryMock)
)]
#[async_trait]
/// Discovers validated OAuth metadata from an MCP Bearer challenge.
pub trait McpOAuthDiscovery: Send + Sync {
    /// Discovers metadata for one canonical MCP resource.
    async fn discover(
        &self,
        resource: &CanonicalMcpResource,
        challenge: &McpAuthorizationChallenge,
    ) -> Result<OAuthDiscoveryResult>;
}

/// Default discovery implementation over injected network, selection, and time.
pub struct DefaultMcpOAuthDiscovery {
    transport: DynOAuthHttpTransport,
    selector: DynAuthorizationServerSelector,
    clock: DynOAuthClock,
    config: McpOAuthConfig,
    cache: Mutex<BTreeMap<CanonicalMcpResource, CacheEntry>>,
}

impl DefaultMcpOAuthDiscovery {
    /// Builds a validated discovery service.
    pub fn new(
        transport: DynOAuthHttpTransport,
        selector: DynAuthorizationServerSelector,
        clock: DynOAuthClock,
        config: McpOAuthConfig,
    ) -> Result<Self> {
        config.validate()?;
        Ok(Self {
            transport,
            selector,
            clock,
            config,
            cache: Mutex::new(BTreeMap::new()),
        })
    }

    async fn discover_uncached(
        &self,
        resource: &CanonicalMcpResource,
        metadata_url: &str,
    ) -> Result<(OAuthDiscoveryResult, u64)> {
        let protected_response = self
            .transport
            .get_json(
                metadata_url,
                OAuthEndpointKind::ProtectedResourceMetadata,
                &self.config.url_policy,
                self.config.http_limits(),
            )
            .await?;
        require_discovery_response(
            &protected_response,
            OAuthEndpointKind::ProtectedResourceMetadata,
        )?;
        let protected = parse_protected_resource(resource, protected_response.body)?;
        let issuer = select_issuer(
            self.selector.as_ref(),
            resource,
            &protected.authorization_servers,
        )
        .await?;
        self.config
            .url_policy
            .parse(&issuer, OAuthEndpointKind::AuthorizationServerMetadata)?;
        let server_metadata_url = authorization_server_metadata_url(&issuer)?;
        let server_response = self
            .transport
            .get_json(
                &server_metadata_url,
                OAuthEndpointKind::AuthorizationServerMetadata,
                &self.config.url_policy,
                self.config.http_limits(),
            )
            .await?;
        require_discovery_response(
            &server_response,
            OAuthEndpointKind::AuthorizationServerMetadata,
        )?;
        let server = parse_authorization_server(&issuer, server_response.body, &self.config)?;
        let age = cache_age_seconds(
            &protected_response.headers,
            &server_response.headers,
            self.config.max_metadata_cache_age.as_secs(),
        );
        Ok((
            OAuthDiscoveryResult {
                resource_metadata_url: metadata_url.to_owned(),
                protected_resource: protected,
                authorization_server: server,
            },
            age,
        ))
    }
}

#[async_trait]
impl McpOAuthDiscovery for DefaultMcpOAuthDiscovery {
    async fn discover(
        &self,
        resource: &CanonicalMcpResource,
        challenge: &McpAuthorizationChallenge,
    ) -> Result<OAuthDiscoveryResult> {
        let now = self.clock.now_unix_seconds()?;
        let metadata_url = if let Some(advertised) = &challenge.resource_metadata_url {
            self.config
                .url_policy
                .parse(advertised, OAuthEndpointKind::ProtectedResourceMetadata)?;
            advertised.clone()
        } else {
            resource.protected_resource_metadata_url()?
        };
        {
            let mut cache = self.cache.lock().await;
            if let Some(entry) = cache.get(resource) {
                if entry.metadata_url == metadata_url && entry.expires_at > now {
                    return Ok(entry.result.clone());
                }
                cache.remove(resource);
            }
        }
        let (result, cache_age) = self.discover_uncached(resource, &metadata_url).await?;
        if cache_age > 0 {
            self.cache.lock().await.insert(
                resource.clone(),
                CacheEntry {
                    metadata_url,
                    expires_at: now.saturating_add(cache_age),
                    result: result.clone(),
                },
            );
        }
        Ok(result)
    }
}

#[derive(Clone)]
struct CacheEntry {
    metadata_url: String,
    expires_at: u64,
    result: OAuthDiscoveryResult,
}

async fn select_issuer(
    selector: &dyn crate::AuthorizationServerSelector,
    resource: &CanonicalMcpResource,
    issuers: &[String],
) -> Result<String> {
    if issuers.len() == 1 {
        return Ok(issuers[0].clone());
    }
    let selected = selector.select(resource.as_str(), issuers).await?;
    if !issuers.iter().any(|issuer| issuer == &selected) {
        return Err(Error::InvalidIssuerSelection);
    }
    Ok(selected)
}
