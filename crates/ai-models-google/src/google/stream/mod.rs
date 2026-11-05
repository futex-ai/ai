//! Internal Google generate-content SSE accumulation.

mod accumulator;
mod client;
mod part;
mod replay;
pub(super) mod types;

pub(super) use client::open;
