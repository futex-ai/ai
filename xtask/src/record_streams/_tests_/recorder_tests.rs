//! Recorder writes each scenario through mocked transport and storage boundaries.

use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

use ai_testkit::{
    provider::{ModelFactory, Provider, ProviderModelFactory},
    runner::{ConformanceRunner, ScenarioRunner},
    scenario::Scenario,
    storage::{RecordingSource, RecordingStorage, RecordingStorageMock, ScenarioRecording},
};
use json_http::{
    DynJsonHttpSseStream, DynJsonHttpTransport, Error, JsonHttpRequest, JsonHttpSseEvent,
    JsonHttpSseStreamMock, JsonHttpTransportMock,
    test_support::{RecordedResponse, RecordingTransport},
};
use serde_json::json;
use unimock::{MockFn, Unimock, matching};

use super::{
    ProviderStreamRecorder, RecordingSummary, RecordingTransportFactory,
    RecordingTransportFactoryMock, StreamRecorder,
};

const KEY: &str = "sentinel-recorder-private-key";

fn fixtures() -> VecDeque<RecordedResponse> {
    let text = [
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../crates/ai-testkit/recordings/openai/tool_loop.json"
        )),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../crates/ai-testkit/recordings/openai/schema_call.json"
        )),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../crates/ai-testkit/recordings/openai/image_input.json"
        )),
    ];
    text.into_iter()
        .flat_map(|text| {
            serde_json::from_str::<ScenarioRecording>(text)
                .expect("fixture")
                .exchanges
                .into_iter()
                .map(|exchange| exchange.response)
        })
        .collect()
}

fn mock_stream(response: RecordedResponse) -> DynJsonHttpSseStream {
    let RecordedResponse::Sse { events, .. } = response else {
        panic!("SSE fixture");
    };
    let events = Mutex::new(VecDeque::from(events));
    Box::new(Unimock::new(
        JsonHttpSseStreamMock::next
            .each_call(matching!())
            .answers_arc(Arc::new(move |_| {
                Ok(events
                    .lock()
                    .expect("events lock")
                    .pop_front()
                    .map(|event| JsonHttpSseEvent {
                        event: event.event,
                        data: event.data.replace("Done", KEY),
                    }))
            })),
    ))
}

#[tokio::test]
async fn writes_all_success_scenarios_and_preserves_errors_without_retries_or_secrets() {
    let responses = Arc::new(Mutex::new(fixtures()));
    let requests = Arc::new(Mutex::new(Vec::<JsonHttpRequest>::new()));
    let requests_for_mock = requests.clone();
    let responses_for_mock = responses.clone();
    let upstream: DynJsonHttpTransport = Arc::new(Unimock::new(
        JsonHttpTransportMock::execute_sse
            .each_call(matching!(_))
            .answers_arc(Arc::new(move |_, request| {
                let mut requests = requests_for_mock.lock().expect("request lock");
                requests.push(request.clone());
                if requests.len() == 1 {
                    return Err(Error::HttpStatus {
                        status: 429,
                        body: json!({"error": {"message": KEY}}),
                    });
                }
                Ok(mock_stream(
                    responses_for_mock
                        .lock()
                        .expect("responses lock")
                        .pop_front()
                        .expect("one fixture per call"),
                ))
            })),
    ));
    let transports: Arc<dyn RecordingTransportFactory + Send + Sync> = Arc::new(Unimock::new(
        RecordingTransportFactoryMock::create
            .each_call(matching!(_))
            .answers_arc(Arc::new(move |_, log| {
                Arc::new(RecordingTransport::new(upstream.clone(), log))
            })),
    ));
    let writes = Arc::new(Mutex::new(Vec::new()));
    let writes_for_mock = writes.clone();
    let storage: Arc<dyn RecordingStorage + Send + Sync> = Arc::new(Unimock::new(
        RecordingStorageMock::write
            .each_call(matching!(_, _, _))
            .answers_arc(Arc::new(move |_, provider, scenario, recording| {
                writes_for_mock.lock().expect("write lock").push((
                    provider,
                    scenario,
                    recording.clone(),
                ));
                Ok(())
            })),
    ));
    let models: Arc<dyn ModelFactory + Send + Sync> = Arc::new(ProviderModelFactory);
    let runner: Arc<dyn ConformanceRunner + Send + Sync> =
        Arc::new(ScenarioRunner::new(models.clone()));
    let recorder: Arc<dyn StreamRecorder + Send + Sync> = Arc::new(ProviderStreamRecorder::new(
        transports, models, runner, storage,
    ));

    let summary = recorder
        .record(Provider::Openai, KEY)
        .await
        .expect("record each scenario");

    assert_eq!(
        summary,
        RecordingSummary {
            recorded: 4,
            failed: 1
        }
    );
    assert_eq!(
        requests.lock().expect("request lock").len(),
        5,
        "the tool loop has two calls and no call is retried"
    );
    assert!(responses.lock().expect("response lock").is_empty());
    let writes = writes.lock().expect("write lock");
    assert_eq!(
        writes
            .iter()
            .map(|(provider, scenario, _)| (*provider, *scenario))
            .collect::<Vec<_>>(),
        Scenario::SUCCESS.map(|scenario| (Provider::Openai, scenario))
    );
    for (_, _, recording) in &*writes {
        assert_eq!(recording.source, RecordingSource::Recorded);
        assert!(
            !serde_json::to_string(recording)
                .expect("JSON")
                .contains(KEY)
        );
        assert_eq!(
            recording.exchanges[0]
                .request
                .headers
                .iter()
                .find(|(name, _)| name.eq_ignore_ascii_case("authorization"))
                .map(|(_, value)| value.as_str()),
            Some("[redacted]")
        );
    }
    assert_eq!(
        writes[0].2.exchanges[0].response,
        RecordedResponse::Json {
            status: 429,
            body: json!({"error": {"message": "[redacted]"}})
        }
    );
}
