//! OpenAI Decisions judgment adapter.

mod client;
mod error;
mod request;
mod response;
mod usage;

pub use client::OpenAiJudgmentModel;

#[cfg(test)]
#[path = "_tests_/support.rs"]
mod test_support;
