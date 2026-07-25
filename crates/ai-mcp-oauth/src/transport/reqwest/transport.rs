//! Production OAuth request execution.

use std::{net::SocketAddr, sync::Arc};

use async_trait::async_trait;
use reqwest::{Client, header::LOCATION, redirect::Policy};
use serde_json::Value;
use tokio::time::{Instant, timeout_at};
use url::{Host, Url};

use crate::{
    DynOAuthDnsResolver, Error, OAuthConfigField, OAuthEndpointKind, OAuthHttpLimits,
    OAuthHttpResponse, OAuthHttpTransport, OAuthUnsafeUrlReason, OAuthUrlPolicy, Result,
};

use super::{
    SystemOAuthDnsResolver,
    request::{RequestPayload, follows_redirect, request_builder},
    response::bounded_response,
};

#[derive(Clone)]
/// Production OAuth transport that manually validates every redirect and peer.
pub struct ReqwestOAuthHttpTransport {
    resolver: DynOAuthDnsResolver,
}

impl ReqwestOAuthHttpTransport {
    /// Builds the production transport with the system DNS resolver.
    pub fn new() -> Self {
        Self {
            resolver: Arc::new(SystemOAuthDnsResolver),
        }
    }

    /// Builds a transport with an injected resolver for testing or host policy.
    pub fn with_resolver(resolver: DynOAuthDnsResolver) -> Self {
        Self { resolver }
    }

    async fn execute(
        &self,
        initial_url: &str,
        endpoint: OAuthEndpointKind,
        policy: &OAuthUrlPolicy,
        limits: OAuthHttpLimits,
        payload: RequestPayload,
    ) -> Result<OAuthHttpResponse> {
        let mut url = policy.parse(initial_url, endpoint)?;
        for redirect_count in 0..=limits.max_redirects {
            match self
                .execute_hop(&url, endpoint, policy, limits, &payload)
                .await?
            {
                HopOutcome::Complete(response) => return Ok(response),
                HopOutcome::Redirect(next) => {
                    if redirect_count == limits.max_redirects {
                        return Err(Error::TooManyRedirects);
                    }
                    url = next;
                }
            }
        }
        Err(Error::TooManyRedirects)
    }

    async fn execute_hop(
        &self,
        url: &Url,
        endpoint: OAuthEndpointKind,
        policy: &OAuthUrlPolicy,
        limits: OAuthHttpLimits,
        payload: &RequestPayload,
    ) -> Result<HopOutcome> {
        let deadline = match Instant::now().checked_add(limits.timeout) {
            Some(deadline) => deadline,
            None => {
                return Err(Error::InvalidConfig {
                    field: OAuthConfigField::HttpTimeout,
                });
            }
        };
        let client = match timeout_at(deadline, self.client_for(url, endpoint, policy)).await {
            Ok(result) => result?,
            Err(_) => return Err(Error::Dns),
        };
        let remaining = deadline.saturating_duration_since(Instant::now());
        let request = request_builder(&client, url, payload).timeout(remaining);
        let response = match timeout_at(deadline, request.send()).await {
            Ok(Ok(response)) => response,
            Ok(Err(_)) | Err(_) => return Err(Error::Transport),
        };
        if follows_redirect(response.status()) {
            if !payload.is_get() {
                return Err(Error::RedirectNotAllowed { endpoint });
            }
            let location = match response.headers().get(LOCATION) {
                Some(location) => location,
                None => return Err(Error::InvalidRedirect),
            };
            let location = match location.to_str() {
                Ok(location) => location,
                Err(_) => return Err(Error::InvalidRedirect),
            };
            let next = match url.join(location) {
                Ok(url) => url,
                Err(_) => return Err(Error::InvalidRedirect),
            };
            policy.validate_url(&next, endpoint)?;
            return Ok(HopOutcome::Redirect(next));
        }
        match timeout_at(
            deadline,
            bounded_response(response, limits.max_response_bytes),
        )
        .await
        {
            Ok(result) => Ok(HopOutcome::Complete(result?)),
            Err(_) => Err(Error::Transport),
        }
    }

    async fn client_for(
        &self,
        url: &Url,
        endpoint: OAuthEndpointKind,
        policy: &OAuthUrlPolicy,
    ) -> Result<Client> {
        policy.validate_url(url, endpoint)?;
        let host = match url.host() {
            Some(host) => host,
            None => return Err(Error::InvalidUrl { endpoint }),
        };
        let port = match url.port_or_known_default() {
            Some(port) => port,
            None => return Err(Error::InvalidUrl { endpoint }),
        };
        let mut builder = Client::builder().redirect(Policy::none());
        if let Host::Domain(domain) = host {
            let addresses = self.resolver.resolve(domain, port).await?;
            if addresses
                .iter()
                .any(|address| !policy.address_allowed(*address, url.scheme()))
            {
                return Err(Error::UnsafeUrl {
                    endpoint,
                    reason: OAuthUnsafeUrlReason::Address,
                });
            }
            let sockets = addresses
                .into_iter()
                .map(|address| SocketAddr::new(address, port))
                .collect::<Vec<_>>();
            builder = builder.resolve_to_addrs(domain, &sockets);
        }
        match builder.build() {
            Ok(client) => Ok(client),
            Err(_) => Err(Error::Transport),
        }
    }
}

impl Default for ReqwestOAuthHttpTransport {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl OAuthHttpTransport for ReqwestOAuthHttpTransport {
    async fn get_json(
        &self,
        url: &str,
        endpoint: OAuthEndpointKind,
        policy: &OAuthUrlPolicy,
        limits: OAuthHttpLimits,
    ) -> Result<OAuthHttpResponse> {
        self.execute(url, endpoint, policy, limits, RequestPayload::Get)
            .await
    }

    async fn post_json(
        &self,
        url: &str,
        endpoint: OAuthEndpointKind,
        policy: &OAuthUrlPolicy,
        limits: OAuthHttpLimits,
        body: &Value,
    ) -> Result<OAuthHttpResponse> {
        self.execute(
            url,
            endpoint,
            policy,
            limits,
            RequestPayload::Json(body.clone()),
        )
        .await
    }

    async fn post_form(
        &self,
        url: &str,
        endpoint: OAuthEndpointKind,
        policy: &OAuthUrlPolicy,
        limits: OAuthHttpLimits,
        fields: &[(String, String)],
    ) -> Result<OAuthHttpResponse> {
        self.execute(
            url,
            endpoint,
            policy,
            limits,
            RequestPayload::Form(fields.to_vec()),
        )
        .await
    }
}

enum HopOutcome {
    Redirect(Url),
    Complete(OAuthHttpResponse),
}
