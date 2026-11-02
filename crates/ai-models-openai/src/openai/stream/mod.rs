//! Internal OpenAI Responses SSE consumption.

mod client;
mod state;
mod text;
mod tools;
mod types;

pub(super) use client::open;
