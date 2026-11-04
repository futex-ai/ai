//! Shared Chat Completions request, transport, stream, and response mapping.

mod client;
mod controls;
mod deferred_stream;
pub(crate) mod dialect;
pub(crate) mod errors;
pub(crate) mod lifecycle;
mod message_content;
mod messages;
pub(crate) mod replay_state;
pub(crate) mod request;
mod request_legacy;
mod request_replay;
pub(crate) mod request_types;
pub(crate) mod response;
pub(crate) mod stream;
mod stream_events;
mod stream_tools;
mod thinking;
mod tools;

pub(crate) use client::CompatModel;
pub(crate) use dialect::{CompletionContext, Dialect};

#[cfg(test)]
#[path = "_tests_/spec_controls_tests.rs"]
mod spec_controls_tests;

#[cfg(test)]
#[path = "_tests_/pull_stream_tests.rs"]
mod pull_stream_tests;

#[cfg(test)]
#[path = "_tests_/stream_lifecycle_tests.rs"]
mod stream_lifecycle_tests;

#[cfg(test)]
#[path = "_tests_/delayed_tool_tests.rs"]
mod delayed_tool_tests;

#[cfg(test)]
#[path = "_tests_/finish_state_tests.rs"]
mod finish_state_tests;

#[cfg(test)]
#[path = "_tests_/part_replay_tests.rs"]
mod part_replay_tests;

#[cfg(test)]
#[path = "_tests_/provider_default_tests.rs"]
mod provider_default_tests;
