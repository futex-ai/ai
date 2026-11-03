//! Deterministic identity helpers for provider-synthesized tool calls.

mod hasher;
mod provider_context;
mod scope;

pub use scope::{synthetic_tool_call_id, synthetic_tool_call_scope};
