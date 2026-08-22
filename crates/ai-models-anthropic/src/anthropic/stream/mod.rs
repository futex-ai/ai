//! Internal Anthropic Messages SSE consumption.

mod accumulator;
mod block;
mod client;
mod types;

pub(super) use client::complete;
