//! Shared fold from ordered model events into the current response contract.

mod fold;
mod order_error;
mod part;

pub use fold::ModelResponseFold;
