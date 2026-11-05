//! Typed provider chunk fields before accumulation.

use serde::Deserialize;

use super::types::ChatCompletionsUsage;

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct ChatCompletionsStreamChunk {
    #[serde(default)]
    pub(crate) choices: Vec<ChatCompletionsChoiceDelta>,
    #[serde(default)]
    pub(crate) usage: Option<ChatCompletionsUsage>,
}

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct ChatCompletionsChoiceDelta {
    #[serde(default)]
    pub(crate) index: u64,
    #[serde(default)]
    pub(crate) delta: ChatCompletionsMessageDelta,
    #[serde(default)]
    pub(crate) finish_reason: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub(crate) struct ChatCompletionsMessageDelta {
    #[serde(default)]
    pub(crate) content: Option<String>,
    #[serde(default)]
    pub(crate) reasoning_content: Option<String>,
    #[serde(default)]
    pub(crate) tool_calls: Vec<ChatCompletionsToolCallDelta>,
    #[serde(default)]
    pub(crate) function_call: Option<ChatCompletionsToolFunctionDelta>,
}

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct ChatCompletionsToolCallDelta {
    #[serde(default)]
    pub(crate) index: u64,
    #[serde(default)]
    pub(crate) id: Option<String>,
    #[serde(default)]
    pub(crate) function: ChatCompletionsToolFunctionDelta,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub(crate) struct ChatCompletionsToolFunctionDelta {
    #[serde(default)]
    pub(crate) name: Option<String>,
    #[serde(default)]
    pub(crate) arguments: Option<String>,
}
