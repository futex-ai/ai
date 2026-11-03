//! Google video submission, polling transport, and origin-scoped sample downloads.

use ai_interface::{
    ErrorKind, VideoGenerationError, VideoGenerationRequest, VideoGenerationResult,
};
use ai_models_core::{ApiKeyScheme, ErrorContext, SuccessfulResponse};
use json_http::{JsonHttpRequestBuilder, JsonHttpResponse};
use serde_json::Value;
use url::Url;

use super::{
    client::GoogleVideoGenerator, deadline::VideoDeadline, error::classify_status,
    request::build_request,
};

impl GoogleVideoGenerator {
    async fn send_json(
        &self,
        context: &ErrorContext,
        deadline: &VideoDeadline<'_>,
        request: JsonHttpRequestBuilder,
    ) -> VideoGenerationResult<JsonHttpResponse<Value>> {
        let response = match request.send_value().await {
            Ok(response) => response,
            Err(source) => return Err(deadline.request_error(context, source)),
        };
        match SuccessfulResponse::check(response.status, response) {
            Ok(response) => Ok(response.into_inner()),
            Err(response) => Err(classify_status(
                context,
                response.status,
                &response.headers,
                &response.body,
            )),
        }
    }

    pub(super) async fn submit(
        &self,
        context: &ErrorContext,
        request: &VideoGenerationRequest,
        deadline: &VideoDeadline<'_>,
    ) -> VideoGenerationResult<JsonHttpResponse<Value>> {
        let timeout = deadline.remaining(context)?;
        let endpoint = format!(
            "{}/models/{}:predictLongRunning",
            self.base_url, self.model_id
        );
        let builder = match self
            .config
            .apply_headers(
                self.http_client
                    .post(&endpoint)
                    .auth(
                        context
                            .capture_auth(self.config.auth(ApiKeyScheme::Header("x-goog-api-key"))),
                    )
                    .timeout(timeout),
                &self.base_url,
            )
            .json(build_request(context, request)?)
        {
            Ok(builder) => builder,
            Err(source) => return Err(deadline.request_error(context, source)),
        };
        self.send_json(context, deadline, builder).await
    }

    pub(super) async fn retrieve(
        &self,
        context: &ErrorContext,
        operation: &str,
        deadline: &VideoDeadline<'_>,
    ) -> VideoGenerationResult<JsonHttpResponse<Value>> {
        self.send_json(
            context,
            deadline,
            self.config.apply_headers(
                self.http_client
                    .get(&format!("{}/{}", self.base_url, operation))
                    .auth(
                        context
                            .capture_auth(self.config.auth(ApiKeyScheme::Header("x-goog-api-key"))),
                    )
                    .timeout(deadline.remaining(context)?),
                &self.base_url,
            ),
        )
        .await
    }

    pub(super) async fn download(
        &self,
        context: &ErrorContext,
        uri: &str,
        deadline: &VideoDeadline<'_>,
    ) -> VideoGenerationResult<Vec<u8>> {
        let timeout = deadline.remaining(context)?;
        if !allowed_download_uri(&self.base_url, uri) {
            return Err(VideoGenerationError::new(context.core(
                ErrorKind::InvalidOutput,
                "provider returned a download URI outside the configured HTTPS host",
            )));
        }
        let response = self
            .config
            .apply_headers(
                self.http_client
                    .get(uri)
                    .auth_for_origin(
                        context
                            .capture_auth(self.config.auth(ApiKeyScheme::Header("x-goog-api-key"))),
                        self.base_url.clone(),
                    )
                    .timeout(timeout),
                &self.base_url,
            )
            .send_bytes()
            .await;
        let response = match response {
            Ok(response) => response,
            Err(source) => return Err(deadline.request_error(context, source)),
        };
        let response = match SuccessfulResponse::check(response.status, response) {
            Ok(response) => response.into_inner(),
            Err(response) => {
                return Err(classify_status(
                    context,
                    response.status,
                    &response.headers,
                    &Value::String(String::from_utf8_lossy(&response.body).into_owned()),
                ));
            }
        };
        let context = context.for_response(&response.headers);
        if response.body.is_empty() {
            return Err(VideoGenerationError::NoVideo {
                core: Box::new(context.core(ErrorKind::InvalidOutput, "no video returned")),
            });
        }
        Ok(response.body)
    }
}

fn allowed_download_uri(base_url: &str, uri: &str) -> bool {
    let (Ok(base), Ok(sample)) = (Url::parse(base_url), Url::parse(uri)) else {
        return false;
    };
    sample.scheme() == "https" && sample.host().is_some() && sample.host() == base.host()
}
