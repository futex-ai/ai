//! Agent outcomes and messages from recorded provider exchanges.

use std::path::PathBuf;

use ai_testkit::{
    provider::Provider,
    runner::compare_requests,
    scenario::Scenario,
    storage::{FileRecordingStorage, RecordingStorage},
};

use super::agent_support::execute_agent;

#[tokio::test]
async fn agent_turns_send_recorded_requests_and_snapshot_appended_messages() {
    let storage =
        FileRecordingStorage::new(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("recordings"));
    for provider in Provider::ALL {
        for scenario in Scenario::AGENT {
            let recording = storage
                .load(provider, scenario)
                .expect("read agent recording")
                .unwrap_or_else(|| {
                    panic!("missing {}/{} recording", provider.name(), scenario.name())
                });
            let (outcome, messages, exchanges) =
                execute_agent(provider, scenario, &recording).await;
            compare_requests(provider, scenario, &recording.exchanges, &exchanges)
                .unwrap_or_else(|error| panic!("agent conformance: {error}"));
            let name = format!("agent_{}_{}", provider.name(), scenario.name());
            insta::with_settings!({
                description => format!("{} {} agent outcome and appended messages from synthetic responses.", provider.name(), scenario.name())
            }, {
                insta::assert_debug_snapshot!(name, (outcome, messages));
            });
        }
    }
}
