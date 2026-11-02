//! Original buffered-response fixtures for moved fallback tests.

use std::sync::{
    Arc, Mutex,
    atomic::{AtomicUsize, Ordering},
};

use ai_interface::{
    FinishReason, Model, ModelError, ModelMock, ModelRequest, ModelResponse, ModelToolChoice,
    ModelUsage, test_support::model_response_stream,
};
use unimock::{MockFn, Unimock, matching};

pub(super) fn control_asserting_model(
    calls: Arc<AtomicUsize>,
    result: std::result::Result<ModelResponse, ModelError>,
) -> Arc<dyn Model> {
    let result = Arc::new(Mutex::new(Some(result)));
    Arc::new(Unimock::new(
        ModelMock::stream
            .next_call(matching!(_))
            .answers_arc(Arc::new(move |_, request: &ModelRequest| {
                assert_eq!(
                    request.controls.generation.tool_choice,
                    Some(ModelToolChoice::RequiredOrAuto)
                );
                calls.fetch_add(1, Ordering::SeqCst);
                result
                    .lock()
                    .expect("result lock should not be poisoned")
                    .take()
                    .expect("unexpected model call")
                    .map(model_response_stream)
            })),
    ))
}

pub(super) fn scripted_model(
    calls: Arc<AtomicUsize>,
    result: std::result::Result<ModelResponse, ModelError>,
) -> Arc<dyn Model> {
    let result = Arc::new(Mutex::new(Some(result)));
    Arc::new(Unimock::new(ModelMock::stream.stub(move |each| {
        let calls = calls.clone();
        let result = result.clone();
        each.call(matching!(_))
            .answers_arc(Arc::new(move |_, _request: &ModelRequest| {
                calls.fetch_add(1, Ordering::SeqCst);
                result
                    .lock()
                    .expect("result lock should not be poisoned")
                    .take()
                    .expect("unexpected model call")
                    .map(model_response_stream)
            }));
    })))
}

pub(super) fn empty_request() -> ModelRequest {
    ModelRequest {
        system_prompt: "system".to_owned(),
        messages: Vec::new(),
        tools: Vec::new(),
        response_schema: None,
        controls: Default::default(),
    }
}

pub(super) fn success_response(provider: &str, model_id: &str) -> ModelResponse {
    ModelResponse {
        provider: provider.to_owned(),
        model_id: model_id.to_owned(),
        catalog_model_id: None,
        thinking_level: None,
        parts: vec![ai_interface::ConversationPart::text("ok".to_owned())],
        finish_reason: FinishReason::Stop,
        structured_output: None,

        usage: ModelUsage::default(),
        attempts: Vec::new(),
    }
}
