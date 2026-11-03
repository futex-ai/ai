//! Injectable recording orchestration without retries or provider diagnostics.

use std::sync::Arc;

use ai_interface::ProviderId;
use ai_models_registry::ProviderConfig;
use ai_testkit::{
    error::Result,
    outcome::StepOutcome,
    provider::{ModelFactory, Provider},
    runner::ConformanceRunner,
    scenario::Scenario,
    storage::{RecordingSource, RecordingStorage, ScenarioRecording},
};
use async_trait::async_trait;
use json_http::{
    DynJsonHttpTransport, TransportBackedJsonHttpClient,
    test_support::{MemoryRecordingLog, RecordingLog, RecordingTransport, Redactor},
};

#[cfg_attr(test, unimock::unimock(api = RecordingTransportFactoryMock))]
/// Builds a fresh recording wrapper for one scenario.
pub(super) trait RecordingTransportFactory: Send + Sync {
    /// Uses the supplied log to retain exchanges in call order.
    fn create(&self, log: Arc<dyn RecordingLog + Send + Sync>) -> DynJsonHttpTransport;
}

/// Uses the existing reqwest implementation without changing its behavior.
pub(super) struct ReqwestRecordingFactory;

impl RecordingTransportFactory for ReqwestRecordingFactory {
    fn create(&self, log: Arc<dyn RecordingLog + Send + Sync>) -> DynJsonHttpTransport {
        Arc::new(RecordingTransport::reqwest(log))
    }
}

#[derive(Debug, Eq, PartialEq)]
/// Counts persisted scenarios and model-level failures.
pub(super) struct RecordingSummary {
    /// Number of files written.
    pub(super) recorded: usize,
    /// Number of scenarios with an unsuccessful step.
    pub(super) failed: usize,
}

#[async_trait]
/// Runs only the four success probes, without retrying failed calls.
pub(super) trait StreamRecorder: Send + Sync {
    /// Records a selected provider using its explicit credential.
    async fn record(&self, provider: Provider, key: &str) -> Result<RecordingSummary>;
}

/// Coordinates injected adapters, transport factories, execution, and storage.
pub(super) struct ProviderStreamRecorder {
    transports: Arc<dyn RecordingTransportFactory + Send + Sync>,
    models: Arc<dyn ModelFactory + Send + Sync>,
    runner: Arc<dyn ConformanceRunner + Send + Sync>,
    storage: Arc<dyn RecordingStorage + Send + Sync>,
}

impl ProviderStreamRecorder {
    /// Injects every external and stateful boundary.
    pub(super) fn new(
        transports: Arc<dyn RecordingTransportFactory + Send + Sync>,
        models: Arc<dyn ModelFactory + Send + Sync>,
        runner: Arc<dyn ConformanceRunner + Send + Sync>,
        storage: Arc<dyn RecordingStorage + Send + Sync>,
    ) -> Self {
        Self {
            transports,
            models,
            runner,
            storage,
        }
    }
}

#[async_trait]
impl StreamRecorder for ProviderStreamRecorder {
    async fn record(&self, provider: Provider, key: &str) -> Result<RecordingSummary> {
        let mut summary = RecordingSummary {
            recorded: 0,
            failed: 0,
        };
        let redactor = Redactor::new([key.to_owned()]);
        for scenario in Scenario::SUCCESS {
            let spec = provider.model(scenario)?;
            let log: Arc<dyn RecordingLog + Send + Sync> = Arc::new(MemoryRecordingLog::default());
            let client = Arc::new(TransportBackedJsonHttpClient::new(
                self.transports.create(log.clone()),
            ));
            let model = self.models.build(
                provider,
                &spec.text_id(),
                client,
                ProviderConfig::new(ProviderId::new(provider.name()), key),
            )?;
            let result = self
                .runner
                .execute(model, scenario, Some(spec.default_effort), &redactor)
                .await;
            self.storage.write(
                provider,
                scenario,
                &ScenarioRecording {
                    source: RecordingSource::Recorded,
                    catalog_model_id: spec.text_id(),
                    base_url: None,
                    effort: Some(spec.default_effort),
                    exchanges: log.snapshot(),
                },
            )?;
            let outcome = result?;
            summary.recorded += 1;
            if outcome
                .steps
                .iter()
                .any(|step| !matches!(step, StepOutcome::Response { .. }))
            {
                summary.failed += 1;
            }
        }
        Ok(summary)
    }
}

#[cfg(test)]
#[path = "_tests_/recorder_tests.rs"]
mod recorder_tests;
