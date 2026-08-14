//! Google model implementation for the shared AI interface.

#![warn(unreachable_pub)]

mod catalog;
mod google;

pub use catalog::{
    GEMINI_2_5_FLASH, GEMINI_2_5_FLASH_LITE, GEMINI_2_5_PRO, GEMINI_2_5_PRO_THINKING_HIGH,
    GEMINI_2_5_PRO_THINKING_MAX, GEMINI_3_1_FLASH_IMAGE, GEMINI_3_5_FLASH_LITE, GEMINI_3_6_FLASH,
    GEMINI_3_6_FLASH_THINKING_HIGH, VEO_3_1_GENERATE_PREVIEW, known_models,
};
pub use google::{GoogleImageGenerator, GoogleModel, GoogleVideoGenerator};
