//! Accumulated Google candidate content parts.

use serde_json::{Map, Value};

use super::types::{GoogleFunctionCall, GoogleStreamPart};

#[derive(Debug)]
pub(super) struct AccumulatedPart {
    text: Option<String>,
    thought: Option<bool>,
    function_call: Option<GoogleFunctionCall>,
}

impl AccumulatedPart {
    pub(super) fn append(parts: &mut Vec<Self>, part: GoogleStreamPart, merge_with_previous: bool) {
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
                    && let Some(previous_text) = previous.text.as_mut()
                {
                    previous_text.push_str(&text);
                    return;
                }
                parts.push(Self {
                    text: Some(text),
                    thought,
                    function_call: None,
                });
            }
            (text, Some(function_call)) => parts.push(Self {
                text,
                thought,
                function_call: Some(function_call),
            }),
            (None, None) => {}
        }
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
