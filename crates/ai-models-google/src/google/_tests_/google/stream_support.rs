//! Shared in-memory SSE fixtures for Google model tests.

use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

use ai_interface::{ConversationPart, Effort, FinishReason, ModelResponse, ModelSpec, ModelUsage};
use ai_models_core::ErrorContext;
use json_http::{
    DynJsonHttpSseStream, JsonHttpClient, JsonHttpHeaders, JsonHttpRequest, JsonHttpSseEvent,
    JsonHttpSseStreamMock, JsonHttpTransportMock, TransportBackedJsonHttpClient,
};
use serde_json::Value;
use unimock::{MockFn, Unimock, matching};

use crate::google::error_context::new;
use crate::{CUSTOM_DEFAULTS, PROVIDER_ID};

pub(super) type RecordedRequests = Arc<Mutex<Vec<JsonHttpRequest>>>;
pub(super) type StreamItem = json_http::Result<Option<JsonHttpSseEvent>>;

pub(in crate::google) fn custom_spec(model_id: &'static str) -> ModelSpec {
    ModelSpec::custom(PROVIDER_ID, model_id, &CUSTOM_DEFAULTS)
}

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

pub(super) fn event(data: Value) -> StreamItem {
    Ok(Some(JsonHttpSseEvent {
        event: None,
        data: data.to_string(),
    }))
}

fn mock_stream(items: Vec<StreamItem>) -> DynJsonHttpSseStream {
    let items = Arc::new(Mutex::new(VecDeque::from(items)));
    Box::new(Unimock::new((
        JsonHttpSseStreamMock::headers
            .each_call(matching!())
            .returns(JsonHttpHeaders::new()),
        JsonHttpSseStreamMock::next
            .each_call(matching!())
            .answers_arc(Arc::new(move |_| {
                items
                    .lock()
                    .expect("stream item lock should not be poisoned")
                    .pop_front()
                    .unwrap_or(Ok(None))
            })),
    )))
}

pub(super) fn expected_response(
    assistant_message: &str,
    finish_reason: FinishReason,
    usage: ModelUsage,
) -> ModelResponse {
    ModelResponse {
        provider: "google".to_owned(),
        model_id: "gemini-3.6-flash".to_owned(),
        catalog_model_id: Some("google/gemini-3.6-flash".to_owned()),
        thinking_level: Some(Effort::Disabled),
        parts: if assistant_message.is_empty() {
            Vec::new()
        } else {
            vec![ConversationPart::text(assistant_message)]
        },
        finish_reason,
        structured_output: None,
        usage,
        attempts: Vec::new(),
    }
}

pub(in crate::google) fn error_context(model: &str) -> ErrorContext {
    new(model, Arc::new(Unimock::new(())))
}
