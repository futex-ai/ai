//! Reqwest OAuth transport with validated, DNS-pinned request hops.

use std::{
    collections::{BTreeMap, BTreeSet},
    net::{IpAddr, SocketAddr},
    sync::Arc,
};

use async_trait::async_trait;
use futures_util::StreamExt;
use reqwest::{Client, StatusCode, header::LOCATION, redirect::Policy};
use serde_json::Value;
use url::{Host, Url};

use crate::{
    DynOAuthDnsResolver, Error, OAuthDnsResolver, OAuthEndpointKind, OAuthHttpLimits,
    OAuthHttpResponse, OAuthHttpTransport, OAuthUrlPolicy, Result,
};

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
        mut payload: RequestPayload,
    ) -> Result<OAuthHttpResponse> {
        let mut url = policy.parse(initial_url, endpoint)?;
        for redirect_count in 0..=limits.max_redirects {
            let client = self.client_for(&url, endpoint, policy).await?;
            let request = request_builder(&client, &url, &payload).timeout(limits.timeout);
            let response = match request.send().await {
                Ok(response) => response,
                Err(_) => return Err(Error::Transport),
            };
            if follows_redirect(response.status()) {
                if redirect_count == limits.max_redirects {
                    return Err(Error::TooManyRedirects);
                }
                let location = match response.headers().get(LOCATION) {
                    Some(location) => location,
                    None => return Err(Error::InvalidRedirect),
                };
                let location = match location.to_str() {
                    Ok(location) => location,
                    Err(_) => return Err(Error::InvalidRedirect),
                };
                url = match url.join(location) {
                    Ok(url) => url,
                    Err(_) => return Err(Error::InvalidRedirect),
                };
                policy.validate_url(&url, endpoint)?;
                payload = redirected_payload(response.status(), payload);
                continue;
            }
            return bounded_response(response, limits.max_response_bytes).await;
        }
        Err(Error::TooManyRedirects)
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
            None => {
                return Err(Error::InvalidUrl { endpoint });
            }
        };
        let port = match url.port_or_known_default() {
            Some(port) => port,
            None => {
                return Err(Error::InvalidUrl { endpoint });
            }
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
                    reason: crate::OAuthUnsafeUrlReason::Address,
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

#[derive(Clone)]
enum RequestPayload {
    Get,
    Json(Value),
    Form(Vec<(String, String)>),
}

fn request_builder(
    client: &Client,
    url: &Url,
    payload: &RequestPayload,
) -> reqwest::RequestBuilder {
    match payload {
        RequestPayload::Get => client.get(url.clone()),
        RequestPayload::Json(body) => client.post(url.clone()).json(body),
        RequestPayload::Form(fields) => client.post(url.clone()).form(fields),
    }
}

fn follows_redirect(status: StatusCode) -> bool {
    matches!(
        status,
        StatusCode::MOVED_PERMANENTLY
            | StatusCode::FOUND
            | StatusCode::SEE_OTHER
            | StatusCode::TEMPORARY_REDIRECT
            | StatusCode::PERMANENT_REDIRECT
    )
}

fn redirected_payload(status: StatusCode, payload: RequestPayload) -> RequestPayload {
    if matches!(
        status,
        StatusCode::MOVED_PERMANENTLY | StatusCode::FOUND | StatusCode::SEE_OTHER
    ) {
        RequestPayload::Get
    } else {
        payload
    }
}

async fn bounded_response(response: reqwest::Response, limit: usize) -> Result<OAuthHttpResponse> {
    let status = response.status().as_u16();
    let headers = normalized_headers(response.headers());
    let mut bytes = Vec::new();
    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = match chunk {
            Ok(chunk) => chunk,
            Err(_) => return Err(Error::Transport),
        };
        if bytes.len().saturating_add(chunk.len()) > limit {
            return Err(Error::ResponseTooLarge { limit_bytes: limit });
        }
        bytes.extend_from_slice(&chunk);
    }
    let body = if bytes.is_empty() {
        Value::Null
    } else {
        match serde_json::from_slice(&bytes) {
            Ok(body) => body,
            Err(_) => return Err(Error::InvalidJsonResponse),
        }
    };
    Ok(OAuthHttpResponse {
        status,
        headers,
        body,
    })
}

fn normalized_headers(headers: &reqwest::header::HeaderMap) -> BTreeMap<String, Vec<String>> {
    let mut normalized = BTreeMap::new();
    for name in headers.keys() {
        let values = headers
            .get_all(name)
            .iter()
            .filter_map(|value| value.to_str().ok().map(str::to_owned))
            .collect::<Vec<_>>();
        normalized.insert(name.as_str().to_ascii_lowercase(), values);
    }
    normalized
}
