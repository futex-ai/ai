//! DeepSeek Chat Completions model adapter.

mod client;
mod request;
mod request_types;
mod response;
mod stream;

pub use client::{DeepSeekConfigurationError, DeepSeekConfigurationResult, DeepSeekModel};
