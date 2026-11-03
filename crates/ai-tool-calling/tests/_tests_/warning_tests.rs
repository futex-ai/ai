//! Observer failures produce credential-free tracing warnings.

use std::{
    fmt,
    sync::{Arc, Mutex},
};

use ai_interface::{
    ConversationMessage, FinishReason, LoggerError, LoggerMock, test_support::model_response_stream,
};
use ai_tool_calling::{Agent, InMemoryToolOutputStore, ToolCallingRuntime, ToolOutputPolicy};
use tracing::{
    Event, Level, Metadata, Subscriber,
    field::{Field, Visit},
    instrument::WithSubscriber,
    span::{Attributes, Id, Record},
};
use unimock::{MockFn, Unimock, matching};

use super::support::{collect, model, request, response};

#[derive(Clone, Default)]
struct Warnings {
    events: Arc<Mutex<Vec<(Level, String)>>>,
}

impl Subscriber for Warnings {
    fn enabled(&self, metadata: &Metadata<'_>) -> bool {
        metadata.level() == &Level::WARN
    }
    fn new_span(&self, _: &Attributes<'_>) -> Id {
        Id::from_u64(1)
    }
    fn record(&self, _: &Id, _: &Record<'_>) {}
    fn record_follows_from(&self, _: &Id, _: &Id) {}
    fn enter(&self, _: &Id) {}
    fn exit(&self, _: &Id) {}

    fn event(&self, event: &Event<'_>) {
        let mut text = EventText(String::new());
        event.record(&mut text);
        self.events
            .lock()
            .unwrap()
            .push((*event.metadata().level(), text.0));
    }
}

struct EventText(String);

impl Visit for EventText {
    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        self.0.push_str(&format!("{}={value:?}\n", field.name()));
    }
}

#[derive(Debug, thiserror::Error)]
#[error("private-observer-cause")]
struct PrivateObserverFailure;

#[tokio::test]
async fn observer_failures_emit_warnings_without_the_private_cause() {
    let logger = Unimock::new((
        LoggerMock::log_model_call
            .some_call(matching!(_))
            .answers(&|_, _| Err(LoggerError::write(PrivateObserverFailure))),
        LoggerMock::log_turn_outcome
            .some_call(matching!(_))
            .answers(&|_, _| Err(LoggerError::write(PrivateObserverFailure))),
    ));
    let (model, _) = model(vec![Ok(model_response_stream(response(
        "done",
        vec![],
        FinishReason::Stop,
    )))]);
    let agent = ToolCallingRuntime::new(
        model,
        Arc::new(logger),
        vec![],
        Arc::new(InMemoryToolOutputStore::new()),
        ToolOutputPolicy::default(),
    )
    .unwrap();
    let warnings = Warnings::default();
    let mut conversation = vec![ConversationMessage::user("start")];
    let result = async { collect(agent.run(&mut conversation, request(None)).as_mut()).await }
        .with_subscriber(warnings.clone())
        .await;
    assert!(
        result.is_ok(),
        "observer errors must not end a turn: {result:?}"
    );
    let events = warnings.events.lock().unwrap();
    assert_eq!(events.len(), 2);
    for (level, message) in events.iter() {
        assert_eq!(level, &Level::WARN);
        assert!(message.contains("agent observer callback failed"));
        assert!(!message.contains("private-observer-cause"));
    }
}
