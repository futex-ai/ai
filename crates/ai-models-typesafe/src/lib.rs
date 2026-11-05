//! TypeSafe judgment model implementation for the shared AI interface.

#![warn(unreachable_pub)]

mod catalog;
mod typesafe;

pub use catalog::{CATALOG, CUSTOM_DEFAULTS, JEV_1_13_0, JEV_LATEST, PROVIDER_ID, known_models};
pub use typesafe::TypeSafeJudgmentModel;
