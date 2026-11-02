//! MiniMax chat-completions model adapter.

mod client;
mod request;
mod request_types;
mod response;
mod stream;
mod stream_normalizer;

pub use client::MiniMaxModel;
