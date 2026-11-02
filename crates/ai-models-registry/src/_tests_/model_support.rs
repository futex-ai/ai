//! Included model specs and minimal streams for each native protocol.

use ai_interface::{CustomModelDefaults, ModelSpec};
#[cfg(any(
    feature = "deepseek",
    feature = "kimi",
    feature = "minimax",
    feature = "qwen",
    feature = "xai"
))]
use ai_models_core::test_support::data_event;
use ai_models_core::test_support::{SseFixture, StreamItem, event};
use serde_json::json;

pub(super) enum Wire {
    #[cfg(feature = "anthropic")]
    Anthropic,
    #[cfg(feature = "google")]
    Google,
    #[cfg(feature = "openai")]
    OpenAi,
    #[cfg(any(
        feature = "deepseek",
        feature = "kimi",
        feature = "minimax",
        feature = "qwen",
        feature = "xai"
    ))]
    Compatible,
}

pub(super) struct ModelCase {
    pub(super) known: ModelSpec,
    pub(super) custom: ModelSpec,
    pub(super) path: &'static str,
    pub(super) auth_header: &'static str,
    pub(super) output_pointer: &'static str,
    wire: Wire,
}

impl ModelCase {
    fn new(
        known: ModelSpec,
        defaults: &CustomModelDefaults,
        path: &'static str,
        auth_header: &'static str,
        output_pointer: &'static str,
        wire: Wire,
    ) -> Self {
        let custom = ModelSpec::custom(known.provider.clone(), "registry-unlisted", defaults);
        Self {
            known,
            custom,
            path,
            auth_header,
            output_pointer,
            wire,
        }
    }

    pub(super) fn fixture(&self) -> SseFixture {
        let items: Vec<StreamItem> = match self.wire {
            #[cfg(feature = "anthropic")]
            Wire::Anthropic => vec![
                event(json!({"type":"message_start","message":{"content":[],"usage":{}}})),
                event(
                    json!({"type":"message_delta","delta":{"stop_reason":"end_turn"},"usage":{}}),
                ),
                event(json!({"type":"message_stop"})),
            ],
            #[cfg(feature = "google")]
            Wire::Google => vec![event(json!({"candidates":[{
                "finishReason":"STOP","content":{"parts":[{"text":"Done"}]}
            }]}))],
            #[cfg(feature = "openai")]
            Wire::OpenAi => vec![
                event(json!({"type":"response.created","response":{"status":"in_progress"}})),
                event(
                    json!({"type":"response.completed","response":{"status":"completed","output":[]}}),
                ),
            ],
            #[cfg(any(
                feature = "deepseek",
                feature = "kimi",
                feature = "minimax",
                feature = "qwen",
                feature = "xai"
            ))]
            Wire::Compatible => vec![
                event(
                    json!({"choices":[{"index":0,"finish_reason":"stop","delta":{"content":"Done"}}],"usage":{}}),
                ),
                data_event("[DONE]"),
            ],
        };
        SseFixture::Stream(items)
    }
}

pub(super) fn cases() -> Vec<ModelCase> {
    vec![
        #[cfg(feature = "anthropic")]
        ModelCase::new(
            ai_models_anthropic::CLAUDE_SONNET_5,
            &ai_models_anthropic::CUSTOM_DEFAULTS,
            "/v1/messages",
            "x-api-key",
            "/max_tokens",
            Wire::Anthropic,
        ),
        #[cfg(feature = "deepseek")]
        ModelCase::new(
            ai_models_compat::deepseek::DEEPSEEK_V4_PRO,
            &ai_models_compat::deepseek::CUSTOM_DEFAULTS,
            "/chat/completions",
            "Authorization",
            "/max_tokens",
            Wire::Compatible,
        ),
        #[cfg(feature = "google")]
        ModelCase::new(
            ai_models_google::GEMINI_3_6_FLASH,
            &ai_models_google::CUSTOM_DEFAULTS,
            "/v1beta/models/{model}:streamGenerateContent?alt=sse",
            "x-goog-api-key",
            "/generationConfig/maxOutputTokens",
            Wire::Google,
        ),
        #[cfg(feature = "kimi")]
        ModelCase::new(
            ai_models_compat::kimi::KIMI_K3,
            &ai_models_compat::kimi::CUSTOM_DEFAULTS,
            "/v1/chat/completions",
            "Authorization",
            "/max_completion_tokens",
            Wire::Compatible,
        ),
        #[cfg(feature = "minimax")]
        ModelCase::new(
            ai_models_compat::minimax::MINIMAX_M3,
            &ai_models_compat::minimax::CUSTOM_DEFAULTS,
            "/v1/chat/completions",
            "Authorization",
            "/max_completion_tokens",
            Wire::Compatible,
        ),
        #[cfg(feature = "openai")]
        ModelCase::new(
            ai_models_openai::GPT_5_6_SOL,
            &ai_models_openai::CUSTOM_DEFAULTS,
            "/v1/responses",
            "Authorization",
            "/max_output_tokens",
            Wire::OpenAi,
        ),
        #[cfg(feature = "qwen")]
        ModelCase::new(
            ai_models_compat::qwen::QWEN_3_7_PLUS,
            &ai_models_compat::qwen::CUSTOM_DEFAULTS,
            "/v1/chat/completions",
            "Authorization",
            "/max_completion_tokens",
            Wire::Compatible,
        ),
        #[cfg(feature = "xai")]
        ModelCase::new(
            ai_models_compat::xai::GROK_4_5,
            &ai_models_compat::xai::CUSTOM_DEFAULTS,
            "/v1/chat/completions",
            "Authorization",
            "/max_tokens",
            Wire::Compatible,
        ),
    ]
}
