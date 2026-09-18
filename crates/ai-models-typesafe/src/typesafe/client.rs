//! TypeSafe judgment transport client.

use std::{collections::BTreeMap, sync::Arc, time::Duration};

use ai_interface::{
    JudgmentError, JudgmentModel, JudgmentRequest, JudgmentResponse, JudgmentResult,
};
use async_trait::async_trait;
use json_http::{DynJsonHttpAuth, DynJsonHttpClient, StaticHeaderAuth};
use serde_json::Value;

use super::{
    error::{AUTH_HOOK_FAILED, classify_request_error, classify_status, http_status_message},
    redaction::secrets_from_headers,
    request::build_request,
    response::{parse_response, redact_response_error},
};

const PROVIDER: &str = "typesafe";
const TYPESAFE_SYSTEM_ONE_URL: &str = "https://api.typesafe.ai/v1/systemone";
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(60);

/// TypeSafe-backed `ai_interface::JudgmentModel` implementation.
#[derive(Clone)]
pub struct TypeSafeJudgmentModel {
    http_client: DynJsonHttpClient,
    model_id: String,
    auth: DynJsonHttpAuth,
    redaction_secrets: Vec<String>,
    endpoint: String,
    timeout: Duration,
}

impl TypeSafeJudgmentModel {
    /// Builds a TypeSafe judgment model from an injected client, model id, and API key.
    pub fn new(
        http_client: DynJsonHttpClient,
        model_id: impl Into<String>,
        api_key: impl Into<String>,
    ) -> Self {
        let api_key = api_key.into();
        let mut model = Self::with_auth(
            http_client,
            model_id,
            Arc::new(StaticHeaderAuth::bearer_token(api_key.clone())),
        );
        model.redaction_secrets.push(api_key);
        model
    }

    /// Builds a TypeSafe judgment model from an injected client, model id, and auth hook.
    pub fn with_auth(
        http_client: DynJsonHttpClient,
        model_id: impl Into<String>,
        auth: DynJsonHttpAuth,
    ) -> Self {
        Self {
            http_client,
            model_id: model_id.into(),
            auth,
            redaction_secrets: Vec::new(),
            endpoint: TYPESAFE_SYSTEM_ONE_URL.to_owned(),
            timeout: DEFAULT_TIMEOUT,
        }
    }

    /// Overrides the System One endpoint.
    pub fn with_endpoint(mut self, endpoint: impl Into<String>) -> Self {
        self.endpoint = endpoint.into();
        self
    }

    /// Overrides the request timeout.
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }
}

#[async_trait]
impl JudgmentModel for TypeSafeJudgmentModel {
    async fn judge(&self, request: &JudgmentRequest) -> JudgmentResult<JudgmentResponse> {
        request.validate()?;
        let mut headers = BTreeMap::new();
        if self.auth.apply_headers(&mut headers).await.is_err() {
            return Err(JudgmentError::transient_provider(
                PROVIDER,
                &self.model_id,
                AUTH_HOOK_FAILED,
            ));
        }
        let secrets = secrets_from_headers(&self.redaction_secrets, &headers);
        let body = build_request(&self.model_id, request);
        let http_request = match self
            .http_client
            .post(&self.endpoint)
            .headers(headers.clone())
            .timeout(self.timeout)
            .json(body)
        {
            Ok(request) => request,
            Err(source) => {
                return Err(classify_request_error(source, &self.model_id, &secrets));
            }
        };
        let response = match http_request.send_bytes().await {
            Ok(response) => response,
            Err(source) => {
                return Err(classify_request_error(source, &self.model_id, &secrets));
            }
        };
        if !(200..300).contains(&response.status) {
            let classification_body = match serde_json::from_slice::<Value>(&response.body) {
                Ok(body) => body,
                Err(_) => Value::String(String::from_utf8_lossy(&response.body).into_owned()),
            };
            let classification_body = if response.body.is_empty() {
                Value::String(http_status_message(response.status))
            } else {
                classification_body
            };
            return Err(classify_status(
                response.status,
                &self.model_id,
                &classification_body,
                &secrets,
            ));
        }
        match parse_response(&self.model_id, request, &response.body) {
            Ok(response) => Ok(response),
            Err(error) => Err(redact_response_error(error, &secrets)),
        }
    }
}

#[cfg(test)]
#[path = "_tests_/support.rs"]
mod test_support;

#[cfg(test)]
#[path = "_tests_/construction_tests.rs"]
mod construction_tests;

#[cfg(test)]
#[path = "_tests_/client_tests.rs"]
mod client_tests;

#[cfg(test)]
#[path = "_tests_/auth_redaction_tests.rs"]
mod auth_redaction_tests;

#[cfg(test)]
#[path = "_tests_/redaction_tests.rs"]
mod redaction_tests;

#[cfg(test)]
#[path = "_tests_/status_tests.rs"]
mod status_tests;
