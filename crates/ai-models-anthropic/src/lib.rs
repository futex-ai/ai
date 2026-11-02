//! Anthropic model implementation for the shared AI interface.

#![warn(unreachable_pub)]

mod anthropic;
mod catalog;
mod legacy;

pub use anthropic::AnthropicModel;
pub use catalog::{
    CATALOG, CLAUDE_FABLE_5, CLAUDE_HAIKU_4_5, CLAUDE_OPUS_4_7, CLAUDE_OPUS_5, CLAUDE_SONNET_4_6,
    CLAUDE_SONNET_5, CUSTOM_DEFAULTS, PROVIDER_ID, known_models,
};
pub use legacy::AnthropicLegacyConverter;
