//! Every supported catalog effort that needs replay must retain native state.

use std::path::PathBuf;

use ai_interface::{Effort, FinishReason, ModelSpec};
use ai_models_registry::{ProviderConfig, Registry, catalog};
use ai_testkit::{
    provider::Provider,
    scenario::Scenario,
    storage::{FileRecordingStorage, RecordingStorage, ScenarioRecording},
};
use serde_json::Value;

use super::replay_support::{assistant_items, recording_client};

#[tokio::test]
async fn every_catalog_spec_replays_state_at_each_required_effort() {
    let storage =
        FileRecordingStorage::new(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("recordings"));
    let mut cases = 0;
    for spec in catalog().all() {
        let efforts = spec
            .efforts
            .iter()
            .filter(|effort| spec.needs_replay(*effort))
            .collect::<Vec<_>>();
        if efforts.is_empty() {
            continue;
        }
        let provider: Provider = spec
            .provider
            .as_str()
            .parse()
            .expect("chat provider with replay");
        let recording = storage.load(provider, Scenario::ToolLoop).unwrap().unwrap();
        for effort in efforts {
            check_replay(provider, spec, effort, &recording).await;
            cases += 1;
            eprintln!("replayed {} at {}", spec.text_id(), effort.as_str());
        }
    }
    assert_ne!(cases, 0, "catalog must exercise provider replay");
    eprintln!("checked {cases} catalog spec/effort pairs");
}

async fn check_replay(
    provider: Provider,
    spec: &ModelSpec,
    effort: Effort,
    recording: &ScenarioRecording,
) {
    let case = format!("{} tool_loop at {}", spec.text_id(), effort.as_str());
    let (client, log) = recording_client(recording);
    let mut config = ProviderConfig::new(spec.provider.clone(), "conformance-placeholder-key");
    config.base_url = recording.base_url.clone();
    let model = Registry::new(client)
        .with_provider(config)
        .model(spec)
        .unwrap();
    let mut request = Scenario::ToolLoop.request();
    request.controls.generation.effort = Some(effort);
    let first = model
        .complete(&request)
        .await
        .unwrap_or_else(|error| panic!("{case} first response: {error}"));
    assert_eq!(first.finish_reason, FinishReason::ToolCalls, "{case}");
    assert_eq!(
        first.model_id.as_str(),
        spec.provider_model_id.as_ref(),
        "{case}"
    );
    assert_eq!(first.thinking_level, Some(effort), "{case}");
    let mut continuation = Scenario::ToolLoop.follow_up(&first).unwrap();
    continuation.controls.generation.effort = Some(effort);
    let last = model
        .complete(&continuation)
        .await
        .unwrap_or_else(|error| panic!("{case} final response: {error}"));
    assert_eq!(last.finish_reason, FinishReason::Stop, "{case}");
    assert_eq!(last.text(), "Done", "{case}");
    assert_eq!(last.thinking_level, Some(effort), "{case}");
    let exchanges = log.snapshot();
    assert_eq!(exchanges.len(), 2, "{case}");
    let expected = assistant_items(provider, &recording.exchanges[1].request);
    assert_ne!(expected, Vec::<Value>::new(), "{case} replay fixture");
    let actual = assistant_items(provider, &exchanges[1].request);
    assert_eq!(actual, expected, "{case} native continuation state");
}
