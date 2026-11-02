//! XAI chat-completions model adapter.

mod client;
mod deferred;
mod request;
mod request_types;
mod response;
mod stream;

pub use client::XaiModel;
