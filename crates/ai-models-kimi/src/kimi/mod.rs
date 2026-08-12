//! Kimi Chat Completions model adapter.

mod client;
mod request;
mod request_types;
mod response;

pub use client::{KimiConfigurationError, KimiConfigurationResult, KimiModel};

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
#[path = "_tests_/structured_output_tests.rs"]
mod structured_output_tests;

#[cfg(test)]
#[path = "_tests_/response_tests.rs"]
mod response_tests;

#[cfg(test)]
#[path = "_tests_/usage_tests.rs"]
mod usage_tests;

#[cfg(test)]
#[path = "_tests_/tool_call_tests.rs"]
mod tool_call_tests;

#[cfg(test)]
#[path = "_tests_/continuation_tests.rs"]
mod continuation_tests;

#[cfg(test)]
#[path = "_tests_/client_tests.rs"]
mod client_tests;

#[cfg(test)]
#[path = "_tests_/controls_tests.rs"]
mod controls_tests;
