//! Internal Anthropic Messages SSE consumption.

mod accumulator;
mod block;
mod client;
pub(super) mod types;

pub(super) use client::open;
