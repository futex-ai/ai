//! Duplicate-rejecting map deserialization tests.

use std::collections::BTreeMap;

use serde::Deserialize;

use super::deserialize_unique_string_map;

#[derive(Debug, Deserialize, PartialEq)]
struct Wrapper {
    #[serde(deserialize_with = "deserialize_unique_string_map")]
    entries: BTreeMap<String, f64>,
}

#[test]
fn unique_keys_deserialize_in_sorted_order() {
    let wrapper = serde_json::from_str::<Wrapper>(r#"{"entries":{"b":2.0,"a":1.0}}"#).unwrap();

    assert_eq!(
        wrapper.entries,
        BTreeMap::from([("a".to_owned(), 1.0), ("b".to_owned(), 2.0)])
    );
}

#[test]
fn repeated_keys_are_rejected_instead_of_last_wins() {
    let error = serde_json::from_str::<Wrapper>(r#"{"entries":{"a":1.0,"a":2.0}}"#).unwrap_err();

    assert!(error.to_string().contains("a unique map key"), "{error}");
}

#[test]
fn empty_maps_are_accepted() {
    let wrapper = serde_json::from_str::<Wrapper>(r#"{"entries":{}}"#).unwrap();

    assert!(wrapper.entries.is_empty());
}
