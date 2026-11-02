//! Chat Completions response decoding, validation, replay, and token buckets.

mod base_error;
mod eager;
mod legacy;
mod mapping;
mod parse;
mod replay;
mod types;
mod usage;
mod xai_usage;

pub(crate) use parse::parse_response;

pub(crate) use base_error::stream_base_response_error;
pub(crate) use types::BaseResponse;
