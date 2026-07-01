//! OpenAI model implementation for the shared AI interface.

#![warn(unreachable_pub)]

mod catalog;
mod openai;

pub use catalog::{
    GPT_5_5, GPT_5_5_MINI, GPT_5_5_NANO, GPT_5_5_THINKING_EXTRA_HIGH, GPT_5_5_THINKING_HIGH,
    GPT_5_5_THINKING_LOW, known_models,
};
pub use openai::{OpenAiAudioTranscriber, OpenAiModel};
