//! Round trip of persisted recording metadata and exchanges.

use ai_testkit::{
    provider::Provider,
    scenario::Scenario,
    storage::{FileRecordingStorage, RecordingSource, RecordingStorage, ScenarioRecording},
};

#[test]
fn round_trips_recordings_and_reports_absent_scenarios() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let storage = FileRecordingStorage::new(directory.path().to_path_buf());
    assert_eq!(
        storage
            .load(Provider::Openai, Scenario::TextTurn)
            .expect("absent file"),
        None
    );
    let recording = ScenarioRecording {
        source: RecordingSource::Recorded,
        catalog_model_id: "openai/catalog-id".to_owned(),
        effort: None,
        exchanges: vec![],
    };
    storage
        .write(Provider::Openai, Scenario::TextTurn, &recording)
        .expect("write recording");
    let loaded = storage
        .load(Provider::Openai, Scenario::TextTurn)
        .expect("load recording");
    assert_eq!(loaded, Some(recording));
    let text = std::fs::read_to_string(storage.path(Provider::Openai, Scenario::TextTurn))
        .expect("JSON file");
    assert!(text.contains("\"source\": \"recorded\""));
}
