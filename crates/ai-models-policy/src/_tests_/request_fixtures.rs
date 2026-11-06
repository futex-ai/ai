//! Full portable requests and model-boundary request capture.

use std::{
    collections::VecDeque,
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

use ai_interface::{
    ConversationMessage, DynModel, DynModelEventStream, Effort, ModelCompletionMode, ModelMock,
    ModelRequest, ModelResult, ModelToolChoice, StructuredOutputSchema, ToolDefinition,
};
use serde_json::json;
use unimock::{MockFn, Unimock, matching};

pub(super) fn controlled_request() -> ModelRequest {
    let mut request = super::wrapper_fixtures::request();
    request.system_prompt = " authored system ".to_owned();
    request.messages = vec![ConversationMessage::user("retained request")];
    request.tools = vec![ToolDefinition {
        name: "search".to_owned(),
        description: "Find".to_owned(),
        input_schema: json!({"type":"object"}),
        activity_verb: Some("Search".to_owned()),
    }];
    request.response_schema = Some(StructuredOutputSchema {
        name: "result".to_owned(),
        schema: json!({"type":"object"}),
    });
    request.controls.generation.effort = Some(Effort::High);
    request.controls.generation.temperature = Some(0.3);
    request.controls.generation.top_p = Some(0.6);
    request.controls.generation.max_output_tokens = Some(91);
    request.controls.generation.stop_sequences = vec!["last".to_owned(), "first".to_owned()];
    request.controls.generation.tool_choice = Some(ModelToolChoice::RequiredOrAuto);
    request.controls.execution.total_timeout = Some(Duration::from_millis(17));
    request.controls.execution.completion_mode = ModelCompletionMode::PreferDeferred;
    request
}

pub(super) fn capturing_model(
    results: Vec<ModelResult<DynModelEventStream>>,
    requests: Arc<Mutex<Vec<ModelRequest>>>,
    calls: Arc<AtomicUsize>,
) -> DynModel {
    let results = Mutex::new(VecDeque::from(results));
    Arc::new(Unimock::new(
        ModelMock::stream
            .each_call(matching!(_))
            .answers_arc(Arc::new(move |_, request: &ModelRequest| {
                requests.lock().unwrap().push(request.clone());
                calls.fetch_add(1, Ordering::SeqCst);
                results
                    .lock()
                    .unwrap()
                    .pop_front()
                    .expect("unexpected open")
            })),
    ))
}
