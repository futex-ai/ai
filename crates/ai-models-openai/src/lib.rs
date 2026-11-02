//! OpenAI model implementation for the shared AI interface.

#![warn(unreachable_pub)]

mod catalog;
mod openai;

pub use catalog::{
    CATALOG, CUSTOM_DEFAULTS, GPT_5_4_MINI, GPT_5_4_NANO, GPT_5_5, GPT_5_6_LUNA, GPT_5_6_SOL,
    GPT_5_6_TERRA, GPT_IMAGE_2, PROVIDER_ID, SORA_2, known_models,
};
pub use openai::{
    OpenAiAudioTranscriber, OpenAiImageGenerator, OpenAiLegacyConverter, OpenAiModel,
    OpenAiVideoGenerator,
};
