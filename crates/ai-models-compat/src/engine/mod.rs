//! Shared Chat Completions request, transport, stream, and response mapping.

mod client;
mod dialect;
pub(crate) mod errors;
mod messages;
pub(crate) mod request;
pub(crate) mod request_types;
pub(crate) mod response;
mod stream;
mod tools;

pub(crate) use client::CompatModel;
pub(crate) use dialect::{CompletionContext, Dialect};
