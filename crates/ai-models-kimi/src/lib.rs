//! Kimi model implementation for the shared AI interface.

#![warn(unreachable_pub)]

mod catalog;
mod kimi;

pub use catalog::{KIMI_K3, KIMI_K3_THINKING_HIGH, KIMI_K3_THINKING_LOW, known_models};
pub use kimi::{KimiConfigurationError, KimiConfigurationResult, KimiModel};
