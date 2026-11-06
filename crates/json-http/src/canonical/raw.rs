//! Normalizes embedded raw JSON through the same sorted byte serializer.

use serde::Serialize;
use serde_json::{Deserializer, Result};

use super::serializer::CanonicalSerializer;

pub(super) const RAW_VALUE_TOKEN: &str = "$serde_json::private::RawValue";

pub(super) fn serialize<T: ?Sized + Serialize>(value: &T) -> Result<Vec<u8>> {
    let encoded_string = value.serialize(CanonicalSerializer)?;
    let raw: String = serde_json::from_slice(&encoded_string)?;
    let mut deserializer = Deserializer::from_str(&raw);
    let bytes = serde_transcode::transcode(&mut deserializer, CanonicalSerializer)?;
    deserializer.end()?;
    Ok(bytes)
}
