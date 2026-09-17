//! TypeSafe judgment transport client.

use std::{sync::Arc, time::Duration};

use ai_interface::{JudgmentModel, JudgmentRequest, JudgmentResponse, JudgmentResult};
use async_trait::async_trait;
use json_http::{DynJsonHttpAuth, DynJsonHttpClient, StaticHeaderAuth};

use super::{
    error::{classify_request_error, classify_status},
    request::build_request,
    response::parse_response,
};

const TYPESAFE_SYSTEM_ONE_URL: &str = "https://api.typesafe.ai/v1/systemone";
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(60);

/// TypeSafe-backed `ai_interface::JudgmentModel` implementation.
#[derive(Clone)]
pub struct TypeSafeJudgmentModel {
    http_client: DynJsonHttpClient,
    model_id: String,
    auth: DynJsonHttpAuth,
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
        Self::with_auth(
            http_client,
            model_id,
            Arc::new(StaticHeaderAuth::bearer_token(api_key)),
        )
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
        let body = build_request(&self.model_id, request);
        let http_request = match self
            .http_client
            .post(&self.endpoint)
            .auth(self.auth.clone())
            .timeout(self.timeout)
            .json(body)
        {
            Ok(request) => request,
            Err(source) => return Err(classify_request_error(source, &self.model_id)),
        };
        let response = match http_request.send_value().await {
            Ok(response) => response,
            Err(source) => return Err(classify_request_error(source, &self.model_id)),
        };
        if response.status >= 400 {
            return Err(classify_status(
                response.status,
                &self.model_id,
                &response.body,
            ));
        }
        parse_response(&self.model_id, request, response.body)
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
