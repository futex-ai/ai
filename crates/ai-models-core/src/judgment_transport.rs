//! Shared HTTP transport for judgment adapters.
//!
//! Every judgment adapter sends its provider body through
//! [`HttpJudgmentTransport`]. The transport applies the auth hook once,
//! derives the redaction set from the transmitted headers, classifies request
//! failures and unsuccessful statuses onto `JudgmentError`, and returns only
//! `2xx` bodies for provider-specific parsing. Adapters own only their
//! endpoint constant, wire mapping, failure-message extractor, and parser.

use std::{collections::BTreeMap, fmt, time::Duration};

use ai_interface::{JudgmentError, JudgmentResult};
use async_trait::async_trait;
use json_http::{DynJsonHttpAuth, DynJsonHttpClient};
use serde_json::Value;

use crate::{
    AUTH_HOOK_FAILED, classify_judgment_request_error, classify_judgment_status,
    decode_failure_body, secrets_from_headers,
};

const MALFORMED_PROVIDER_PAYLOAD: &str = "malformed provider payload: ";

/// Pure extractor from a decoded failure body to the provider message.
///
/// An extractor returns a JSON string body unchanged so the `HTTP <status>`
/// text that `decode_failure_body` supplies for an empty body is reported.
pub type JudgmentFailureMessage = fn(&Value) -> String;

/// Static endpoint configuration that one judgment adapter defines as a constant.
#[derive(Clone, Copy, Debug)]
pub struct JudgmentEndpoint {
    /// Provider name reported in responses and errors, for example `openai`.
    pub provider: &'static str,
    /// Default request URL.
    pub url: &'static str,
    /// Default per-request timeout.
    pub timeout: Duration,
    /// Provider failure-message extractor for unsuccessful responses.
    pub failure_message: JudgmentFailureMessage,
}

/// Successful judgment HTTP response with the redaction set of its credentials.
///
/// The type has no `Debug` implementation because `secrets` holds raw
/// credentials.
#[derive(Clone, Eq, PartialEq)]
pub struct JudgmentHttpResponse {
    /// Raw `2xx` response body bytes.
    pub body: Vec<u8>,
    /// Secrets derived from the transmitted credentials, used to redact
    /// diagnostics produced while parsing `body`.
    pub secrets: Vec<String>,
}

#[cfg_attr(
    any(test, doctest, feature = "test-support"),
    unimock::unimock(api = JudgmentTransportMock)
)]
#[async_trait]
/// Transport boundary that sends one judgment request body.
pub trait JudgmentTransport: Send + Sync {
    /// Posts one JSON body and returns the `2xx` body bytes and redaction set.
    async fn post(&self, body: Value) -> JudgmentResult<JudgmentHttpResponse>;
}

/// `json-http` judgment transport for one provider endpoint and model.
///
/// The type has no `Debug` implementation because it holds explicit secrets.
#[derive(Clone)]
pub struct HttpJudgmentTransport {
    provider: &'static str,
    failure_message: JudgmentFailureMessage,
    endpoint: String,
    timeout: Duration,
    model_id: String,
    http_client: DynJsonHttpClient,
    auth: DynJsonHttpAuth,
    explicit_secrets: Vec<String>,
}

impl HttpJudgmentTransport {
    /// Builds a transport from an endpoint constant, a provider model id, an
    /// injected client, and an auth hook.
    pub fn new(
        endpoint: JudgmentEndpoint,
        model_id: impl Into<String>,
        http_client: DynJsonHttpClient,
        auth: DynJsonHttpAuth,
    ) -> Self {
        Self {
            provider: endpoint.provider,
            failure_message: endpoint.failure_message,
            endpoint: endpoint.url.to_owned(),
            timeout: endpoint.timeout,
            model_id: model_id.into(),
            http_client,
            auth,
            explicit_secrets: Vec::new(),
        }
    }

    /// Adds a secret that is redacted from every diagnostic, such as an API
    /// key that the auth hook sends inside a larger header value.
    pub fn with_explicit_secret(mut self, secret: impl Into<String>) -> Self {
        self.explicit_secrets.push(secret.into());
        self
    }

    /// Overrides the request URL.
    pub fn with_endpoint(mut self, endpoint: impl Into<String>) -> Self {
        self.endpoint = endpoint.into();
        self
    }

    /// Overrides the request timeout.
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Returns the provider name reported in responses and errors.
    pub fn provider(&self) -> &'static str {
        self.provider
    }

    /// Returns the configured provider model id.
    pub fn model_id(&self) -> &str {
        &self.model_id
    }

    /// Returns the request URL.
    pub fn endpoint(&self) -> &str {
        &self.endpoint
    }

    /// Returns the request timeout.
    pub fn timeout(&self) -> Duration {
        self.timeout
    }

    fn request_error(&self, source: json_http::Error, secrets: &[String]) -> JudgmentError {
        classify_judgment_request_error(self.provider, source, &self.model_id, secrets)
    }
}

#[async_trait]
impl JudgmentTransport for HttpJudgmentTransport {
    async fn post(&self, body: Value) -> JudgmentResult<JudgmentHttpResponse> {
        let mut headers = BTreeMap::new();
        if self.auth.apply_headers(&mut headers).await.is_err() {
            return Err(JudgmentError::transient_provider(
                self.provider,
                &self.model_id,
                AUTH_HOOK_FAILED,
            ));
        }
        let secrets = secrets_from_headers(&self.explicit_secrets, &headers);
        let request = match self
            .http_client
            .post(&self.endpoint)
            .headers(headers)
            .timeout(self.timeout)
            .json(body)
        {
            Ok(request) => request,
            Err(source) => return Err(self.request_error(source, &secrets)),
        };
        let response = match request.send_bytes().await {
            Ok(response) => response,
            Err(source) => return Err(self.request_error(source, &secrets)),
        };
        if !(200..300).contains(&response.status) {
            let failure_body = decode_failure_body(response.status, &response.body);
            let message = (self.failure_message)(&failure_body);
            return Err(classify_judgment_status(
                self.provider,
                response.status,
                &self.model_id,
                &message,
                &secrets,
            ));
        }
        Ok(JudgmentHttpResponse {
            body: response.body,
            secrets,
        })
    }
}

/// Builds the terminal `Provider` error for a `2xx` body that fails to decode.
///
/// The message is `malformed provider payload: ` followed by `detail`.
/// Callers pass the error through `redact_judgment_error` before returning it.
pub fn malformed_judgment_payload(
    provider: &str,
    model_id: &str,
    detail: &dyn fmt::Display,
) -> JudgmentError {
    let detail = detail.to_string();
    JudgmentError::provider(
        provider,
        model_id,
        [MALFORMED_PROVIDER_PAYLOAD, &detail].concat(),
    )
}

#[cfg(test)]
#[path = "_tests_/judgment_transport_support.rs"]
mod judgment_transport_support;

#[cfg(test)]
#[path = "_tests_/judgment_transport_tests.rs"]
mod judgment_transport_tests;

#[cfg(test)]
#[path = "_tests_/judgment_transport_status_tests.rs"]
mod judgment_transport_status_tests;
