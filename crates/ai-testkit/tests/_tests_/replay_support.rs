//! Response-only playback lets every catalog spec use the same provider fixture.

use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

use ai_testkit::{provider::Provider, storage::ScenarioRecording};
use json_http::{
    DynJsonHttpClient, DynJsonHttpSseStream, JsonHttpSseEvent, JsonHttpSseStreamMock,
    JsonHttpTransportMock, TransportBackedJsonHttpClient,
    test_support::{
        MemoryRecordingLog, RecordedBody, RecordedRequest, RecordedResponse, RecordingLog,
        RecordingTransport, StreamEnd,
    },
};
use serde_json::Value;
use unimock::{MockFn, Unimock, matching};

pub(super) fn recording_client(
    recording: &ScenarioRecording,
) -> (DynJsonHttpClient, Arc<dyn RecordingLog + Send + Sync>) {
    let streams = Mutex::new(
        recording
            .exchanges
            .iter()
            .map(|exchange| recorded_stream(&exchange.response))
            .collect::<VecDeque<_>>(),
    );
    let transport = Arc::new(Unimock::new(
        JsonHttpTransportMock::execute_sse
            .some_call(matching!(_))
            .answers_arc(Arc::new(move |_, _| {
                Ok(streams
                    .lock()
                    .unwrap()
                    .pop_front()
                    .expect("recorded SSE response"))
            })),
    ));
    let log: Arc<dyn RecordingLog + Send + Sync> = Arc::new(MemoryRecordingLog::default());
    let client = Arc::new(TransportBackedJsonHttpClient::new(Arc::new(
        RecordingTransport::new(transport, log.clone()),
    )));
    (client, log)
}

fn recorded_stream(response: &RecordedResponse) -> DynJsonHttpSseStream {
    let RecordedResponse::Sse {
        status,
        events,
        end,
        ..
    } = response
    else {
        panic!("tool-loop fixture must contain SSE responses");
    };
    assert_eq!(*status, 200);
    assert_eq!(*end, StreamEnd::Done);
    let events = Mutex::new(VecDeque::from(events.clone()));
    Box::new(Unimock::new(
        JsonHttpSseStreamMock::next
            .each_call(matching!())
            .answers_arc(Arc::new(move |_| {
                Ok(events
                    .lock()
                    .unwrap()
                    .pop_front()
                    .map(|event| JsonHttpSseEvent {
                        event: event.event,
                        data: event.data,
                    }))
            })),
    ))
}

pub(super) fn assistant_items(provider: Provider, request: &RecordedRequest) -> Vec<Value> {
    let Some(RecordedBody::Json(body)) = &request.body else {
        panic!("tool-loop request must contain JSON");
    };
    let items = match provider {
        Provider::Openai => &body["input"],
        Provider::Google => &body["contents"],
        _ => &body["messages"],
    };
    items
        .as_array()
        .expect("conversation array")
        .iter()
        .filter(|item| match provider {
            Provider::Openai => {
                item["role"] == "assistant"
                    || item["type"] == "reasoning"
                    || item["type"] == "function_call"
            }
            Provider::Google => item["role"] == "model",
            _ => item["role"] == "assistant",
        })
        .cloned()
        .collect()
}
