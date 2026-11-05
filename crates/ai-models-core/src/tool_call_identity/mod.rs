//! Deterministic identity helpers for provider-synthesized tool calls.

mod hasher;
mod message;
mod provider_state;
mod scope;

pub use scope::{synthetic_tool_call_id, synthetic_tool_call_scope};
