//! Shared adapter construction and explicit response facts for error regressions.

use std::{sync::Arc, time::Duration};

use ai_interface::{ConversationMessage, DynModel, ModelError, ModelRequest};
use ai_models_core::Clock;
use json_http::{DynJsonHttpAuth, DynJsonHttpClient, JsonHttpHeaders};
use unimock::Unimock;

use crate::{
    deepseek::DeepSeekModel,
    kimi::KimiModel,
    minimax::{MINIMAX_M3, MiniMaxModel},
    qwen::QwenModel,
    xai::{GROK_4_5, XaiModel},
};

pub(super) fn model(
    provider: &str,
    http: DynJsonHttpClient,
    auth: DynJsonHttpAuth,
    clock: Arc<dyn Clock>,
) -> DynModel {
    match provider {
        "deepseek" => Arc::new(DeepSeekModel::with_auth(http, auth).with_clock(clock)),
        "kimi" => Arc::new(KimiModel::with_auth(http, auth).with_clock(clock)),
        "qwen" => Arc::new(QwenModel::with_auth(http, auth).with_clock(clock)),
        "minimax" => Arc::new(MiniMaxModel::with_auth(http, MINIMAX_M3, auth).with_clock(clock)),
        "xai" => Arc::new(XaiModel::with_auth(http, GROK_4_5, auth).with_clock(clock)),
        _ => panic!("unknown test dialect"),
    }
}

pub(super) fn request() -> ModelRequest {
    ModelRequest {
        system_prompt: String::new(),
        messages: vec![ConversationMessage::user("hello")],
        tools: Vec::new(),
        response_schema: None,
        controls: Default::default(),
    }
}

pub(super) fn headers(id: &str, retry: &str) -> JsonHttpHeaders {
    let mut headers = JsonHttpHeaders::new();
    headers.insert("x-request-id", id.parse().unwrap());
    headers.insert("retry-after", retry.parse().unwrap());
    headers
}

pub(super) fn no_clock() -> Arc<dyn Clock> {
    Arc::new(Unimock::new(()))
}

pub(super) fn assert_metadata(error: &ModelError, id: &str, retry: u64) {
    assert_eq!(error.core().provider_request_id.as_deref(), Some(id));
    assert_eq!(error.core().retry_after, Some(Duration::from_secs(retry)));
}
