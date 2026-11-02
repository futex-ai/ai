//! Optional response context distinguishes no response from an empty header map.

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::JsonHttpHeaders;

use super::headers;

pub(super) fn serialize<S>(
    value: &Option<JsonHttpHeaders>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    value.as_ref().map(BorrowedHeaders).serialize(serializer)
}

pub(super) fn deserialize<'de, D>(deserializer: D) -> Result<Option<JsonHttpHeaders>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Option::<OwnedHeaders>::deserialize(deserializer)?;
    Ok(value.map(|value| value.0))
}

struct BorrowedHeaders<'a>(&'a JsonHttpHeaders);

impl Serialize for BorrowedHeaders<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        headers::serialize(self.0, serializer)
    }
}

struct OwnedHeaders(JsonHttpHeaders);

impl<'de> Deserialize<'de> for OwnedHeaders {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(Self(headers::deserialize(deserializer)?))
    }
}
