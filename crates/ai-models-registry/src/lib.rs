//! Provider-configured model and modality construction without runtime policy layers.

#![warn(unreachable_pub)]

mod error;
mod modalities;
mod models;
mod providers;
mod registry;

pub use ai_models_core::{ProviderConfig, ProviderTimeouts};
pub use error::{BuildError, BuildResult, Modality};
pub use providers::catalog;
pub use registry::Registry;

#[cfg(test)]
#[path = "_tests_/mod.rs"]
mod tests;
