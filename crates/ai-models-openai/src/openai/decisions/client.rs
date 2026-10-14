//! OpenAI Decisions transport client.

use std::{collections::BTreeMap, sync::Arc, time::Duration};

use ai_interface::{
    JudgmentError, JudgmentModel, JudgmentRequest, JudgmentResponse, JudgmentResult,
};
use ai_models_core::{
    AUTH_HOOK_FAILED, classify_judgment_request_error, decode_failure_body, redact_judgment_error,
    secrets_from_headers,
};
use async_trait::async_trait;
use json_http::{DynJsonHttpAuth, DynJsonHttpClient, StaticHeaderAuth};

use super::{error::classify_status, request::build_request, response::parse_response};

const PROVIDER: &str = "openai";
const OPENAI_DECISIONS_URL: &str = "https://api.openai.com/v1/decisions";
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(60);

/// OpenAI Decisions-backed `ai_interface::JudgmentModel` implementation.
#[derive(Clone)]
pub struct OpenAiJudgmentModel {
    http_client: DynJsonHttpClient,
    model_id: String,
    auth: DynJsonHttpAuth,
    redaction_secrets: Vec<String>,
    endpoint: String,
    timeout: Duration,
}

impl OpenAiJudgmentModel {
    /// Builds an OpenAI judgment model from an injected client, provider model id, and API key.
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

    /// Builds an OpenAI judgment model from an injected client, provider model id, and auth hook.
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
            endpoint: OPENAI_DECISIONS_URL.to_owned(),
            timeout: DEFAULT_TIMEOUT,
        }
    }

    /// Overrides the Decisions endpoint.
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
impl JudgmentModel for OpenAiJudgmentModel {
    async fn judge(&self, request: &JudgmentRequest) -> JudgmentResult<JudgmentResponse> {
        request.validate()?;
        let body = build_request(&self.model_id, request)?;
        let mut headers = BTreeMap::new();
        if self.auth.apply_headers(&mut headers).await.is_err() {
            return Err(JudgmentError::transient_provider(
                PROVIDER,
                &self.model_id,
                AUTH_HOOK_FAILED,
            ));
        }
        let secrets = secrets_from_headers(&self.redaction_secrets, &headers);
        let http_request = match self
            .http_client
            .post(&self.endpoint)
            .headers(headers.clone())
            .timeout(self.timeout)
            .json(body)
        {
            Ok(request) => request,
            Err(source) => {
                return Err(classify_judgment_request_error(
                    PROVIDER,
                    source,
                    &self.model_id,
                    &secrets,
                ));
            }
        };
        let response = match http_request.send_bytes().await {
            Ok(response) => response,
            Err(source) => {
                return Err(classify_judgment_request_error(
                    PROVIDER,
                    source,
                    &self.model_id,
                    &secrets,
                ));
            }
        };
        if !(200..300).contains(&response.status) {
            let failure_body = decode_failure_body(response.status, &response.body);
            return Err(classify_status(
                response.status,
                &self.model_id,
                &failure_body,
                &secrets,
            ));
        }
        match parse_response(&self.model_id, request, &response.body) {
            Ok(response) => Ok(response),
            Err(error) => Err(redact_judgment_error(error, &secrets)),
        }
    }
}

#[cfg(test)]
#[path = "_tests_/construction_tests.rs"]
mod construction_tests;

#[cfg(test)]
#[path = "_tests_/client_tests.rs"]
mod client_tests;

#[cfg(test)]
#[path = "_tests_/status_tests.rs"]
mod status_tests;

#[cfg(test)]
#[path = "_tests_/redaction_tests.rs"]
mod redaction_tests;

#[cfg(test)]
#[path = "_tests_/auth_redaction_tests.rs"]
mod auth_redaction_tests;
