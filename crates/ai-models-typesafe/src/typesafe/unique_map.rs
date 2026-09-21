//! Duplicate-rejecting deserialization for provider string-keyed maps.

use std::{collections::BTreeMap, fmt, marker::PhantomData};

use serde::de::{Error as DeError, MapAccess, Unexpected, Visitor};
use serde::{Deserialize, Deserializer};

/// Deserializes a string-keyed map while rejecting repeated keys.
///
/// `serde_json` lets the last repeated key win when filling a `BTreeMap`,
/// which would let an ambiguous provider payload become a successful
/// judgment. Provider answer ids and choice labels use this helper instead.
pub(super) fn deserialize_unique_string_map<'de, D, V>(
    deserializer: D,
) -> Result<BTreeMap<String, V>, D::Error>
where
    D: Deserializer<'de>,
    V: Deserialize<'de>,
{
    deserializer.deserialize_map(UniqueStringMapVisitor(PhantomData))
}

struct UniqueStringMapVisitor<V>(PhantomData<V>);

impl<'de, V> Visitor<'de> for UniqueStringMapVisitor<V>
where
    V: Deserialize<'de>,
{
    type Value = BTreeMap<String, V>;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a map with unique string keys")
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut entries = BTreeMap::new();
        while let Some(key) = map.next_key::<String>()? {
            if entries.contains_key(&key) {
                return Err(A::Error::invalid_value(
                    Unexpected::Str(&key),
                    &"a unique map key",
                ));
            }
            let value = map.next_value::<V>()?;
            entries.insert(key, value);
        }
        Ok(entries)
    }
}

#[cfg(test)]
#[path = "_tests_/unique_map_tests.rs"]
mod unique_map_tests;
