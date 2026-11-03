//! Google model implementation for the shared AI interface.

#![warn(unreachable_pub)]

mod catalog;
mod google;

pub use catalog::{
    CATALOG, CUSTOM_DEFAULTS, GEMINI_2_5_FLASH, GEMINI_2_5_FLASH_LITE, GEMINI_2_5_PRO,
    GEMINI_3_1_FLASH_IMAGE, GEMINI_3_5_FLASH_LITE, GEMINI_3_6_FLASH, PROVIDER_ID, known_models,
};
pub use google::{
    GoogleImageGenerator, GoogleLegacyProviderConverter, GoogleModel, GoogleVideoGenerator,
};
