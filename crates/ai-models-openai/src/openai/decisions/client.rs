//! OpenAI Decisions judgment client.

use std::{sync::Arc, time::Duration};

use ai_interface::{
    JudgmentError, JudgmentModel, JudgmentRequest, JudgmentResponse, JudgmentResult,
};
use ai_models_core::{
    HttpJudgmentTransport, JudgmentEndpoint, JudgmentTransport, redact_judgment_error,
};
use async_trait::async_trait;
use json_http::{DynJsonHttpAuth, DynJsonHttpClient, StaticHeaderAuth};

use super::{error::failure_message, request::build_request, response::parse_response};

/// Decisions endpoint, default timeout, and failure-message extractor.
const OPENAI_DECISIONS_ENDPOINT: JudgmentEndpoint = JudgmentEndpoint {
    provider: "openai",
    url: "https://api.openai.com/v1/decisions",
    timeout: Duration::from_secs(60),
    failure_message,
};

/// OpenAI Decisions-backed `ai_interface::JudgmentModel` implementation.
#[derive(Clone)]
pub struct OpenAiJudgmentModel {
    transport: HttpJudgmentTransport,
}

impl OpenAiJudgmentModel {
    /// Builds an OpenAI judgment model from an injected client, provider model id, and API key.
    pub fn new(
        http_client: DynJsonHttpClient,
        model_id: impl Into<String>,
        api_key: impl Into<String>,
    ) -> Self {
        let api_key = api_key.into();
        let model = Self::with_auth(
            http_client,
            model_id,
            Arc::new(StaticHeaderAuth::bearer_token(api_key.clone())),
        );
        Self {
            transport: model.transport.with_explicit_secret(api_key),
        }
    }

    /// Builds an OpenAI judgment model from an injected client, provider model id, and auth hook.
    pub fn with_auth(
        http_client: DynJsonHttpClient,
        model_id: impl Into<String>,
        auth: DynJsonHttpAuth,
    ) -> Self {
        Self {
            transport: HttpJudgmentTransport::new(
                OPENAI_DECISIONS_ENDPOINT,
                model_id,
                http_client,
                auth,
            ),
        }
    }

    /// Overrides the Decisions endpoint.
    pub fn with_endpoint(self, endpoint: impl Into<String>) -> Self {
        Self {
            transport: self.transport.with_endpoint(endpoint),
        }
    }

    /// Overrides the request timeout.
    pub fn with_timeout(self, timeout: Duration) -> Self {
        Self {
            transport: self.transport.with_timeout(timeout),
        }
    }
}

#[async_trait]
impl JudgmentModel for OpenAiJudgmentModel {
    async fn judge(&self, request: &JudgmentRequest) -> JudgmentResult<JudgmentResponse> {
        request.validate()?;
        let model_id = self.transport.model_id();
        let body = build_request(model_id, request)?;
        let body = match serde_json::to_value(body) {
            Ok(body) => body,
            Err(source) => return Err(JudgmentError::internal(source)),
        };
        let response = self.transport.post(body).await?;
        match parse_response(model_id, request, &response.body) {
            Ok(judgment) => Ok(judgment),
            Err(error) => Err(redact_judgment_error(error, &response.secrets)),
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
