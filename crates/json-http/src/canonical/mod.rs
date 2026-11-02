//! Typed JSON serialization with the existing sorted object and number encoding.

mod key;
mod object;
mod raw;
mod sequence;
mod serializer;

pub(crate) use serializer::CanonicalSerializer;
