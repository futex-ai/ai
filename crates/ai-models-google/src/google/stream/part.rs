//! Accumulated Google candidate content parts.

use serde_json::{Map, Value};

use super::types::{GoogleFunctionCall, GoogleStreamDelta, GoogleStreamPart};

#[derive(Debug)]
pub(super) struct AccumulatedPart {
    text: Option<String>,
    thought: Option<bool>,
    function_call: Option<GoogleFunctionCall>,
    assistant_emitted: bool,
}

impl AccumulatedPart {
    pub(super) fn append(
        parts: &mut Vec<Self>,
        part: GoogleStreamPart,
        merge_with_previous: bool,
    ) -> Option<GoogleStreamDelta> {
        let GoogleStreamPart {
            text,
            thought,
            function_call,
        } = part;
        match (text, function_call) {
            (Some(text), None) => {
                if merge_with_previous
                    && let Some(previous) = parts.last_mut()
                    && previous.function_call.is_none()
                    && previous.thought == thought
                {
                    return previous.append_text(text);
                }
                Self::push_new(parts, Some(text), thought, None)
            }
            (text, Some(function_call)) => {
                Self::push_new(parts, text, thought, Some(function_call))
            }
            (None, None) => None,
        }
    }

    fn push_new(
        parts: &mut Vec<Self>,
        text: Option<String>,
        thought: Option<bool>,
        function_call: Option<GoogleFunctionCall>,
    ) -> Option<GoogleStreamDelta> {
        let delta = text.as_ref().and_then(|text| initial_delta(text, thought));
        let assistant_emitted = matches!(
            delta,
            Some(GoogleStreamDelta::AssistantText {
                starts_part: true,
                ..
            })
        );
        parts.push(Self {
            text,
            thought,
            function_call,
            assistant_emitted,
        });
        delta
    }

    fn append_text(&mut self, delta: String) -> Option<GoogleStreamDelta> {
        let text = self.text.get_or_insert_default();
        text.push_str(&delta);
        if delta.is_empty() {
            return None;
        }
        if self.thought == Some(true) {
            return Some(GoogleStreamDelta::ReasoningText { delta });
        }
        if self.assistant_emitted {
            return Some(GoogleStreamDelta::AssistantText {
                delta,
                starts_part: false,
            });
        }
        if text.trim().is_empty() {
            return None;
        }
        self.assistant_emitted = true;
        Some(GoogleStreamDelta::AssistantText {
            delta: text.clone(),
            starts_part: true,
        })
    }

    pub(super) fn body(&self) -> Value {
        let mut body = Map::new();
        if let Some(text) = &self.text {
            body.insert("text".to_owned(), Value::String(text.clone()));
        }
        if let Some(thought) = self.thought {
            body.insert("thought".to_owned(), Value::Bool(thought));
        }
        if let Some(function_call) = &self.function_call {
            body.insert("functionCall".to_owned(), function_call_body(function_call));
        }
        Value::Object(body)
    }
}

fn initial_delta(text: &str, thought: Option<bool>) -> Option<GoogleStreamDelta> {
    if text.is_empty() {
        return None;
    }
    if thought == Some(true) {
        return Some(GoogleStreamDelta::ReasoningText {
            delta: text.to_owned(),
        });
    }
    if text.trim().is_empty() {
        return None;
    }
    Some(GoogleStreamDelta::AssistantText {
        delta: text.to_owned(),
        starts_part: true,
    })
}

fn function_call_body(function_call: &GoogleFunctionCall) -> Value {
    let mut body = Map::new();
    if let Some(id) = &function_call.id {
        body.insert("id".to_owned(), Value::String(id.clone()));
    }
    body.insert("name".to_owned(), Value::String(function_call.name.clone()));
    if let Some(args) = &function_call.args {
        body.insert("args".to_owned(), args.clone());
    }
    Value::Object(body)
}
