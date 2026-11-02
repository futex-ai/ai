//! An injected composition of provider-owned stored message converters.

use std::sync::Arc;

use serde_json::Value;

use super::{
    conversation::ConversationMessage,
    legacy::{LegacyConversationMessage, LegacyProviderConverter},
    legacy_conversion::compose_legacy_message,
    stored::{StoredMessageError, read_stored_message},
};

#[derive(Clone, Default)]
/// Composes the installed provider converters without naming any provider.
pub struct LegacyMessageConverter {
    converters: Vec<Arc<dyn LegacyProviderConverter>>,
}

impl LegacyMessageConverter {
    /// Builds a composition from the provider converters installed by the host.
    pub fn new(converters: Vec<Arc<dyn LegacyProviderConverter>>) -> Self {
        Self { converters }
    }

    /// Converts a decoded version one message to ordered portable parts.
    pub fn convert(&self, message: &LegacyConversationMessage) -> ConversationMessage {
        let converters = self.converters.iter().map(Arc::as_ref).collect::<Vec<_>>();
        compose_legacy_message(message, &converters)
    }

    /// Reads a stored version one or version two message.
    pub fn read(&self, value: Value) -> Result<ConversationMessage, StoredMessageError> {
        let converters = self.converters.iter().map(Arc::as_ref).collect::<Vec<_>>();
        read_stored_message(value, &converters)
    }
}
