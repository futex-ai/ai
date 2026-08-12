//! XAI chat-completions model adapter.

mod client;
mod deferred;
mod request;
mod request_types;
mod response;

pub use client::XaiModel;

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
