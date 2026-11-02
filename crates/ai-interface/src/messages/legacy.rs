//! Provider-neutral input types for the version one stored message format.

use serde::Deserialize;
use serde_json::{Map, Value};

use crate::{ProviderId, ToolCall};

use super::{conversation::ConversationRole, part::ConversationPart, state::ProviderState};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
/// A version one provider item with its original tag and fields.
pub struct LegacyProviderItem {
    /// Original item tag, interpreted by the provider converter.
    #[serde(rename = "type")]
    pub kind: String,
    /// All original item fields without the tag.
    #[serde(flatten)]
    pub fields: Map<String, Value>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
/// A version one message accepted only as conversion input.
pub struct LegacyConversationMessage {
    /// Original actor role.
    pub role: ConversationRole,
    /// Original plain text.
    #[serde(default)]
    pub content: String,
    /// Original user media parts, when present.
    #[serde(default)]
    pub content_parts: Vec<ConversationPart>,
    /// Original participant or tool name.
    #[serde(default)]
    pub name: Option<String>,
    /// Original tool-result call identifier.
    #[serde(default)]
    pub tool_call_id: Option<String>,
    /// Original normalized tool calls.
    #[serde(default)]
    pub tool_calls: Vec<ToolCall>,
    /// Original provider items in stored order.
    #[serde(default)]
    pub provider_context: Vec<LegacyProviderItem>,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
/// Order rule applied by the provider of the first legacy item.
pub enum LegacyPartOrder {
    /// Reasoning first, followed by text and portable tool calls.
    #[default]
    ReasoningFirst,
    /// Parts in item order, then unmatched text and portable tool calls.
    Context,
}

#[derive(Clone, Debug, Eq, PartialEq)]
/// Portable part that receives one converted replay state.
pub enum LegacyPartTarget {
    /// The message's first portable text part.
    Text,
    /// The portable call with the specified identifier.
    ToolCall {
        /// Identifier of the call that receives the state.
        call_id: String,
    },
    /// A new reasoning part at this item's position.
    Reasoning {
        /// Provider-exposed reasoning, when available.
        text: String,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
/// One provider-owned conversion attachment.
pub struct LegacyPartAttachment {
    /// Portable destination or a new reasoning part.
    pub target: LegacyPartTarget,
    /// State decoded and written by the owning provider.
    pub provider_state: ProviderState,
}

#[cfg_attr(any(test, doctest, feature = "test-support"), unimock::unimock(api = LegacyProviderConverterMock))]
/// One provider's conversion of its old replay items.
pub trait LegacyProviderConverter: Send + Sync {
    /// Returns the identity used by converted provider state.
    fn provider(&self) -> ProviderId;

    /// Returns the message order used by this provider.
    fn order(&self) -> LegacyPartOrder {
        LegacyPartOrder::ReasoningFirst
    }

    /// Converts an item, or returns `None` when this provider does not own it.
    /// An empty vector explicitly ignores an item such as duplicate metadata.
    fn convert_item(
        &self,
        item: &LegacyProviderItem,
        message: &LegacyConversationMessage,
        item_index: usize,
    ) -> Option<Vec<LegacyPartAttachment>>;

    /// Rebuilds a complete primary message when its original blocks need exact order.
    fn convert_message(
        &self,
        _message: &LegacyConversationMessage,
    ) -> Option<Vec<ConversationPart>> {
        None
    }
}
