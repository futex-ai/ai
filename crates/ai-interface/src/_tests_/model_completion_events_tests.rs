//! Tests for public model completion events and sink compatibility.

use serde_json::json;
use unimock::{MockFn, Unimock, matching};

use crate::{
    ConversationMessage, MockModel, Model, ModelCompletionEvent, ModelCompletionEventSink,
    ModelCompletionEventSinkMock, ModelRequest,
};

#[test]
fn completion_events_have_stable_typed_serialization() {
    let event = ModelCompletionEvent::AssistantTextDelta {
        delta: "hello".to_owned(),
    };

    assert_eq!(
        serde_json::to_value(event).expect("event should serialize"),
        json!({"AssistantTextDelta": {"delta": "hello"}})
    );
}

#[tokio::test]
async fn completion_event_sink_is_unimockable() {
    let sink = Unimock::new(
        ModelCompletionEventSinkMock::emit
            .next_call(matching!(_))
            .answers(&|_, event| {
                assert_eq!(
                    event,
                    ModelCompletionEvent::ReasoningTextDelta {
                        delta: "considering".to_owned(),
                    }
                );
            }),
    );

    sink.emit(ModelCompletionEvent::ReasoningTextDelta {
        delta: "considering".to_owned(),
    })
    .await;
}

#[tokio::test]
async fn default_event_entrypoint_delegates_without_emitting() {
    let model = MockModel::new("mock-dev");
    let sink = Unimock::new(());

    let response = model
        .complete_with_events(&request(), &sink)
        .await
        .expect("default event entrypoint should complete");

    assert_eq!(
        response.assistant_message,
        "Acknowledged: inspect the queue"
    );
}

fn request() -> ModelRequest {
    ModelRequest {
        system_prompt: "Be concise.".to_owned(),
        messages: vec![ConversationMessage::user("- body: inspect the queue")],
        tools: Vec::new(),
        response_schema: None,
        controls: Default::default(),
    }
}
