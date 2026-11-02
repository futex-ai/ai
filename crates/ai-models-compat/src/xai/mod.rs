//! xAI Chat Completions dialect and public construction API.

mod catalog;
mod client;
mod deferred;
mod dialect;

#[cfg(test)]
#[path = "_tests_/response.rs"]
mod response;

pub use catalog::{
    GROK_4_5, GROK_4_5_THINKING_LOW, GROK_4_5_THINKING_MEDIUM, GROK_4_20, GROK_4_20_REASONING,
    known_models,
};
pub use client::XaiModel;

#[cfg(test)]
#[path = "_tests_/support.rs"]
mod test_support;

#[cfg(test)]
#[path = "_tests_/xai_continuation_tests.rs"]
mod xai_continuation_tests;
#[cfg(test)]
#[path = "_tests_/xai_tests.rs"]
mod xai_tests;

#[cfg(test)]
#[path = "_tests_/xai_structured_finish_tests.rs"]
mod xai_structured_finish_tests;

#[cfg(test)]
#[path = "_tests_/xai_tool_finish_tests.rs"]
mod xai_tool_finish_tests;

#[cfg(test)]
#[path = "_tests_/xai_multimodal_tests.rs"]
mod xai_multimodal_tests;

#[cfg(test)]
#[path = "_tests_/xai_operation_id_tests.rs"]
mod xai_operation_id_tests;

#[cfg(test)]
#[path = "_tests_/xai_thinking_tests.rs"]
mod xai_thinking_tests;

#[cfg(test)]
#[path = "_tests_/xai_usage_tests.rs"]
mod xai_usage_tests;

#[cfg(test)]
#[path = "_tests_/xai_deferred_tests.rs"]
mod xai_deferred_tests;

#[cfg(test)]
#[path = "_tests_/xai_controls_tests.rs"]
mod xai_controls_tests;

#[cfg(test)]
#[path = "_tests_/xai_streaming_tests.rs"]
mod xai_streaming_tests;

#[cfg(test)]
#[path = "_tests_/xai_stream_error_tests.rs"]
mod xai_stream_error_tests;

#[cfg(test)]
#[path = "_tests_/xai_event_tests.rs"]
mod xai_event_tests;

#[cfg(test)]
#[path = "_tests_/catalog_tests.rs"]
mod catalog_tests;
