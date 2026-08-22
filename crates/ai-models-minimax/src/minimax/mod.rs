//! MiniMax chat-completions model adapter.

mod client;
mod request;
mod request_types;
mod response;
mod stream;
mod stream_normalizer;

pub use client::MiniMaxModel;

#[cfg(test)]
#[path = "_tests_/error_tests.rs"]
mod error_tests;
#[cfg(test)]
#[path = "_tests_/finish_tests.rs"]
mod finish_tests;
#[cfg(test)]
#[path = "_tests_/multimodal_tests.rs"]
mod multimodal_tests;
#[cfg(test)]
#[path = "_tests_/provider_error_tests.rs"]
mod provider_error_tests;
#[cfg(test)]
#[path = "_tests_/replay_tests.rs"]
mod replay_tests;
#[cfg(test)]
#[path = "_tests_/response_shape_tests.rs"]
mod response_shape_tests;
#[cfg(test)]
#[path = "_tests_/structured_output_tests.rs"]
mod structured_output_tests;
#[cfg(test)]
#[path = "_tests_/support.rs"]
mod support;
#[cfg(test)]
#[path = "_tests_/text_tests.rs"]
mod text_tests;
#[cfg(test)]
#[path = "_tests_/thinking_tests.rs"]
mod thinking_tests;
#[cfg(test)]
#[path = "_tests_/tool_tests.rs"]
mod tool_tests;
#[cfg(test)]
#[path = "_tests_/usage_tests.rs"]
mod usage_tests;

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
