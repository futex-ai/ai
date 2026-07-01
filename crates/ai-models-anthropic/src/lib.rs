//! Anthropic model implementation for the shared AI interface.

#![warn(unreachable_pub)]

mod anthropic;
mod catalog;

pub use anthropic::AnthropicModel;
pub use catalog::{
    CLAUDE_HAIKU_4_5, CLAUDE_OPUS_4_7, CLAUDE_OPUS_4_7_THINKING_MAX, CLAUDE_SONNET_4_6,
    known_models,
};
