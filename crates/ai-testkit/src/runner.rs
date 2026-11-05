//! Runs shared scenarios through production adapters and injected transports.

use std::sync::Arc;

use ai_interface::{DynModel, Effort, ModelResponse};
use async_trait::async_trait;
use json_http::{
    TransportBackedJsonHttpClient,
    test_support::{
        MemoryRecordingLog, Recording, RecordingLog, RecordingTransport, Redactor, ReplayControl,
        ReplayTransport,
    },
};

use crate::{
    error::{Error, ErrorResultExt, Result},
    outcome::{ErrorOutcome, ScenarioOutcome, StepOutcome},
    provider::{ModelFactory, Provider},
    scenario::Scenario,
    storage::ScenarioRecording,
};

/// Shared execution boundary used by snapshot tests and the explicit recorder.
#[async_trait]
pub trait ConformanceRunner: Send + Sync {
    /// Replays a recording and compares every sanitized outbound request with its exchange.
    async fn replay(
        &self,
        provider: Provider,
        scenario: Scenario,
        recording: &ScenarioRecording,
    ) -> Result<ScenarioOutcome>;
    /// Runs every step with an already constructed model, without automatic retries.
    async fn execute(
        &self,
        model: DynModel,
        scenario: Scenario,
        effort: Option<Effort>,
        redactor: &Redactor,
    ) -> Result<ScenarioOutcome>;
}

/// Constructs adapters through a replaceable factory.
pub struct ScenarioRunner {
    factory: Arc<dyn ModelFactory + Send + Sync>,
}

impl ScenarioRunner {
    /// Injects the adapter construction boundary.
    pub fn new(factory: Arc<dyn ModelFactory + Send + Sync>) -> Self {
        Self { factory }
    }
}

#[async_trait]
impl ConformanceRunner for ScenarioRunner {
    async fn replay(
        &self,
        provider: Provider,
        scenario: Scenario,
        recording: &ScenarioRecording,
    ) -> Result<ScenarioOutcome> {
        let transport = Arc::new(ReplayTransport::new(recording.exchanges.clone()));
        let cursor: Arc<dyn ReplayControl + Send + Sync> = transport.clone();
        let log: Arc<dyn RecordingLog + Send + Sync> = Arc::new(MemoryRecordingLog::default());
        let client = Arc::new(TransportBackedJsonHttpClient::new(Arc::new(
            RecordingTransport::new(transport, log.clone()),
        )));
        let model = self.factory.build(
            provider,
            &recording.catalog_model_id,
            client,
            provider.auth("conformance-placeholder-key".to_owned()),
        )?;
        let mut outcome = self
            .execute(model, scenario, recording.effort, &Redactor::default())
            .await?;
        compare_requests(provider, scenario, &recording.exchanges, &log.snapshot())?;
        outcome.remaining_exchanges = cursor.remaining();
        Ok(outcome)
    }

    async fn execute(
        &self,
        model: DynModel,
        scenario: Scenario,
        effort: Option<Effort>,
        redactor: &Redactor,
    ) -> Result<ScenarioOutcome> {
        let mut request = scenario.request();
        request.controls.generation.effort = effort;
        let mut steps = Vec::new();
        let step_count = if scenario == Scenario::ToolLoop { 2 } else { 1 };
        for step in 0..step_count {
            match model.complete(&request).await {
                Ok(response) => {
                    if scenario == Scenario::ToolLoop && step == 0 {
                        match scenario.follow_up(&response) {
                            Ok(mut follow_up) => {
                                follow_up.controls.generation.effort = effort;
                                request = follow_up;
                            }
                            Err(_) => {
                                steps.push(StepOutcome::Response {
                                    response: Box::new(sanitized_response(response, redactor)?),
                                });
                                steps.push(StepOutcome::MissingToolCall);
                                break;
                            }
                        }
                    }
                    steps.push(StepOutcome::Response {
                        response: Box::new(sanitized_response(response, redactor)?),
                    });
                }
                Err(error) => {
                    steps.push(StepOutcome::Error {
                        error: ErrorOutcome::capture(error, redactor),
                    });
                    break;
                }
            }
        }
        Ok(ScenarioOutcome {
            steps,
            remaining_exchanges: 0,
        })
    }
}

fn sanitized_response(response: ModelResponse, redactor: &Redactor) -> Result<ModelResponse> {
    let value = serde_json::to_value(response).context("serialize model response")?;
    serde_json::from_value(redactor.value(&value)).context("sanitize model response")
}

fn compare_requests(
    provider: Provider,
    scenario: Scenario,
    expected: &[Recording],
    actual: &[Recording],
) -> Result<()> {
    for exchange_index in 0..expected.len().max(actual.len()) {
        let expected = expected
            .get(exchange_index)
            .map(|exchange| &exchange.request);
        let actual = actual.get(exchange_index).map(|exchange| &exchange.request);
        if actual != expected {
            return Err(Error::RequestMismatch {
                provider,
                scenario,
                exchange_index,
                expected: serde_json::to_string_pretty(&expected)
                    .context("format expected conformance request")?,
                actual: serde_json::to_string_pretty(&actual)
                    .context("format actual conformance request")?,
            });
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "_tests_/runner_tests.rs"]
mod runner_tests;

#[cfg(test)]
#[path = "_tests_/effort_tests.rs"]
mod effort_tests;
