//! Bounded, redirect-aware OAuth HTTP transport boundaries.

use std::{collections::BTreeMap, net::IpAddr, sync::Arc, time::Duration};

use async_trait::async_trait;
use serde_json::Value;

use crate::{OAuthEndpointKind, OAuthUrlPolicy, Result};

mod reqwest;

pub use reqwest::{ReqwestOAuthHttpTransport, SystemOAuthDnsResolver};

/// Shared OAuth HTTP transport.
pub type DynOAuthHttpTransport = Arc<dyn OAuthHttpTransport>;

/// Shared DNS resolver used by the production transport.
pub type DynOAuthDnsResolver = Arc<dyn OAuthDnsResolver>;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
/// Bounds applied to each OAuth HTTP operation.
pub struct OAuthHttpLimits {
    /// Maximum duration of each request hop.
    pub timeout: Duration,
    /// Maximum final response size.
    pub max_response_bytes: usize,
    /// Maximum number of manually validated redirects.
    pub max_redirects: usize,
}

#[derive(Clone, PartialEq)]
/// Bounded OAuth HTTP response with normalized multi-value headers.
pub struct OAuthHttpResponse {
    /// Final HTTP status.
    pub status: u16,
    /// Lowercase response-header names mapped to wire-order values.
    pub headers: BTreeMap<String, Vec<String>>,
    /// Parsed JSON response, or `null` for an empty body.
    pub body: Value,
}

impl std::fmt::Debug for OAuthHttpResponse {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("OAuthHttpResponse")
            .field("status", &self.status)
            .field("headers", &"[REDACTED]")
            .field("body", &"[REDACTED]")
            .finish()
    }
}

#[cfg_attr(
    any(test, doctest, feature = "test-support"),
    unimock::unimock(api = OAuthHttpTransportMock)
)]
#[async_trait]
/// Sends bounded OAuth JSON and form requests.
pub trait OAuthHttpTransport: Send + Sync {
    /// Fetches one JSON endpoint.
    async fn get_json(
        &self,
        url: &str,
        endpoint: OAuthEndpointKind,
        policy: &OAuthUrlPolicy,
        limits: OAuthHttpLimits,
    ) -> Result<OAuthHttpResponse>;

    /// Posts one JSON object.
    async fn post_json(
        &self,
        url: &str,
        endpoint: OAuthEndpointKind,
        policy: &OAuthUrlPolicy,
        limits: OAuthHttpLimits,
        body: &Value,
    ) -> Result<OAuthHttpResponse>;

    /// Posts one URL-encoded form.
    async fn post_form(
        &self,
        url: &str,
        endpoint: OAuthEndpointKind,
        policy: &OAuthUrlPolicy,
        limits: OAuthHttpLimits,
        fields: &[(String, String)],
    ) -> Result<OAuthHttpResponse>;
}

#[cfg_attr(
    any(test, doctest, feature = "test-support"),
    unimock::unimock(api = OAuthDnsResolverMock)
)]
#[async_trait]
/// Resolves one hostname immediately before a pinned HTTP connection.
pub trait OAuthDnsResolver: Send + Sync {
    /// Returns every resolved address for one host and port.
    async fn resolve(&self, host: &str, port: u16) -> Result<Vec<IpAddr>>;
}
