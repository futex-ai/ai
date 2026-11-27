//! Adapter support for judgment input forms and parts.

use crate::ConversationPart;

use super::{
    JudgmentContent, JudgmentError, JudgmentInput, JudgmentPart, JudgmentRequest, JudgmentResult,
};

/// One form of material evaluated by a judgment model.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum JudgmentInputForm {
    /// Judgment content parts.
    Content,
    /// A conversation.
    Conversation,
}

/// A kind of part in judgment content.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum JudgmentPartKind {
    /// Plain text.
    Text,
    /// A JSON object or array.
    Json,
    /// An inline image.
    Image,
}

/// A kind of part in a retained conversation message.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum JudgmentConversationPartKind {
    /// Plain text.
    Text,
    /// An inline image.
    Image,
    /// An inline video.
    Video,
    /// A tool call.
    ToolCall,
    /// A tool result.
    ToolResult,
    /// Model reasoning.
    Reasoning,
}

/// A request feature named by a judgment support error.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum JudgmentInputFeature {
    /// The form of the request input.
    InputForm(JudgmentInputForm),
    /// A part in the content input or question instructions.
    ContentPart(JudgmentPartKind),
    /// A part in a conversation message.
    ConversationPart(JudgmentConversationPartKind),
}

/// Input forms and part kinds accepted by a judgment adapter.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct JudgmentSupport {
    /// Parts permitted in judgment content.
    pub content_parts: &'static [JudgmentPartKind],
    /// Forms permitted in the input.
    pub input_forms: &'static [JudgmentInputForm],
    /// Parts permitted in conversation messages.
    pub conversation_parts: &'static [JudgmentConversationPartKind],
}

impl JudgmentSupport {
    /// Support for every input form and part kind.
    pub const ALL: Self = Self {
        content_parts: &[
            JudgmentPartKind::Text,
            JudgmentPartKind::Json,
            JudgmentPartKind::Image,
        ],
        input_forms: &[JudgmentInputForm::Content, JudgmentInputForm::Conversation],
        conversation_parts: &[
            JudgmentConversationPartKind::Text,
            JudgmentConversationPartKind::Image,
            JudgmentConversationPartKind::Video,
            JudgmentConversationPartKind::ToolCall,
            JudgmentConversationPartKind::ToolResult,
            JudgmentConversationPartKind::Reasoning,
        ],
    };

    /// Checks a request against this adapter's support after local validation.
    pub fn check(
        &self,
        request: &JudgmentRequest,
        provider: &str,
        model_id: &str,
    ) -> JudgmentResult<()> {
        let unsupported = self.unsupported_input(&request.input).or_else(|| {
            request
                .questions
                .values()
                .find_map(|question| self.unsupported_content(question.instructions()))
        });
        if let Some(feature) = unsupported {
            return Err(JudgmentError::unsupported_input(
                provider, model_id, feature,
            ));
        }
        Ok(())
    }

    fn unsupported_input(&self, input: &JudgmentInput) -> Option<JudgmentInputFeature> {
        let form = match input {
            JudgmentInput::Content(_) => JudgmentInputForm::Content,
            JudgmentInput::Conversation { .. } => JudgmentInputForm::Conversation,
        };
        if !self.input_forms.contains(&form) {
            return Some(JudgmentInputFeature::InputForm(form));
        }
        match input {
            JudgmentInput::Content(content) => self.unsupported_content(content),
            JudgmentInput::Conversation { messages, .. } => messages
                .iter()
                .flat_map(|message| &message.parts)
                .map(conversation_part_kind)
                .find(|kind| !self.conversation_parts.contains(kind))
                .map(JudgmentInputFeature::ConversationPart),
        }
    }

    fn unsupported_content(&self, content: &JudgmentContent) -> Option<JudgmentInputFeature> {
        content
            .parts
            .iter()
            .map(content_part_kind)
            .find(|kind| !self.content_parts.contains(kind))
            .map(JudgmentInputFeature::ContentPart)
    }
}

fn content_part_kind(part: &JudgmentPart) -> JudgmentPartKind {
    match part {
        JudgmentPart::Text { .. } => JudgmentPartKind::Text,
        JudgmentPart::Json { .. } => JudgmentPartKind::Json,
        JudgmentPart::Image { .. } => JudgmentPartKind::Image,
    }
}

fn conversation_part_kind(part: &ConversationPart) -> JudgmentConversationPartKind {
    match part {
        ConversationPart::Text { .. } => JudgmentConversationPartKind::Text,
        ConversationPart::Image { .. } => JudgmentConversationPartKind::Image,
        ConversationPart::Video { .. } => JudgmentConversationPartKind::Video,
        ConversationPart::ToolCall { .. } => JudgmentConversationPartKind::ToolCall,
        ConversationPart::ToolResult { .. } => JudgmentConversationPartKind::ToolResult,
        ConversationPart::Reasoning { .. } => JudgmentConversationPartKind::Reasoning,
    }
}

#[cfg(test)]
#[path = "_tests_/support_tests.rs"]
mod support_tests;
