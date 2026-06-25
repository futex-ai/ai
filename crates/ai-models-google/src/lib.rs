//! Google model implementation for the shared AI interface.

#![warn(unreachable_pub)]

mod catalog;
mod google;

pub use catalog::{
    GEMINI_2_5_FLASH, GEMINI_2_5_FLASH_LITE, GEMINI_2_5_PRO, GEMINI_2_5_PRO_THINKING_HIGH,
    GEMINI_2_5_PRO_THINKING_MAX, known_models,
};
pub use google::GoogleModel;
