//! In-memory Chat Completions SSE fixtures for provider adapter tests.

use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

use json_http::{
    DynJsonHttpSseStream, JsonHttpClient, JsonHttpRequest, JsonHttpResponse, JsonHttpSseEvent,
    JsonHttpSseStreamMock, JsonHttpTransportMock, TransportBackedJsonHttpClient,
};
use serde_json::{Map, Value, json};
use unimock::{MockFn, Unimock, matching};

/// Requests captured by an in-memory streaming transport.
pub type RecordedRequests = Arc<Mutex<Vec<JsonHttpRequest>>>;

/// One result returned by an in-memory SSE stream.
pub type StreamItem = json_http::Result<Option<JsonHttpSseEvent>>;

/// One result returned while opening an in-memory SSE fixture.
pub enum SseFixture {
    /// A successfully opened stream with the supplied items.
    Stream(Vec<StreamItem>),
    /// A failure returned before a stream opens.
    OpeningError(json_http::Error),
}

/// Builds a recording client from ordered SSE fixtures.
pub fn recording_streaming_client(
    fixtures: Vec<SseFixture>,
) -> (Arc<dyn JsonHttpClient>, RecordedRequests) {
    let requests = Arc::new(Mutex::new(Vec::new()));
    let fixtures = Arc::new(Mutex::new(VecDeque::from(fixtures)));
    let transport = Arc::new(Unimock::new(
        JsonHttpTransportMock::execute_sse
            .each_call(matching!(_))
            .answers_arc({
                let requests = requests.clone();
                let fixtures = fixtures.clone();
                Arc::new(move |_, request: &JsonHttpRequest| {
                    requests
                        .lock()
                        .expect("request lock should not be poisoned")
                        .push(request.clone());
                    match fixtures
                        .lock()
                        .expect("fixture lock should not be poisoned")
                        .pop_front()
                        .expect("unexpected streaming transport call")
                    {
                        SseFixture::Stream(items) => Ok(mock_stream(items)),
                        SseFixture::OpeningError(error) => Err(error),
                    }
                })
            }),
    ));
    (
        Arc::new(TransportBackedJsonHttpClient::new(transport)),
        requests,
    )
}

/// Converts buffered provider fixtures into equivalent one-chunk SSE fixtures.
pub fn fixtures_for_buffered_responses(responses: Vec<JsonHttpResponse<Value>>) -> Vec<SseFixture> {
    responses.into_iter().map(buffered_fixture).collect()
}

/// Creates one JSON data event.
pub fn event(data: Value) -> StreamItem {
    data_event(data.to_string())
}

/// Creates one raw SSE data event.
pub fn data_event(data: impl Into<String>) -> StreamItem {
    Ok(Some(JsonHttpSseEvent {
        event: None,
        data: data.into(),
    }))
}

/// Creates the Chat Completions terminal sentinel event.
pub fn done_event() -> StreamItem {
    data_event("[DONE]")
}

fn buffered_fixture(response: JsonHttpResponse<Value>) -> SseFixture {
    if response.status >= 400 {
        return SseFixture::OpeningError(json_http::Error::HttpStatus {
            status: response.status,
            body: response.body,
        });
    }
    SseFixture::Stream(buffered_events(response.body))
}

fn buffered_events(body: Value) -> Vec<StreamItem> {
    let choices = body
        .get("choices")
        .and_then(Value::as_array)
        .map(|choices| {
            choices
                .iter()
                .enumerate()
                .map(|(position, choice)| {
                    json!({
                        "index": choice
                            .get("index")
                            .and_then(Value::as_u64)
                            .unwrap_or(position as u64),
                        "delta": buffered_delta(
                            choice.get("message").cloned().unwrap_or_else(|| json!({}))
                        ),
                        "finish_reason": choice.get("finish_reason").cloned().unwrap_or(Value::Null)
                    })
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let mut chunk = Map::from_iter([("choices".to_owned(), Value::Array(choices))]);
    chunk.insert(
        "usage".to_owned(),
        body.get("usage").cloned().unwrap_or_else(|| json!({})),
    );
    if let Some(base_response) = body.get("base_resp") {
        chunk.insert("base_resp".to_owned(), base_response.clone());
    }
    vec![event(Value::Object(chunk)), done_event()]
}

fn buffered_delta(mut message: Value) -> Value {
    let Some(tool_calls) = message.get_mut("tool_calls").and_then(Value::as_array_mut) else {
        return message;
    };
    for (position, tool_call) in tool_calls.iter_mut().enumerate() {
        if let Some(tool_call) = tool_call.as_object_mut() {
            tool_call
                .entry("index")
                .or_insert_with(|| Value::from(position as u64));
        }
    }
    message
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
