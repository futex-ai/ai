//! DeepSeek model implementation for the shared AI interface.

#![warn(unreachable_pub)]

mod catalog;
mod deepseek;

pub use catalog::{
    DEEPSEEK_V4_FLASH, DEEPSEEK_V4_FLASH_THINKING_DISABLED, DEEPSEEK_V4_FLASH_THINKING_MAX,
    DEEPSEEK_V4_PRO, DEEPSEEK_V4_PRO_THINKING_DISABLED, DEEPSEEK_V4_PRO_THINKING_MAX, known_models,
};
pub use deepseek::{DeepSeekConfigurationError, DeepSeekConfigurationResult, DeepSeekModel};
