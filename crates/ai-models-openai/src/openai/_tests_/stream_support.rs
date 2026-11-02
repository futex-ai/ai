//! Shared in-memory SSE fixtures for OpenAI model tests.

use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

use ai_interface::{
    ConversationPart, Effort, FinishReason, ModelResponse, ModelResponseFold, ModelResult,
    ModelSpec, ModelUsage, StructuredOutputSchema,
};
use ai_models_core::ErrorContext;
use json_http::{
    DynJsonHttpSseStream, JsonHttpClient, JsonHttpHeaders, JsonHttpRequest, JsonHttpSseEvent,
    JsonHttpSseStreamMock, JsonHttpTransportMock, TransportBackedJsonHttpClient,
};
use serde_json::{Value, json};
use unimock::{MockFn, Unimock, matching};

use crate::{CUSTOM_DEFAULTS, PROVIDER_ID};

use super::stream;

pub(super) type RecordedRequests = Arc<Mutex<Vec<JsonHttpRequest>>>;
pub(super) type StreamItem = json_http::Result<Option<JsonHttpSseEvent>>;

pub(super) fn custom_spec(model_id: &'static str) -> ModelSpec {
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

pub(super) async fn response_from_fixture(
    catalog_model_id: &str,
    provider_model_id: &str,
    thinking_level: Effort,
    body: Value,
    schema: Option<&StructuredOutputSchema>,
) -> ModelResult<ModelResponse> {
    let mut stream = stream::open(
        mock_stream(vec![terminal_event("response.completed", body)]),
        error_context(provider_model_id),
        catalog_model_id.to_owned(),
        provider_model_id.to_owned(),
        thinking_level,
        schema.cloned(),
    );
    let mut fold = ModelResponseFold::new();
    while let Some(event) = stream.next().await? {
        fold.push(event)?;
    }
    fold.finish()
}

pub(super) fn expected_response(
    assistant_message: &str,
    thinking_level: Effort,
    finish_reason: FinishReason,
    usage: ModelUsage,
) -> ModelResponse {
    ModelResponse {
        provider: "openai".to_owned(),
        model_id: "gpt-5.5".to_owned(),
        catalog_model_id: Some("openai/gpt-5.5".to_owned()),
        thinking_level: Some(thinking_level),
        parts: vec![ConversationPart::text(assistant_message)],
        finish_reason,
        structured_output: None,
        usage,
        attempts: Vec::new(),
    }
}

pub(super) fn replay_states(response: &ModelResponse) -> Vec<super::replay_state::ReplayState> {
    response
        .parts
        .iter()
        .filter_map(|part| part.provider_state())
        .filter(|state| state.provider == PROVIDER_ID)
        .map(|state| serde_json::from_value(Value::Object(state.payload.clone())).unwrap())
        .collect()
}

pub(super) fn state(payload: Value) -> ai_interface::ProviderState {
    ai_interface::ProviderState {
        provider: PROVIDER_ID,
        model: "gpt-5.5".to_owned(),
        version: 1,
        payload: payload.as_object().unwrap().clone(),
    }
}

pub(super) fn error_context(model_id: &str) -> ErrorContext {
    super::error_context::new(model_id, Arc::new(Unimock::new(())))
}
