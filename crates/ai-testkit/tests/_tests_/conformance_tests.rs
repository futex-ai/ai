//! Snapshots of every available provider and scenario recording.

use std::{path::PathBuf, sync::Arc};

use ai_testkit::{
    provider::{Provider, ProviderModelFactory},
    runner::{ConformanceRunner, ScenarioRunner},
    scenario::Scenario,
    storage::{FileRecordingStorage, RecordingStorage},
};

#[tokio::test]
async fn snapshots_every_available_provider_scenario() {
    let storage: Arc<dyn RecordingStorage + Send + Sync> = Arc::new(FileRecordingStorage::new(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("recordings"),
    ));
    let runner: Arc<dyn ConformanceRunner + Send + Sync> =
        Arc::new(ScenarioRunner::new(Arc::new(ProviderModelFactory)));
    let mut count = 0;
    for provider in Provider::ALL {
        for scenario in Scenario::ALL {
            let Some(recording) = storage.load(provider, scenario).expect("read recording") else {
                continue;
            };
            let outcome = runner
                .replay(provider, scenario, &recording)
                .await
                .unwrap_or_else(|error| panic!("run conformance scenario: {error}"));
            let name = format!("{}_{}", provider.name(), scenario.name());
            insta::with_settings!({description => format!("Today's {} adapter result for {} using {} {} recordings.", provider.name(), scenario.name(), recording.catalog_model_id, match recording.source { ai_testkit::storage::RecordingSource::Synthetic => "synthetic", ai_testkit::storage::RecordingSource::Recorded => "live" })}, {
                insta::assert_json_snapshot!(name, outcome);
            });
            count += 1;
        }
    }
    assert!(count > 0, "conformance must exercise recorded scenarios");
}
