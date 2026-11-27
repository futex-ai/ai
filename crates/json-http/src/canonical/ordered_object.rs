//! JSON object with caller-defined field order.

use serde::{Serialize, Serializer};

pub(super) const ORDERED_OBJECT_TOKEN: &str = "$json_http::private::OrderedObject";

/// JSON object whose keys are written in insertion order by the request builder.
///
/// A repeated key keeps its first position and uses its last value.
#[derive(Debug)]
pub struct OrderedObject<K, V> {
    entries: Vec<(K, V)>,
}

impl<K, V> OrderedObject<K, V> {
    /// Builds an ordered object from key-value pairs in iteration order.
    pub fn new(entries: impl IntoIterator<Item = (K, V)>) -> Self {
        Self {
            entries: entries.into_iter().collect(),
        }
    }
}

impl<K: AsRef<str>, V: Serialize> Serialize for OrderedObject<K, V> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_newtype_struct(ORDERED_OBJECT_TOKEN, &Entries(&self.entries))
    }
}

struct Entries<'a, K, V>(&'a [(K, V)]);

impl<K: AsRef<str>, V: Serialize> Serialize for Entries<'_, K, V> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_map(self.0.iter().map(|(key, value)| (key.as_ref(), value)))
    }
}
