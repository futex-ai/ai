//! Version one conversion reports each discarded input through tracing.

use std::{
    fmt::Debug,
    sync::{Arc, Mutex},
};

use serde_json::json;
use tracing::{
    Event, Metadata, Subscriber,
    field::{Field, Visit},
    span::{Attributes, Id, Record},
};
use unimock::{MockFn, Unimock, matching};

use crate::{ConversationPart, LegacyPartOrder, read_stored_message};

use super::conversion_tests::converter;

#[unimock::unimock(api = WarningSinkMock)]
trait WarningSink: Send + Sync {
    fn observe(&self, message: &str);
}

#[test]
fn discarded_legacy_parts_and_items_emit_warnings() {
    let observed = Arc::new(Mutex::new(Vec::new()));
    let captured = observed.clone();
    let sink = Arc::new(Unimock::new(
        WarningSinkMock::observe
            .each_call(matching!(_))
            .answers_arc(Arc::new(move |_, message: &str| {
                captured.lock().unwrap().push(message.to_owned());
            })),
    ));
    let sample = converter("sample", LegacyPartOrder::ReasoningFirst);
    let message =
        tracing::subscriber::with_default(WarningSubscriber { sink: sink.clone() }, || {
            read_stored_message(
                json!({
                    "role":"assistant", "content":"kept",
                    "content_parts":[{"type":"text", "text":"dropped"}],
                    "provider_context":[{"type":"sample_function_call", "call_id":"missing"}]
                }),
                &[&sample],
            )
            .unwrap()
        });
    assert_eq!(message.parts, vec![ConversationPart::text("kept")]);
    let mut warnings = observed.lock().unwrap().clone();
    warnings.sort();
    assert_eq!(
        warnings,
        [
            "dropping legacy content_parts from an assistant or tool message",
            "dropping legacy provider item without a matching part",
        ]
    );
}

struct WarningSubscriber {
    sink: Arc<dyn WarningSink>,
}

impl Subscriber for WarningSubscriber {
    fn enabled(&self, metadata: &Metadata<'_>) -> bool {
        *metadata.level() == tracing::Level::WARN
    }

    fn new_span(&self, _span: &Attributes<'_>) -> Id {
        Id::from_u64(1)
    }

    fn record(&self, _span: &Id, _values: &Record<'_>) {}

    fn record_follows_from(&self, _span: &Id, _follows: &Id) {}

    fn event(&self, event: &Event<'_>) {
        let mut visitor = WarningMessage::default();
        event.record(&mut visitor);
        self.sink.observe(&visitor.message);
    }

    fn enter(&self, _span: &Id) {}

    fn exit(&self, _span: &Id) {}
}

#[derive(Default)]
struct WarningMessage {
    message: String,
}

impl Visit for WarningMessage {
    fn record_debug(&mut self, field: &Field, value: &dyn Debug) {
        if field.name() == "message" {
            self.message = format!("{value:?}");
        }
    }
}
