//! Event observation helpers that use the public stream and response fold.

use ai_interface::{Model, ModelRequest, ModelResponse, ModelResponseFold, ModelResult};
use serde_json::{Value, json};

pub(crate) async fn observe(
    model: &dyn Model,
    request: &ModelRequest,
    content: &mut Vec<Value>,
) -> ModelResult<ModelResponse> {
    let mut stream = model.stream(request).await?;
    let mut fold = ModelResponseFold::new();
    while let Some(event) = stream.next().await? {
        content.push(serde_json::to_value(&event).expect("model event serializes"));
        fold.push(event)?;
    }
    fold.finish()
}

pub(crate) fn assistant_delta(delta: &str) -> Value {
    json!({"TextDelta": {"delta": delta}})
}

pub(crate) fn reasoning_delta(delta: &str) -> Value {
    json!({"ReasoningDelta": {"delta": delta}})
}

pub(crate) fn assistant_text(events: &[Value]) -> String {
    events
        .iter()
        .filter_map(|event| event["TextDelta"]["delta"].as_str())
        .collect()
}

pub(crate) fn content_events(events: &[Value]) -> Vec<Value> {
    events
        .iter()
        .filter_map(|event| {
            if let Some(delta) = event["TextDelta"]["delta"]
                .as_str()
                .filter(|delta| !delta.is_empty())
            {
                Some(assistant_delta(delta))
            } else {
                event["ReasoningDelta"]["delta"]
                    .as_str()
                    .filter(|delta| !delta.is_empty())
                    .map(reasoning_delta)
            }
        })
        .collect()
}

/// Projects events onto the fields present before parts carried replay state.
pub(crate) fn portable_events(events: &[Value]) -> Vec<Value> {
    events
        .iter()
        .filter_map(|event| {
            let mut event = event.clone();
            let fields = event
                .as_object_mut()?
                .values_mut()
                .next()?
                .as_object_mut()?;
            if fields.contains_key("provider_state") && fields.get("delta") == Some(&json!("")) {
                return None;
            }
            fields.remove("provider_state");
            Some(event)
        })
        .collect()
}
