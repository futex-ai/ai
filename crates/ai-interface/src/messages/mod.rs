//! Conversation message DTOs.

mod conversation;
mod legacy;
mod legacy_conversion;
mod legacy_fallback;
mod legacy_message_converter;
mod part;
mod part_helpers;
mod state;
mod stored;

pub use conversation::{ConversationMessage, ConversationRole};
pub use legacy::{
    LegacyConversationMessage, LegacyPartAttachment, LegacyPartOrder, LegacyPartTarget,
    LegacyProviderConverter, LegacyProviderItem,
};
pub use legacy_conversion::compose_legacy_message;
pub use legacy_message_converter::LegacyMessageConverter;
pub use part::ConversationPart;
pub use state::ProviderState;
pub use stored::{StoredMessageError, read_stored_message};

#[cfg(any(test, doctest, feature = "test-support"))]
pub use legacy::LegacyProviderConverterMock;
