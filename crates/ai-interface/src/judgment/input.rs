//! Judgment input and local conversation validation.

use std::collections::BTreeSet;

use serde::{Deserialize, Deserializer, Serialize};

use crate::{ConversationMessage, ConversationPart, ErrorCore, ErrorKind};

use super::{JudgmentContent, JudgmentError, JudgmentResult};

const VALIDATION_MODEL_ID: &str = "judgment-validation";

/// Content or a conversation that a judgment model evaluates.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(untagged)]
pub enum JudgmentInput {
    /// An ordered list of judgment parts.
    Content(JudgmentContent),
    /// A conversation with optional system instructions.
    Conversation {
        /// System instructions for the evaluation.
        #[serde(
            default,
            deserialize_with = "deserialize_system",
            skip_serializing_if = "system_is_absent"
        )]
        system: Option<String>,
        /// Messages in conversation order.
        messages: Vec<ConversationMessage>,
    },
}

impl JudgmentInput {
    pub(super) fn validate(&self) -> JudgmentResult<()> {
        match self {
            Self::Content(content) if content.is_empty() => Err(JudgmentError::empty_state()),
            Self::Conversation { messages, .. } => validate_conversation(messages),
            Self::Content(_) => Ok(()),
        }
    }
}

fn validate_conversation(messages: &[ConversationMessage]) -> JudgmentResult<()> {
    if messages.is_empty() {
        return Err(JudgmentError::empty_state());
    }
    for message in messages {
        if let Err(error) = message.validate("local", VALIDATION_MODEL_ID) {
            return Err(JudgmentError::new(error.core().clone()));
        }
    }

    let mut tool_call_ids = BTreeSet::new();
    for part in messages.iter().flat_map(|message| &message.parts) {
        match part {
            ConversationPart::ToolCall { call_id, .. } => {
                tool_call_ids.insert(call_id.as_str());
            }
            ConversationPart::ToolResult { call_id, .. }
                if !tool_call_ids.contains(call_id.as_str()) =>
            {
                return Err(JudgmentError::new(ErrorCore::local(
                    ErrorKind::InvalidRequest,
                    VALIDATION_MODEL_ID,
                    format!("tool result has no earlier tool call with id `{call_id}`"),
                )));
            }
            _ => {}
        }
    }
    Ok(())
}

fn system_is_absent(system: &Option<String>) -> bool {
    system.as_deref().is_none_or(|text| text.trim().is_empty())
}

fn deserialize_system<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let system = Option::<String>::deserialize(deserializer)?;
    if system_is_absent(&system) {
        Ok(None)
    } else {
        Ok(system)
    }
}

impl From<JudgmentContent> for JudgmentInput {
    fn from(content: JudgmentContent) -> Self {
        Self::Content(content)
    }
}

impl From<&str> for JudgmentInput {
    fn from(text: &str) -> Self {
        Self::Content(text.into())
    }
}

impl From<String> for JudgmentInput {
    fn from(text: String) -> Self {
        Self::Content(text.into())
    }
}

#[cfg(test)]
#[path = "_tests_/input_tests.rs"]
mod input_tests;
