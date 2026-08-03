//! QwenCloud model implementation for the shared AI interface.

#![warn(unreachable_pub)]

mod catalog;
mod qwen;

pub use catalog::{
    QWEN_3_7_FLASH, QWEN_3_7_FLASH_THINKING_DISABLED, QWEN_3_7_MAX, QWEN_3_7_MAX_THINKING_DISABLED,
    QWEN_3_7_PLUS, QWEN_3_7_PLUS_THINKING_DISABLED, known_models,
};
pub use qwen::{QwenConfigurationError, QwenConfigurationResult, QwenModel};
