//! Provider identity serialization at the configuration boundary.

use serde_json::json;

use crate::{MOCK_PROVIDER, ProviderId};

#[test]
fn dynamic_provider_names_round_trip_as_plain_strings() {
    let provider = ProviderId::from(String::from("unlisted-provider"));
    let encoded = serde_json::to_value(&provider).expect("provider should serialize");
    let decoded: ProviderId =
        serde_json::from_value(json!("unlisted-provider")).expect("provider should deserialize");

    assert_eq!(encoded, json!("unlisted-provider"));
    assert_eq!(decoded, ProviderId::new("unlisted-provider"));
    assert_eq!(provider, decoded);
}

#[test]
fn mock_provider_keeps_its_stable_config_identity() {
    assert_eq!(MOCK_PROVIDER.as_str(), "mock");
    assert_eq!(MOCK_PROVIDER.to_string(), "mock");
    assert_eq!(serde_json::to_value(MOCK_PROVIDER).unwrap(), json!("mock"));
    assert_eq!(
        serde_json::from_value::<ProviderId>(json!("mock")).unwrap(),
        MOCK_PROVIDER
    );
}
