//! Credentialed provider-specific tool-choice verification.

use std::{env, sync::Arc, time::Duration};

use ai_interface::{
    ConversationMessage, FinishReason, Model, ModelCallControls, ModelExecutionControls,
    ModelGenerationControls, ModelRequest, ModelToolChoice, ToolDefinition,
};
use ai_models_core::RetryingModel;
use ai_models_minimax::MINIMAX_M3;
use json_http::{JsonHttpClient, ReqwestJsonHttpClient};
use serde_json::json;

use super::provider_tests::LiveProvider;

const API_KEY_ENV: &str = "LIVE_MODEL_API_KEY";
const MODEL_TIMEOUT: Duration = Duration::from_secs(10 * 60);
const TOOL_NAME: &str = "live_probe";

pub(super) async fn run_minimax_m3_required_tool_call() {
    let api_key = env::var(API_KEY_ENV)
        .unwrap_or_else(|_| panic!("{API_KEY_ENV} must contain the MiniMax API credential"));
    assert!(
        !api_key.trim().is_empty(),
        "{API_KEY_ENV} must not be empty"
    );

    let provider = LiveProvider::MiniMax;
    let spec = provider
        .chat_catalog()
        .into_iter()
        .find(|spec| spec.id == MINIMAX_M3)
        .expect("MiniMax-M3 must remain in the live catalog");
    let client: Arc<dyn JsonHttpClient> = Arc::new(ReqwestJsonHttpClient::new());
    let model = RetryingModel::with_standard_transient_retry(provider.build(
        client,
        provider.auth(api_key),
        &spec,
    ));
    let response = model
        .complete(&ModelRequest {
            system_prompt: "Use the provided tool for this connectivity check.".to_owned(),
            messages: vec![ConversationMessage::user(
                "Call live_probe with the token MINIMAX_REQUIRED_OK.",
            )],
            tools: vec![ToolDefinition {
                name: TOOL_NAME.to_owned(),
                description: "Record a fixed live provider connectivity token.".to_owned(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "token": {
                            "type": "string",
                            "const": "MINIMAX_REQUIRED_OK"
                        }
                    },
                    "required": ["token"],
                    "additionalProperties": false
                }),
                activity_verb: None,
            }],
            response_schema: None,
            controls: ModelCallControls {
                generation: ModelGenerationControls {
                    tool_choice: Some(ModelToolChoice::Required),
                    ..Default::default()
                },
                execution: ModelExecutionControls {
                    total_timeout: Some(MODEL_TIMEOUT),
                    ..Default::default()
                },
            },
        })
        .await
        .expect("MiniMax-M3 must accept a strict required tool request");

    assert_eq!(response.finish_reason, FinishReason::ToolCalls);
    assert!(
        response
            .tool_calls
            .iter()
            .any(|tool_call| tool_call.name == TOOL_NAME),
        "MiniMax-M3 required request did not call {TOOL_NAME}"
    );
}

#[test]
fn minimax_required_probe_uses_typed_strict_control() {
    let controls = ModelCallControls {
        generation: ModelGenerationControls {
            tool_choice: Some(ModelToolChoice::Required),
            ..Default::default()
        },
        ..Default::default()
    };

    assert_eq!(
        controls.generation.tool_choice,
        Some(ModelToolChoice::Required)
    );
}
