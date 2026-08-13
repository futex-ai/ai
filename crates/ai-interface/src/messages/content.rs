//! Typed model-visible conversation content parts.

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
/// Typed model-visible content part.
pub enum ConversationContentPart {
    /// Plain text content.
    Text {
        /// Text body.
        text: String,
    },
    /// Image bytes encoded as base64.
    Image {
        /// Image MIME content type.
        mime_type: String,
        /// Base64-encoded image bytes.
        data_base64: String,
    },
    /// Video bytes encoded as base64.
    Video {
        /// Video MIME content type.
        mime_type: String,
        /// Base64-encoded video bytes.
        data_base64: String,
    },
}
