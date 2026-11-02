//! Primary-choice text extraction from Chat Completions chunks.

use ai_interface::ModelCompletionEvent;

use super::types::{ChatCompletionsDelta, ChatCompletionsStreamChunk};

impl From<ChatCompletionsDelta> for ModelCompletionEvent {
    fn from(delta: ChatCompletionsDelta) -> Self {
        match delta {
            ChatCompletionsDelta::AssistantText { delta } => Self::AssistantTextDelta { delta },
            ChatCompletionsDelta::ReasoningText { delta } => Self::ReasoningTextDelta { delta },
        }
    }
}

pub(super) fn primary_deltas(chunk: &ChatCompletionsStreamChunk) -> Vec<ChatCompletionsDelta> {
    let mut deltas = Vec::new();
    for choice in &chunk.choices {
        if choice.index != 0 {
            continue;
        }
        push_reasoning(&mut deltas, choice.delta.reasoning_content.as_deref());
        push_assistant(&mut deltas, choice.delta.content.as_deref());
    }
    deltas
}

fn push_reasoning(deltas: &mut Vec<ChatCompletionsDelta>, delta: Option<&str>) {
    if let Some(delta) = delta.filter(|delta| !delta.is_empty()) {
        deltas.push(ChatCompletionsDelta::ReasoningText {
            delta: delta.to_owned(),
        });
    }
}

fn push_assistant(deltas: &mut Vec<ChatCompletionsDelta>, delta: Option<&str>) {
    if let Some(delta) = delta.filter(|delta| !delta.is_empty()) {
        deltas.push(ChatCompletionsDelta::AssistantText {
            delta: delta.to_owned(),
        });
    }
}
