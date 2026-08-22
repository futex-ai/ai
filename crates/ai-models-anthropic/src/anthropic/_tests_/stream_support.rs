//! Shared in-memory SSE fixtures for Anthropic model tests.

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
    recording_streaming_client(bodies.into_iter().map(events_from_buffered_body).collect())
}

pub(super) fn event(kind: &str, data: Value) -> StreamItem {
    Ok(Some(JsonHttpSseEvent {
        event: Some(kind.to_owned()),
        data: data.to_string(),
    }))
}

pub(super) fn events_from_buffered_body(body: Value) -> Vec<StreamItem> {
    let usage = body.get("usage").cloned().unwrap_or_else(|| json!({}));
    let mut events = vec![event(
        "message_start",
        json!({
            "type": "message_start",
            "message": {
                "content": [],
                "usage": {
                    "input_tokens": usage.get("input_tokens").cloned().unwrap_or(json!(0)),
                    "output_tokens": 0,
                    "cache_read_input_tokens": usage
                        .get("cache_read_input_tokens")
                        .cloned()
                        .unwrap_or(json!(0)),
                    "cache_creation_input_tokens": usage
                        .get("cache_creation_input_tokens")
                        .cloned()
                        .unwrap_or(json!(0))
                }
            }
        }),
    )];
    if let Some(content) = body.get("content").and_then(Value::as_array) {
        for (index, block) in content.iter().enumerate() {
            append_block_events(&mut events, index, block);
        }
    }
    events.push(event(
        "message_delta",
        json!({
            "type": "message_delta",
            "delta": {
                "stop_reason": body.get("stop_reason").cloned().unwrap_or(Value::Null)
            },
            "usage": {
                "output_tokens": usage.get("output_tokens").cloned().unwrap_or(json!(0))
            }
        }),
    ));
    events.push(event("message_stop", json!({"type": "message_stop"})));
    events
}

fn append_block_events(events: &mut Vec<StreamItem>, index: usize, block: &Value) {
    let kind = block
        .get("type")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let start = match kind {
        "text" => json!({"type": "text", "text": ""}),
        "tool_use" => json!({
            "type": "tool_use",
            "id": block.get("id").cloned().unwrap_or(Value::Null),
            "name": block.get("name").cloned().unwrap_or(Value::Null),
            "input": {}
        }),
        "thinking" => json!({"type": "thinking", "thinking": "", "signature": ""}),
        _ => block.clone(),
    };
    events.push(event(
        "content_block_start",
        json!({"type": "content_block_start", "index": index, "content_block": start}),
    ));
    let delta = match kind {
        "text" => Some(json!({
            "type": "text_delta",
            "text": block.get("text").cloned().unwrap_or(json!(""))
        })),
        "tool_use" => Some(json!({
            "type": "input_json_delta",
            "partial_json": block.get("input").cloned().unwrap_or(json!({})).to_string()
        })),
        "thinking" => Some(json!({
            "type": "thinking_delta",
            "thinking": block.get("thinking").cloned().unwrap_or(json!(""))
        })),
        _ => None,
    };
    if let Some(delta) = delta {
        events.push(event(
            "content_block_delta",
            json!({"type": "content_block_delta", "index": index, "delta": delta}),
        ));
    }
    if kind == "thinking"
        && let Some(signature) = block.get("signature")
    {
        events.push(event(
            "content_block_delta",
            json!({
                "type": "content_block_delta",
                "index": index,
                "delta": {"type": "signature_delta", "signature": signature}
            }),
        ));
    }
    events.push(event(
        "content_block_stop",
        json!({"type": "content_block_stop", "index": index}),
    ));
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
