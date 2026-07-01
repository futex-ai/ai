//! Credential-free runtime construction smoke test.

use std::sync::Arc;

use ai_interface::{DynModel, MockModel, NoopLogger, Tool, ToolDefinition};
use ai_models_anthropic::{AnthropicModel, CLAUDE_SONNET_4_6};
use ai_models_google::{GEMINI_2_5_PRO, GoogleModel};
use ai_models_openai::{GPT_5_5, OpenAiAudioTranscriber, OpenAiModel};
use ai_models_xai::{GROK_4_20_REASONING, XaiModel};
use ai_tool_calling::ToolCallingRuntime;
use async_trait::async_trait;
use json_http::{JsonHttpClient, ReqwestJsonHttpClient};
use serde_json::{Value, json};

use crate::error::{Error, Result};

pub(crate) fn run() -> Result<()> {
    let client: Arc<dyn JsonHttpClient> = Arc::new(ReqwestJsonHttpClient::new());
    let _anthropic = AnthropicModel::new(client.clone(), CLAUDE_SONNET_4_6, "anthropic-key");
    let _google = GoogleModel::new(client.clone(), GEMINI_2_5_PRO, "google-key");
    let _openai = OpenAiModel::new(client.clone(), GPT_5_5, "openai-key");
    let _xai = XaiModel::new(client, GROK_4_20_REASONING, "xai-key");
    let _transcriber = OpenAiAudioTranscriber::new("gpt-4o-mini-transcribe", "openai-key");

    let model: DynModel = Arc::new(MockModel::new("mock"));
    let tool: Arc<dyn Tool> = Arc::new(SmokeTool);
    match ToolCallingRuntime::new(
        "Use registered tools when helpful.",
        model,
        Arc::new(NoopLogger),
        vec![tool],
    ) {
        Ok(_runtime) => Ok(()),
        Err(source) => Err(Error::SmokeRuntime { source }),
    }
}

struct SmokeTool;

#[async_trait]
impl Tool for SmokeTool {
    fn definitions(&self) -> Vec<ToolDefinition> {
        vec![ToolDefinition {
            name: "smoke_echo".to_owned(),
            description: "Return the provided payload.".to_owned(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "message": { "type": "string" }
                }
            }),
            activity_verb: Some("Checking".to_owned()),
        }]
    }

    async fn call(&self, tool_name: &str, input: Value) -> ai_interface::ToolResult<Value> {
        let _ = tool_name;
        Ok(input)
    }

    fn group_for_tool(&self, tool_name: &str) -> Option<&'static str> {
        let _ = tool_name;
        Some("smoke")
    }
}
