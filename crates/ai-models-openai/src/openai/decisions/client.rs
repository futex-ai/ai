//! OpenAI Decisions transport and input support.

use std::{sync::Arc, time::Duration};

use ai_interface::{
    ConversationPart, JudgmentContent, JudgmentConversationPartKind, JudgmentError, JudgmentInput,
    JudgmentInputForm, JudgmentModel, JudgmentPart, JudgmentPartKind, JudgmentQuestion,
    JudgmentRequest, JudgmentResponse, JudgmentResult, JudgmentSupport, ModelSpec,
};
use ai_models_core::{ApiKeyScheme, ErrorContext, ProviderConfig, SystemClock};
use async_trait::async_trait;
use json_http::DynJsonHttpClient;

use crate::{
    PROVIDER_ID,
    openai::{OPENAI_BASE_URL, error_context},
};

use super::{
    error::{classify_request_error, classify_response, redact_answer_error},
    request::build_request,
    response::parse_response,
};

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(60);
const SUPPORT: JudgmentSupport = JudgmentSupport {
    content_parts: &[
        JudgmentPartKind::Text,
        JudgmentPartKind::Json,
        JudgmentPartKind::Image,
    ],
    input_forms: &[JudgmentInputForm::Content, JudgmentInputForm::Conversation],
    conversation_parts: &[
        JudgmentConversationPartKind::Text,
        JudgmentConversationPartKind::Image,
        JudgmentConversationPartKind::ToolCall,
        JudgmentConversationPartKind::ToolResult,
        JudgmentConversationPartKind::Reasoning,
    ],
};

/// OpenAI-backed judgment model.
#[derive(Clone)]
pub struct OpenAiJudgmentModel {
    http_client: DynJsonHttpClient,
    model_id: String,
    config: ProviderConfig,
    endpoint: String,
    timeout: Duration,
}

impl OpenAiJudgmentModel {
    /// Builds a Decisions adapter from shared provider settings.
    pub fn from_config(
        http_client: DynJsonHttpClient,
        spec: ModelSpec,
        config: ProviderConfig,
    ) -> Self {
        Self {
            http_client,
            model_id: spec.provider_model_id.into_owned(),
            endpoint: config.endpoint(OPENAI_BASE_URL, "/v1/decisions"),
            timeout: config.timeouts.total_timeout.unwrap_or(DEFAULT_TIMEOUT),
            config,
        }
    }

    async fn send(
        &self,
        context: &ErrorContext,
        request: &JudgmentRequest,
    ) -> JudgmentResult<JudgmentResponse> {
        if let Err(error) = request
            .validate()
            .and_then(|()| SUPPORT.check(request, PROVIDER_ID.as_str(), &self.model_id))
        {
            return Err(context.apply_judgment_error(error));
        }
        let body = match build_request(&self.model_id, request) {
            Ok(body) => body,
            Err(source) => {
                return Err(classify_request_error(
                    context,
                    json_http::Error::SerializeRequest { source },
                ));
            }
        };
        let builder = match self
            .config
            .apply_headers(
                self.http_client
                    .post(&self.endpoint)
                    .auth(context.capture_auth(self.config.auth(ApiKeyScheme::Bearer)))
                    .timeout(self.timeout)
                    .without_redirects(),
                &self.endpoint,
            )
            .json(body)
        {
            Ok(builder) => builder,
            Err(source) => return Err(classify_request_error(context, source)),
        };
        let response = match builder.send_bytes().await {
            Ok(response) => response,
            Err(source) => return Err(classify_request_error(context, source)),
        };
        let context = context.for_response(&response.headers);
        if !(200..300).contains(&response.status) {
            return Err(classify_response(&context, response.status, &response.body));
        }
        match parse_response(&context, &self.model_id, request, &response.body) {
            Err(error @ JudgmentError::InvalidAnswer { .. }) => {
                Err(redact_answer_error(&context, error))
            }
            result => result,
        }
    }
}

#[async_trait]
impl JudgmentModel for OpenAiJudgmentModel {
    async fn judge(&self, request: &JudgmentRequest) -> JudgmentResult<JudgmentResponse> {
        let context = error_context::new(&self.model_id, Arc::new(SystemClock));
        self.config.capture_secrets(&context);
        capture_request_images(&context, request);
        match self.send(&context, request).await {
            Ok(response) => Ok(response),
            Err(mut error) => {
                error.core_mut().model_id = context.redact(&self.model_id);
                Err(error)
            }
        }
    }
}

fn capture_request_images(context: &ErrorContext, request: &JudgmentRequest) {
    match &request.input {
        JudgmentInput::Content(content) => capture_content_images(context, content),
        JudgmentInput::Conversation { messages, .. } => {
            for part in messages.iter().flat_map(|message| &message.parts) {
                if let ConversationPart::Image { data_base64, .. } = part {
                    context.add_secret(data_base64);
                }
            }
        }
    }
    for question in request.questions.values() {
        match question {
            JudgmentQuestion::Condition { instructions, .. }
            | JudgmentQuestion::Choice { instructions, .. }
            | JudgmentQuestion::Score { instructions, .. } => {
                capture_content_images(context, instructions);
            }
        }
    }
}

fn capture_content_images(context: &ErrorContext, content: &JudgmentContent) {
    for part in &content.parts {
        if let JudgmentPart::Image { data_base64, .. } = part {
            context.add_secret(data_base64);
        }
    }
}
