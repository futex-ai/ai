//! xAI model implementation for the shared AI interface.

#![warn(unreachable_pub)]

mod catalog;
mod xai;

pub use catalog::{
    GROK_4_5, GROK_4_5_THINKING_LOW, GROK_4_5_THINKING_MEDIUM, GROK_4_20, GROK_4_20_MINI,
    GROK_4_20_REASONING, GROK_4_20_THINKING_HIGH, known_models,
};
pub use xai::XaiModel;
