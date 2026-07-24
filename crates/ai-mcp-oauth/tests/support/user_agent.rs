//! Loopback user agent and issuer selector for integration tests.

use std::sync::atomic::{AtomicUsize, Ordering};

use ai_mcp_oauth::{
    AuthorizationServerSelector, Error, OAuthAuthorizationError, OAuthAuthorizationResponse,
    OAuthUserAgent, OAuthUserAuthorizationRequest, Result,
};
use async_trait::async_trait;
use reqwest::{StatusCode, header::LOCATION, redirect::Policy};
use url::Url;

#[derive(Default)]
pub(crate) struct LoopbackUserAgent {
    calls: AtomicUsize,
}

impl LoopbackUserAgent {
    pub(crate) fn call_count(&self) -> usize {
        self.calls.load(Ordering::SeqCst)
    }
}

#[async_trait]
impl OAuthUserAgent for LoopbackUserAgent {
    async fn authorize(
        &self,
        request: OAuthUserAuthorizationRequest,
    ) -> Result<OAuthAuthorizationResponse> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        let client = match reqwest::Client::builder().redirect(Policy::none()).build() {
            Ok(client) => client,
            Err(_) => return Err(Error::Transport),
        };
        let response = match client.get(request.authorization_url()).send().await {
            Ok(response) => response,
            Err(_) => return Err(Error::Transport),
        };
        if response.status() != StatusCode::FOUND {
            return Err(Error::Transport);
        }
        let location = match response
            .headers()
            .get(LOCATION)
            .and_then(|value| value.to_str().ok())
        {
            Some(location) => location,
            None => return Err(Error::InvalidRedirect),
        };
        let callback = match Url::parse(location) {
            Ok(callback) => callback,
            Err(_) => return Err(Error::InvalidRedirect),
        };
        let query = callback
            .query_pairs()
            .map(|(name, value)| (name.into_owned(), value.into_owned()))
            .collect::<std::collections::BTreeMap<_, _>>();
        if query.get("error").map(String::as_str) == Some("access_denied") {
            return Ok(OAuthAuthorizationResponse::oauth_error(
                OAuthAuthorizationError::AccessDenied,
                query.get("state"),
            ));
        }
        let Some(code) = query.get("code") else {
            return Err(Error::AuthorizationCodeMissing);
        };
        Ok(OAuthAuthorizationResponse::authorized(
            code,
            query.get("state"),
        ))
    }
}

pub(crate) struct FirstIssuerSelector;

#[async_trait]
impl AuthorizationServerSelector for FirstIssuerSelector {
    async fn select(&self, _resource: &str, issuers: &[String]) -> Result<String> {
        match issuers.first() {
            Some(issuer) => Ok(issuer.clone()),
            None => Err(Error::MissingAuthorizationServer),
        }
    }
}
