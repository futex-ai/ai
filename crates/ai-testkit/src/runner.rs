//! Runs shared scenarios through production adapters and injected transports.

use std::sync::Arc;

use ai_interface::{DynModel, ModelResponse};
use async_trait::async_trait;
use json_http::{
    TransportBackedJsonHttpClient,
    test_support::{Redactor, ReplayControl, ReplayTransport},
};

use crate::{
    error::{ErrorResultExt, Result},
    outcome::{ErrorOutcome, ScenarioOutcome, StepOutcome},
    provider::{ModelFactory, Provider},
    scenario::Scenario,
    storage::ScenarioRecording,
};

/// Shared execution boundary used by snapshot tests and the explicit recorder.
#[async_trait]
pub trait ConformanceRunner: Send + Sync {
    /// Replays a recording through its provider's production adapter.
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
        let client = Arc::new(TransportBackedJsonHttpClient::new(transport.clone()));
        let model = self.factory.build(
            provider,
            &recording.catalog_model_id,
            client,
            provider.auth("conformance-placeholder-key".to_owned()),
        )?;
        let mut outcome = self.execute(model, scenario, &Redactor::default()).await?;
        outcome.remaining_exchanges = cursor.remaining();
        Ok(outcome)
    }

    async fn execute(
        &self,
        model: DynModel,
        scenario: Scenario,
        redactor: &Redactor,
    ) -> Result<ScenarioOutcome> {
        let mut request = scenario.request();
        let mut steps = Vec::new();
        let step_count = if scenario == Scenario::ToolLoop { 2 } else { 1 };
        for step in 0..step_count {
            match model.complete(&request).await {
                Ok(response) => {
                    if scenario == Scenario::ToolLoop && step == 0 {
                        match scenario.follow_up(&response) {
                            Ok(follow_up) => request = follow_up,
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
