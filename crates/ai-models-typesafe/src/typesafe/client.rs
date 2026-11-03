//! TypeSafe judgment transport client.

use std::{collections::BTreeMap, sync::Arc, time::Duration};

use ai_interface::{
    ErrorKind, JudgmentError, JudgmentModel, JudgmentRequest, JudgmentResponse, JudgmentResult,
    ModelSpec,
};
use ai_models_core::{ApiKeyScheme, Clock, ErrorContext, ProviderConfig, SystemClock};
use async_trait::async_trait;
use json_http::{DynJsonHttpAuth, DynJsonHttpClient, StaticHeaderAuth};
use serde_json::Value;

use crate::{CUSTOM_DEFAULTS, PROVIDER_ID};

use super::{
    error::{AUTH_HOOK_FAILED, classify_request_error, classify_status, http_status_message},
    redaction::secrets_from_headers,
    request::build_request,
    response::{parse_response, redact_response_error},
};

const PROVIDER: &str = "typesafe";
const TYPESAFE_BASE_URL: &str = "https://api.typesafe.ai";
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(60);
const REQUEST_ID_HEADERS: &[&str] = &["x-request-id", "request-id"];

/// TypeSafe-backed `ai_interface::JudgmentModel` implementation.
#[derive(Clone)]
pub struct TypeSafeJudgmentModel {
    http_client: DynJsonHttpClient,
    model_id: String,
    auth: DynJsonHttpAuth,
    config: ProviderConfig,
    redaction_secrets: Vec<String>,
    endpoint: String,
    timeout: Duration,
    clock: Arc<dyn Clock>,
}

impl TypeSafeJudgmentModel {
    /// Builds a TypeSafe judgment model from shared provider settings.
    pub fn from_config(
        http_client: DynJsonHttpClient,
        spec: ModelSpec,
        config: ProviderConfig,
    ) -> Self {
        Self {
            http_client,
            model_id: spec.provider_model_id.into_owned(),
            auth: config.auth(ApiKeyScheme::Bearer),
            redaction_secrets: Vec::new(),
            endpoint: config.endpoint(TYPESAFE_BASE_URL, "/v1/systemone"),
            timeout: config.timeouts.total_timeout.unwrap_or(DEFAULT_TIMEOUT),
            config,
            clock: Arc::new(SystemClock),
        }
    }

    /// Builds a TypeSafe judgment model from an injected client, model id, and API key.
    pub fn new(
        http_client: DynJsonHttpClient,
        model_id: impl Into<String>,
        api_key: impl Into<String>,
    ) -> Self {
        let api_key = api_key.into();
        let mut model = Self::from_config(
            http_client,
            ModelSpec::custom(PROVIDER_ID, model_id.into(), &CUSTOM_DEFAULTS),
            ProviderConfig::new(PROVIDER_ID, api_key.clone()),
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
        Self::from_config(
            http_client,
            ModelSpec::custom(PROVIDER_ID, model_id.into(), &CUSTOM_DEFAULTS),
            ProviderConfig::with_auth(PROVIDER_ID, auth),
        )
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

    /// Uses an injected wall clock to evaluate response retry dates.
    pub fn with_clock(mut self, clock: Arc<dyn Clock>) -> Self {
        self.clock = clock;
        self
    }
}

#[async_trait]
impl JudgmentModel for TypeSafeJudgmentModel {
    async fn judge(&self, request: &JudgmentRequest) -> JudgmentResult<JudgmentResponse> {
        let context = ErrorContext::new(
            PROVIDER,
            &self.model_id,
            REQUEST_ID_HEADERS,
            self.clock.clone(),
        );
        self.config.capture_secrets(&context);
        let initial_secrets =
            secrets_from_headers(&self.redaction_secrets, &self.config.extra_headers);
        for secret in &initial_secrets {
            context.add_secret(secret.clone());
        }
        if let Err(error) = request.validate() {
            return Err(context.apply_judgment_error(error.with_identity(PROVIDER, &self.model_id)));
        }
        let mut headers = BTreeMap::new();
        if context
            .capture_auth(self.auth.clone())
            .apply_headers(&mut headers)
            .await
            .is_err()
        {
            return Err(JudgmentError::new(
                context.core(ErrorKind::Transient, AUTH_HOOK_FAILED),
            ));
        }
        let secrets = secrets_from_headers(&initial_secrets, &headers);
        for secret in &secrets {
            context.add_secret(secret.clone());
        }
        let body = build_request(&self.model_id, request);
        let http_request = match self
            .config
            .apply_headers(
                self.http_client
                    .post(&self.endpoint)
                    .auth(Arc::new(StaticHeaderAuth::new(headers)))
                    .timeout(self.timeout),
                &self.endpoint,
            )
            .json(body)
        {
            Ok(request) => request,
            Err(source) => {
                return Err(classify_request_error(source, &context, &secrets));
            }
        };
        let response = match http_request.send_bytes().await {
            Ok(response) => response,
            Err(source) => {
                return Err(classify_request_error(source, &context, &secrets));
            }
        };
        let context = context.for_response(&response.headers);
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
                &context,
                response.status,
                &response.headers,
                &classification_body,
                &secrets,
            ));
        }
        match parse_response(&context, &self.model_id, request, &response.body) {
            Ok(response) => Ok(response),
            Err(error) => Err(redact_response_error(error, &context, &secrets)),
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

#[cfg(test)]
#[path = "_tests_/duplicate_key_tests.rs"]
mod duplicate_key_tests;

#[cfg(test)]
#[path = "_tests_/error_context_tests.rs"]
mod error_context_tests;

#[cfg(test)]
#[path = "_tests_/auth_context_tests.rs"]
mod auth_context_tests;

#[cfg(test)]
#[path = "_tests_/config_tests.rs"]
mod config_tests;

#[cfg(test)]
#[path = "_tests_/config_auth_tests.rs"]
mod config_auth_tests;
