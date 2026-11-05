//! Versioned stored message serialization and explicit conversion entry point.

use internal_error::InternalError;
use serde::{Deserialize, Deserializer, Serialize, Serializer, de::Error};
use serde_json::Value;
use thiserror::Error;

use super::{
    conversation::{ConversationMessage, ConversationRole},
    legacy::{LegacyConversationMessage, LegacyProviderConverter},
    legacy_conversion::compose_legacy_message,
    part::ConversationPart,
};

#[derive(Debug, Error, internal_error::ErrorContract)]
/// Typed failures from the stored conversation reader.
pub enum StoredMessageError {
    /// The message uses a stored format this library cannot read.
    #[error("[ai-interface/messages] unsupported stored format version {version}")]
    UnsupportedFormatVersion {
        /// Unsupported version recorded in the message.
        version: u64,
    },
    /// The stored message could not be decoded.
    #[error("[ai-interface/messages] {0}")]
    Internal(#[from] InternalError),
}

#[derive(Deserialize, Serialize)]
struct StoredMessage {
    format_version: u64,
    role: ConversationRole,
    parts: Vec<ConversationPart>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    name: Option<String>,
}

impl Serialize for ConversationMessage {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        StoredMessage {
            format_version: 2,
            role: self.role,
            parts: self.parts.clone(),
            name: self.name.clone(),
        }
        .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for ConversationMessage {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = Value::deserialize(deserializer)?;
        match read_stored_message(value, &[]) {
            Ok(message) => Ok(message),
            Err(error) => Err(D::Error::custom(error)),
        }
    }
}

/// Reads versions one and two, using installed converters for old provider items.
pub fn read_stored_message(
    value: Value,
    converters: &[&dyn LegacyProviderConverter],
) -> Result<ConversationMessage, StoredMessageError> {
    let version = match value.get("format_version") {
        Some(version) => match serde_json::from_value::<u64>(version.clone()) {
            Ok(version) => version,
            Err(error) => return Err(StoredMessageError::internal(error)),
        },
        None => 1,
    };
    match version {
        1 => {
            let legacy = match serde_json::from_value::<LegacyConversationMessage>(value) {
                Ok(legacy) => legacy,
                Err(error) => return Err(StoredMessageError::internal(error)),
            };
            Ok(compose_legacy_message(&legacy, converters))
        }
        2 => {
            let stored = match serde_json::from_value::<StoredMessage>(value) {
                Ok(stored) => stored,
                Err(error) => return Err(StoredMessageError::internal(error)),
            };
            Ok(ConversationMessage {
                role: stored.role,
                parts: stored.parts,
                name: stored.name,
            })
        }
        version => Err(StoredMessageError::UnsupportedFormatVersion { version }),
    }
}
