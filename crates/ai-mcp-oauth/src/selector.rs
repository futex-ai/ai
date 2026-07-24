//! Host boundary for explicit authorization-server selection.

use std::sync::Arc;

use async_trait::async_trait;

use crate::Result;

/// Shared authorization-server selector.
pub type DynAuthorizationServerSelector = Arc<dyn AuthorizationServerSelector>;

#[cfg_attr(
    any(test, doctest, feature = "test-support"),
    unimock::unimock(api = AuthorizationServerSelectorMock)
)]
#[async_trait]
/// Lets a host explicitly select among multiple advertised issuers.
pub trait AuthorizationServerSelector: Send + Sync {
    /// Selects one issuer or returns `Error::IssuerSelectionCancelled`.
    async fn select(&self, resource: &str, issuers: &[String]) -> Result<String>;
}
