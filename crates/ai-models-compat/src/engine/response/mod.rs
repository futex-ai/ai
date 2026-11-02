//! Chat Completions response decoding, validation, replay, and token buckets.

mod accumulated;
mod base_error;
mod deferred;
mod eager;
mod legacy;
mod mapping;
mod parse;
mod replay;
mod stream_usage;
mod types;
mod usage;
mod xai_usage;

pub(crate) use accumulated::parse_accumulated;
pub(crate) use deferred::parse_deferred;
#[cfg(test)]
pub(crate) use parse::parse_response;

pub(crate) use base_error::stream_base_response_error;
pub(crate) use types::BaseResponse;
