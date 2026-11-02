//! Qwen Chat Completions dialect and public construction API.

mod catalog;
mod client;
mod dialect;
pub(crate) mod state;

#[cfg(test)]
#[path = "_tests_/request.rs"]
mod request;
#[cfg(test)]
#[path = "_tests_/response.rs"]
mod response;

pub use catalog::{
    CATALOG, CUSTOM_DEFAULTS, PROVIDER_ID, QWEN_3_7_FLASH, QWEN_3_7_MAX, QWEN_3_7_PLUS,
    known_models,
};
pub use client::QwenModel;
pub use state::LegacyConverter;

#[cfg(test)]
#[path = "_tests_/support.rs"]
mod test_support;

#[cfg(test)]
#[path = "_tests_/construction_tests.rs"]
mod construction_tests;

#[cfg(test)]
#[path = "_tests_/request_tests.rs"]
mod request_tests;

#[cfg(test)]
#[path = "_tests_/response_tests.rs"]
mod response_tests;

#[cfg(test)]
#[path = "_tests_/usage_structured_tests.rs"]
mod usage_structured_tests;

#[cfg(test)]
#[path = "_tests_/error_tests.rs"]
mod error_tests;

#[cfg(test)]
#[path = "_tests_/controls_tests.rs"]
mod controls_tests;

#[cfg(test)]
#[path = "_tests_/streaming_tests.rs"]
mod streaming_tests;

#[cfg(test)]
#[path = "_tests_/stream_error_tests.rs"]
mod stream_error_tests;

#[cfg(test)]
#[path = "_tests_/event_tests.rs"]
mod event_tests;

#[cfg(test)]
#[path = "_tests_/catalog_tests.rs"]
mod catalog_tests;
