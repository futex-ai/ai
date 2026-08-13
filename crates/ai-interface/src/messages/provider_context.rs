//! Provider-specific conversation replay items and their typed payloads.

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
/// Provider-specific conversation item retained for model-specific replay.
pub enum ProviderConversationItem {
    /// OpenAI Responses assistant message metadata used for stateless replay.
    #[serde(rename = "openai_message")]
    OpenAiMessage {
        /// Optional OpenAI assistant message phase, such as `commentary` or `final_answer`.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        phase: Option<String>,
    },
    /// OpenAI Responses reasoning item used for stateless reasoning turns.
    #[serde(rename = "openai_reasoning")]
    OpenAiReasoning {
        /// OpenAI reasoning item identifier.
        id: String,
        /// Provider-supplied visible reasoning summaries.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        summary: Vec<OpenAiReasoningSummary>,
        /// Opaque encrypted reasoning tokens returned by OpenAI.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        encrypted_content: Option<String>,
    },
    /// OpenAI Responses function-call item used for stateless tool turns.
    #[serde(rename = "openai_function_call")]
    OpenAiFunctionCall {
        /// OpenAI function-call output item identifier.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        /// OpenAI function-call identifier.
        call_id: String,
        /// OpenAI function name.
        name: String,
        /// Raw JSON argument string returned by OpenAI.
        arguments: String,
    },
    /// MiniMax assistant reasoning state required for interleaved-thinking replay.
    #[serde(rename = "minimax_assistant")]
    MiniMaxAssistant {
        /// Private reasoning content retained only for provider replay.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        reasoning_content: Option<String>,
        /// Ordered MiniMax reasoning-detail records retained for provider replay.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        reasoning_details: Vec<MiniMaxReasoningDetail>,
    },
    /// xAI legacy chat-completions function call used for continuation replay.
    #[serde(rename = "xai_legacy_function_call")]
    XaiLegacyFunctionCall {
        /// Runtime-local tool-call identifier used to match the tool result.
        tool_call_id: String,
        /// xAI legacy function name.
        name: String,
        /// Raw JSON argument string returned by xAI.
        arguments: String,
    },
    /// DeepSeek Chat Completions assistant message retained for exact continuation replay.
    DeepSeekAssistantMessage {
        /// Assistant content returned by DeepSeek, with provider null normalized to empty.
        content: String,
        /// Optional provider reasoning retained for replay but hidden from visible text.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        reasoning_content: Option<String>,
        /// Ordered raw DeepSeek tool calls retained exactly as returned.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        tool_calls: Vec<DeepSeekToolCallContext>,
    },
    /// Kimi Chat Completions assistant message retained for exact continuation replay.
    #[serde(rename = "kimi_assistant_message")]
    KimiAssistantMessage {
        /// Nullable assistant content returned by Kimi.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        content: Option<String>,
        /// Nullable provider reasoning retained for replay but hidden from visible text.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        reasoning_content: Option<String>,
        /// Ordered raw Kimi tool calls retained exactly as returned.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        tool_calls: Vec<KimiToolCallContext>,
    },
    /// Qwen Chat Completions assistant message retained for exact continuation replay.
    #[serde(rename = "qwen_assistant_message")]
    QwenAssistantMessage {
        /// Nullable assistant content returned by Qwen.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        content: Option<String>,
        /// Nullable provider reasoning retained for replay but hidden from visible text.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        reasoning_content: Option<String>,
        /// Ordered raw Qwen tool calls retained exactly as returned.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        tool_calls: Vec<QwenToolCallContext>,
    },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// One raw DeepSeek function call retained for lossless assistant replay.
pub struct DeepSeekToolCallContext {
    /// Provider-issued tool-call identifier.
    pub id: String,
    /// Function name returned by DeepSeek.
    pub name: String,
    /// Raw JSON argument string returned by DeepSeek.
    pub arguments: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// One raw Kimi function call retained for lossless assistant replay.
pub struct KimiToolCallContext {
    /// Provider-issued tool-call identifier.
    pub id: String,
    /// Function name returned by Kimi.
    pub name: String,
    /// Raw JSON argument string returned by Kimi.
    pub arguments: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// One raw Qwen function call retained for lossless assistant replay.
pub struct QwenToolCallContext {
    /// Provider-issued tool-call identifier.
    pub id: String,
    /// Function name returned by Qwen.
    pub name: String,
    /// Raw JSON argument string returned by Qwen.
    pub arguments: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// One MiniMax reasoning-detail record retained for interleaved-thinking replay.
pub struct MiniMaxReasoningDetail {
    /// Provider reasoning-detail type.
    #[serde(default, rename = "type", skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    /// Provider reasoning-detail identifier.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// Provider reasoning-detail wire format.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
    /// Zero-based provider ordering index.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub index: Option<u32>,
    /// Private reasoning text retained only for provider replay.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// One OpenAI reasoning summary content block.
pub struct OpenAiReasoningSummary {
    /// OpenAI summary block type.
    #[serde(rename = "type")]
    pub kind: String,
    /// Summary text returned by OpenAI.
    pub text: String,
}
