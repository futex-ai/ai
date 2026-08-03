//! Tokio-backed OAuth DNS resolution.

use std::{collections::BTreeSet, net::IpAddr};

use async_trait::async_trait;

use crate::{Error, OAuthDnsResolver, Result};

#[derive(Clone, Copy, Debug, Default)]
/// Production DNS resolver backed by Tokio.
pub struct SystemOAuthDnsResolver;

#[async_trait]
impl OAuthDnsResolver for SystemOAuthDnsResolver {
    async fn resolve(&self, host: &str, port: u16) -> Result<Vec<IpAddr>> {
        let resolved = match tokio::net::lookup_host((host, port)).await {
            Ok(resolved) => resolved,
            Err(_) => return Err(Error::Dns),
        };
        let addresses = resolved
            .map(|socket| socket.ip())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        if addresses.is_empty() {
            return Err(Error::Dns);
        }
        Ok(addresses)
    }
}
