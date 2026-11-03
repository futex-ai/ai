//! Mocked model streams and pure fixture values for agent contracts.

use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

use ai_interface::{
    ConversationMessage, DynModel, DynModelEventStream, DynTool, FinishReason, ModelMock,
    ModelRequest, ModelResponse, ModelResult, ModelUsage, NoopLogger, ToolCall,
};
use ai_tool_calling::{
    AgentEvent, AgentEventStream, AgentRequest, InMemoryToolOutputStore, RunOutcome,
    ToolCallingRuntime, ToolOutputPolicy,
};
use serde_json::{Value, json};
use unimock::{MockFn, Unimock, matching};

pub(super) type Requests = Arc<Mutex<Vec<ModelRequest>>>;

pub(super) fn model(streams: Vec<ModelResult<DynModelEventStream>>) -> (DynModel, Requests) {
    let streams = Arc::new(Mutex::new(VecDeque::from(streams)));
    let requests = Arc::new(Mutex::new(Vec::new()));
    let model = Unimock::new(ModelMock::stream.each_call(matching!(_)).answers_arc({
        let requests = requests.clone();
        Arc::new(move |_, request: &ModelRequest| {
            requests.lock().unwrap().push(request.clone());
            streams
                .lock()
                .unwrap()
                .pop_front()
                .expect("one stream per model call")
        })
    }));
    (Arc::new(model), requests)
}

pub(super) fn runtime(model: DynModel, tools: Vec<DynTool>) -> ToolCallingRuntime {
    ToolCallingRuntime::new(
        model,
        Arc::new(NoopLogger),
        tools,
        Arc::new(InMemoryToolOutputStore::new()),
        ToolOutputPolicy::default(),
    )
    .unwrap()
}

pub(super) fn request(max_steps: Option<usize>) -> AgentRequest {
    AgentRequest {
        system_prompt: "Use the tools.".to_owned(),
        max_steps,
        ..Default::default()
    }
}

pub(super) fn response(
    text: &str,
    calls: Vec<ToolCall>,
    finish_reason: FinishReason,
) -> ModelResponse {
    ModelResponse {
        provider: "test".to_owned(),
        model_id: "test-model".to_owned(),
        catalog_model_id: None,
        thinking_level: None,
        parts: ConversationMessage::assistant(text, calls).parts,
        finish_reason,
        structured_output: None,
        usage: ModelUsage::default(),
        attempts: Vec::new(),
    }
}

pub(super) fn call(id: &str, name: &str) -> ToolCall {
    ToolCall {
        id: id.to_owned(),
        name: name.to_owned(),
        input: json!({}),
        operation_id: None,
    }
}

pub(super) async fn collect(
    stream: &mut dyn AgentEventStream,
) -> ai_tool_calling::Result<Vec<AgentEvent>> {
    let mut events = Vec::new();
    while let Some(event) = stream.next().await? {
        events.push(event);
    }
    Ok(events)
}

pub(super) fn outcome(events: &[AgentEvent]) -> &RunOutcome {
    let Some(AgentEvent::Finished(outcome)) = events.last() else {
        panic!("missing terminal event: {events:?}")
    };
    outcome
}

pub(super) fn model_values(events: &[AgentEvent]) -> Vec<Value> {
    events
        .iter()
        .filter_map(|event| match event {
            AgentEvent::Model(event) => Some(serde_json::to_value(event).unwrap()),
            _ => None,
        })
        .collect()
}
