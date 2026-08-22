//! Shared in-memory SSE fixtures for OpenAI model tests.

use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

use json_http::{
    DynJsonHttpSseStream, JsonHttpClient, JsonHttpRequest, JsonHttpSseEvent, JsonHttpSseStreamMock,
    JsonHttpTransportMock, TransportBackedJsonHttpClient,
};
use serde_json::{Value, json};
use unimock::{MockFn, Unimock, matching};

pub(super) type RecordedRequests = Arc<Mutex<Vec<JsonHttpRequest>>>;
pub(super) type StreamItem = json_http::Result<Option<JsonHttpSseEvent>>;

pub(super) fn recording_streaming_client(
    streams: Vec<Vec<StreamItem>>,
) -> (Arc<dyn JsonHttpClient>, RecordedRequests) {
    let requests = Arc::new(Mutex::new(Vec::new()));
    let streams = Arc::new(Mutex::new(VecDeque::from(streams)));
    let transport = Arc::new(Unimock::new(
        JsonHttpTransportMock::execute_sse
            .each_call(matching!(_))
            .answers_arc({
                let requests = requests.clone();
                let streams = streams.clone();
                Arc::new(move |_, request: &JsonHttpRequest| {
                    requests
                        .lock()
                        .expect("request lock should not be poisoned")
                        .push(request.clone());
                    let items = streams
                        .lock()
                        .expect("stream lock should not be poisoned")
                        .pop_front()
                        .expect("unexpected streaming transport call");
                    Ok(mock_stream(items))
                })
            }),
    ));
    (
        Arc::new(TransportBackedJsonHttpClient::new(transport)),
        requests,
    )
}

pub(super) fn client_for_buffered_bodies(
    bodies: Vec<Value>,
) -> (Arc<dyn JsonHttpClient>, RecordedRequests) {
    recording_streaming_client(
        bodies
            .into_iter()
            .map(|body| {
                vec![
                    event(
                        "response.created",
                        json!({"type": "response.created", "response": {"status": "in_progress"}}),
                    ),
                    terminal_event("response.completed", body),
                ]
            })
            .collect(),
    )
}

pub(super) fn event(kind: &str, data: Value) -> StreamItem {
    Ok(Some(JsonHttpSseEvent {
        event: Some(kind.to_owned()),
        data: data.to_string(),
    }))
}

pub(super) fn terminal_event(kind: &str, response: Value) -> StreamItem {
    event(kind, json!({"type": kind, "response": response}))
}

fn mock_stream(items: Vec<StreamItem>) -> DynJsonHttpSseStream {
    let items = Arc::new(Mutex::new(VecDeque::from(items)));
    Box::new(Unimock::new(
        JsonHttpSseStreamMock::next
            .each_call(matching!())
            .answers_arc(Arc::new(move |_| {
                items
                    .lock()
                    .expect("stream item lock should not be poisoned")
                    .pop_front()
                    .unwrap_or(Ok(None))
            })),
    ))
}
