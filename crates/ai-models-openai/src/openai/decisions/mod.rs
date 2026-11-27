//! OpenAI Decisions adapter for the shared judgment boundary.

mod answer;
mod client;
mod error;
mod input;
mod request;
mod response;

pub use client::OpenAiJudgmentModel;

#[cfg(test)]
#[path = "_tests_/mod.rs"]
mod tests;
