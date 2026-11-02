//! Internal OpenAI Responses SSE consumption.

mod client;
mod replay;
mod state;
mod text;
mod tools;
mod types;

pub(super) use client::open;

#[cfg(test)]
#[path = "_tests_/state_tests.rs"]
mod state_tests;
