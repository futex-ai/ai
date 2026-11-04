//! Shared Chat Completions request, transport, stream, and response mapping.

mod client;
mod controls;
pub(crate) mod dialect;
pub(crate) mod errors;
pub(crate) mod lifecycle;
mod message_content;
mod messages;
pub(crate) mod request;
mod request_legacy;
mod request_replay;
pub(crate) mod request_types;
pub(crate) mod response;
pub(crate) mod stream;
mod thinking;
mod tools;

pub(crate) use client::CompatModel;
pub(crate) use dialect::{CompletionContext, Dialect};

#[cfg(test)]
#[path = "_tests_/spec_controls_tests.rs"]
mod spec_controls_tests;

#[cfg(test)]
#[path = "_tests_/provider_default_tests.rs"]
mod provider_default_tests;
