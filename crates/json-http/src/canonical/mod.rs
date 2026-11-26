//! Typed JSON serialization with sorted keys and opt-in ordered objects.

mod key;
mod object;
mod ordered_object;
mod raw;
mod sequence;
mod serializer;

pub use ordered_object::OrderedObject;
pub(crate) use serializer::CanonicalSerializer;
