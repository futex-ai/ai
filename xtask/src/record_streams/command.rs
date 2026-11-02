//! CLI composition root for explicitly authorized live recording.

use std::{path::Path, sync::Arc};

use ai_testkit::{
    provider::{ModelFactory, Provider, ProviderModelFactory},
    runner::{ConformanceRunner, ScenarioRunner},
    storage::{FileRecordingStorage, RecordingStorage},
};

use crate::error::{Error, Result};

use super::recorder::{ProviderStreamRecorder, ReqwestRecordingFactory, StreamRecorder};

/// Runs the opt-in recorder with its environment key and production transports.
pub(crate) fn run(root: &Path, provider: Provider) -> Result<()> {
    let key = match std::env::var("RECORD_API_KEY") {
        Ok(key) if !key.trim().is_empty() => key,
        _ => return Err(Error::MissingRecordApiKey),
    };
    let runtime = match tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
    {
        Ok(runtime) => runtime,
        Err(source) => return Err(Error::RecordRuntime { source }),
    };
    let factory: Arc<dyn ModelFactory + Send + Sync> = Arc::new(ProviderModelFactory);
    let runner: Arc<dyn ConformanceRunner + Send + Sync> =
        Arc::new(ScenarioRunner::new(factory.clone()));
    let storage: Arc<dyn RecordingStorage + Send + Sync> = Arc::new(FileRecordingStorage::new(
        root.join("crates/ai-testkit/recordings"),
    ));
    let recorder: Arc<dyn StreamRecorder + Send + Sync> = Arc::new(ProviderStreamRecorder::new(
        Arc::new(ReqwestRecordingFactory),
        factory,
        runner,
        storage,
    ));
    let summary = match runtime.block_on(recorder.record(provider, &key)) {
        Ok(summary) => summary,
        Err(source) => return Err(Error::RecordStreams { source }),
    };
    println!(
        "{}: wrote {} scenarios; {} scenarios returned an error",
        provider.name(),
        summary.recorded,
        summary.failed
    );
    Ok(())
}
