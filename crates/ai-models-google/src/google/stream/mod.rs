//! Internal Google generate-content SSE accumulation.

mod accumulator;
mod client;
mod part;
mod types;

pub(super) use client::complete;
