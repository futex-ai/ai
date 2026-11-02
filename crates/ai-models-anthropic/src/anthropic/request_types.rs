//! Typed Anthropic Messages request bodies.

use serde::Serialize;
use serde_json::Value;

#[derive(Debug, Serialize)]
pub(super) struct AnthropicRequest {
    pub(super) stream: bool,
    pub(super) model: String,
    pub(super) max_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) system: Option<String>,
    pub(super) messages: Vec<AnthropicMessage>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(super) tools: Vec<AnthropicTool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) thinking: Option<AnthropicThinking>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) output_config: Option<AnthropicOutputConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) temperature: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) top_p: Option<f64>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(super) stop_sequences: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) tool_choice: Option<AnthropicToolChoice>,
}

#[derive(Debug, Serialize)]
pub(super) struct AnthropicMessage {
    pub(super) role: String,
    pub(super) content: Vec<AnthropicBlock>,
}

#[derive(Debug, Serialize)]
#[serde(tag = "type")]
pub(super) enum AnthropicBlock {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "image")]
    Image { source: AnthropicImageSource },
    #[serde(rename = "tool_use")]
    ToolUse {
        id: String,
        name: String,
        input: Value,
    },
    #[serde(rename = "tool_result")]
    ToolResult {
        #[serde(skip_serializing_if = "Option::is_none")]
        tool_use_id: Option<String>,
        content: String,
    },
}

#[derive(Debug, Serialize)]
pub(super) struct AnthropicImageSource {
    #[serde(rename = "type")]
    pub(super) kind: String,
    pub(super) media_type: String,
    pub(super) data: String,
}

#[derive(Debug, Serialize)]
pub(super) struct AnthropicTool {
    pub(super) name: String,
    pub(super) description: String,
    pub(super) input_schema: Value,
}

#[derive(Debug, Serialize)]
pub(super) struct AnthropicThinking {
    #[serde(rename = "type")]
    pub(super) kind: String,
    pub(super) display: String,
}

#[derive(Debug, Serialize)]
pub(super) struct AnthropicOutputConfig {
    pub(super) effort: String,
}

#[derive(Debug, Serialize)]
pub(super) struct AnthropicToolChoice {
    #[serde(rename = "type")]
    pub(super) kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) name: Option<String>,
}
