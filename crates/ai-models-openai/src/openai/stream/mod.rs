//! Internal OpenAI Responses SSE consumption.

mod client;
mod types;

pub(super) use client::complete;
