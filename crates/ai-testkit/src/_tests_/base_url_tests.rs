//! Recorded base URLs control public scenario replay without changing exchanges.

use std::sync::Arc;

use crate::{
    outcome::StepOutcome,
    provider::{Provider, ProviderModelFactory},
    runner::{ConformanceRunner, ScenarioRunner},
    scenario::Scenario,
    storage::ScenarioRecording,
};

#[tokio::test]
async fn replay_uses_recorded_base_prefix_with_or_without_a_trailing_slash() {
    for (base_url, expected_url) in [
        (None, "https://api.openai.com/v1/responses"),
        (
            Some("https://gateway.test/provider"),
            "https://gateway.test/provider/v1/responses",
        ),
        (
            Some("https://gateway.test/provider/"),
            "https://gateway.test/provider/v1/responses",
        ),
    ] {
        let mut recording: ScenarioRecording =
            serde_json::from_str(include_str!("../../recordings/openai/text_turn.json")).unwrap();
        recording.base_url = base_url.map(str::to_owned);
        recording.exchanges[0].request.url = expected_url.to_owned();
        let runner = ScenarioRunner::new(Arc::new(ProviderModelFactory));

        let result = runner
            .replay(Provider::Openai, Scenario::TextTurn, &recording)
            .await;

        assert_eq!(
            result
                .as_ref()
                .ok()
                .map(|outcome| outcome.remaining_exchanges),
            Some(0),
            "base={base_url:?}: {result:?}"
        );
        let outcome = result.unwrap();
        assert_eq!(outcome.steps.len(), 1);
        let text = match &outcome.steps[0] {
            StepOutcome::Response { response, .. } => Some(response.text()),
            _ => None,
        };
        assert_eq!(text.as_deref(), Some("Done"));
    }
}
