//! Public construction and response fixtures for provider config tests.

use std::sync::Arc;

use ai_interface::{DynModel, Effort, ModelSpec};
use ai_models_core::ProviderConfig;
use json_http::{DynJsonHttpClient, JsonHttpResponse};
use serde_json::{Value, json};

use crate::{
    deepseek::{DEEPSEEK_V4_FLASH, DeepSeekModel},
    kimi::{KIMI_K3, KimiModel},
    minimax::{MINIMAX_M3, MiniMaxModel},
    qwen::{QWEN_3_7_PLUS, QwenModel},
    xai::{GROK_4_5, XaiModel},
};

pub(super) const CASES: &[(ModelSpec, &str, &str, Effort)] = &[
    (
        DEEPSEEK_V4_FLASH,
        "https://api.deepseek.com/chat/completions",
        "/chat/completions",
        Effort::High,
    ),
    (
        KIMI_K3,
        "https://api.moonshot.ai/v1/chat/completions",
        "/v1/chat/completions",
        Effort::Max,
    ),
    (
        QWEN_3_7_PLUS,
        "https://dashscope-intl.aliyuncs.com/compatible-mode/v1/chat/completions",
        "/v1/chat/completions",
        Effort::High,
    ),
    (
        MINIMAX_M3,
        "https://api.minimax.io/v1/chat/completions",
        "/v1/chat/completions",
        Effort::Medium,
    ),
    (
        GROK_4_5,
        "https://api.x.ai/v1/chat/completions",
        "/v1/chat/completions",
        Effort::High,
    ),
];

pub(super) fn model(http: DynJsonHttpClient, spec: ModelSpec, config: ProviderConfig) -> DynModel {
    let provider = spec.provider.clone();
    match provider.as_str() {
        "deepseek" => Arc::new(DeepSeekModel::from_config(http, spec, config)),
        "kimi" => Arc::new(KimiModel::from_config(http, spec, config)),
        "qwen" => Arc::new(QwenModel::from_config(http, spec, config)),
        "minimax" => Arc::new(MiniMaxModel::from_config(http, spec, config)),
        "xai" => Arc::new(XaiModel::from_config(http, spec, config)),
        _ => panic!("unknown config fixture provider"),
    }
}

pub(super) fn response() -> JsonHttpResponse<Value> {
    JsonHttpResponse {
        status: 200,
        headers: Default::default(),
        body: json!({"choices":[{"finish_reason":"stop","message":{"content":"Done"}}]}),
    }
}
