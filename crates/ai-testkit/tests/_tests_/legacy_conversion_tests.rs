//! Stored version one tool-loop fixtures keep the recorded continuation request.

use std::{path::PathBuf, sync::Arc};

use ai_interface::{
    ConversationMessage, ConversationRole, DynModel, DynModelEventStream, LegacyMessageConverter,
    Model, ModelRequest, ModelResult,
};
use ai_testkit::{
    provider::{ModelFactory, Provider, ProviderModelFactory},
    runner::{ConformanceRunner, ScenarioRunner},
    scenario::Scenario,
    storage::{FileRecordingStorage, RecordingStorage},
};
use async_trait::async_trait;
use json_http::{DynJsonHttpAuth, DynJsonHttpClient};

#[tokio::test]
async fn converted_version_one_tool_loops_send_the_original_recorded_requests() {
    let storage =
        FileRecordingStorage::new(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("recordings"));
    let runner = ScenarioRunner::new(Arc::new(ConvertedHistoryFactory));
    for provider in Provider::ALL {
        let recording = storage.load(provider, Scenario::ToolLoop).unwrap().unwrap();
        let outcome = runner
            .replay(provider, Scenario::ToolLoop, &recording)
            .await
            .unwrap_or_else(|error| panic!("{} version one replay: {error}", provider.name()));
        assert_eq!(outcome.remaining_exchanges, 0, "{}", provider.name());
        assert_eq!(outcome.steps.len(), 2, "{}", provider.name());
    }
}

fn converter() -> LegacyMessageConverter {
    LegacyMessageConverter::new(vec![
        Arc::new(ai_models_anthropic::AnthropicLegacyConverter),
        Arc::new(ai_models_openai::OpenAiLegacyConverter),
        Arc::new(ai_models_google::GoogleLegacyProviderConverter),
        Arc::new(ai_models_compat::deepseek::LegacyConverter),
        Arc::new(ai_models_compat::kimi::LegacyConverter),
        Arc::new(ai_models_compat::qwen::LegacyConverter),
        Arc::new(ai_models_compat::minimax::LegacyConverter),
        Arc::new(ai_models_compat::xai::LegacyConverter),
    ])
}

struct ConvertedHistoryFactory;

impl ModelFactory for ConvertedHistoryFactory {
    fn build(
        &self,
        provider: Provider,
        id: &str,
        client: DynJsonHttpClient,
        auth: DynJsonHttpAuth,
    ) -> ai_testkit::error::Result<DynModel> {
        let value = serde_json::from_str(legacy_fixture(provider)).unwrap();
        Ok(Arc::new(ConvertedHistoryModel {
            inner: ProviderModelFactory.build(provider, id, client, auth)?,
            assistant: converter().read(value).unwrap(),
        }))
    }
}

struct ConvertedHistoryModel {
    inner: DynModel,
    assistant: ConversationMessage,
}

#[async_trait]
impl Model for ConvertedHistoryModel {
    async fn stream(&self, request: &ModelRequest) -> ModelResult<DynModelEventStream> {
        let mut request = request.clone();
        for message in &mut request.messages {
            if message.role == ConversationRole::Assistant {
                *message = self.assistant.clone();
            }
        }
        self.inner.stream(&request).await
    }
}

fn legacy_fixture(provider: Provider) -> &'static str {
    match provider {
        Provider::Anthropic => include_str!("../fixtures/version1/anthropic_tool_loop.json"),
        Provider::Openai => include_str!("../fixtures/version1/openai_tool_loop.json"),
        Provider::Google => include_str!("../fixtures/version1/google_tool_loop.json"),
        Provider::Deepseek => include_str!("../fixtures/version1/deepseek_tool_loop.json"),
        Provider::Kimi => include_str!("../fixtures/version1/kimi_tool_loop.json"),
        Provider::Qwen => include_str!("../fixtures/version1/qwen_tool_loop.json"),
        Provider::Minimax => include_str!("../fixtures/version1/minimax_tool_loop.json"),
        Provider::Xai => include_str!("../fixtures/version1/xai_tool_loop.json"),
    }
}

#[cfg(test)]
#[path = "legacy_request_support.rs"]
mod legacy_request_support;
#[cfg(test)]
#[path = "minimax_legacy_replay_tests.rs"]
mod minimax_legacy_replay_tests;
#[cfg(test)]
#[path = "mixed_native_replay_tests.rs"]
mod mixed_native_replay_tests;
