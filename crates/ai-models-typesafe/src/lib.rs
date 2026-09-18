//! TypeSafe judgment model implementation for the shared AI interface.

#![warn(unreachable_pub)]

mod catalog;
mod typesafe;

pub use catalog::{JEV_1_13_0, JEV_LATEST, known_models};
pub use typesafe::TypeSafeJudgmentModel;
