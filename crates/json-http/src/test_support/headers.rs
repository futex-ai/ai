//! Portable encoding of repeated response headers without losing binary values.

use std::collections::BTreeMap;

use reqwest::header::{HeaderName, HeaderValue};
use serde::{Deserialize, Deserializer, Serialize, Serializer, de::Error};

use crate::JsonHttpHeaders;

pub(super) fn serialize<S>(headers: &JsonHttpHeaders, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let mut values = BTreeMap::<&str, Vec<&[u8]>>::new();
    for (name, value) in headers {
        values
            .entry(name.as_str())
            .or_default()
            .push(value.as_bytes());
    }
    values.serialize(serializer)
}

pub(super) fn deserialize<'de, D>(deserializer: D) -> Result<JsonHttpHeaders, D::Error>
where
    D: Deserializer<'de>,
{
    let values = BTreeMap::<String, Vec<Vec<u8>>>::deserialize(deserializer)?;
    let mut headers = JsonHttpHeaders::new();
    for (name, values) in values {
        let name = match HeaderName::from_bytes(name.as_bytes()) {
            Ok(name) => name,
            Err(error) => return Err(D::Error::custom(error)),
        };
        for value in values {
            let value = match HeaderValue::from_bytes(&value) {
                Ok(value) => value,
                Err(error) => return Err(D::Error::custom(error)),
            };
            headers.append(name.clone(), value);
        }
    }
    Ok(headers)
}
