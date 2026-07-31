//! QwenCloud Chat Completions model adapter.

mod client;
mod request;
mod request_types;
mod response;

pub use client::{QwenConfigurationError, QwenConfigurationResult, QwenModel};

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
