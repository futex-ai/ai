//! Ordered fallback model combinator over multiple `ai-interface` models.

#![warn(unreachable_pub)]

mod model;
mod stream;

pub use model::MultiModel;

#[cfg(test)]
#[path = "_tests_/multi_tests.rs"]
mod multi_tests;

#[cfg(test)]
#[path = "_tests_/multi_event_tests.rs"]
mod multi_event_tests;

#[cfg(test)]
#[path = "_tests_/fixtures.rs"]
mod fixtures;

#[cfg(test)]
#[path = "_tests_/stream_fallback_tests.rs"]
mod stream_fallback_tests;
