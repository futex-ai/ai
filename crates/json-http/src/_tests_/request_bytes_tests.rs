//! Wire-byte contract for typed requests and pre-dispatch serialization errors.

use std::{collections::BTreeMap, sync::Arc};

use serde::Serialize;
use serde_json::json;
use serde_json::value::RawValue;
use unimock::{MockFn, Unimock, matching};

use crate::{
    Error, JsonHttpClient, JsonHttpRequest, JsonHttpResponse, JsonHttpTransportMock,
    TransportBackedJsonHttpClient,
};

#[derive(Serialize)]
struct TypedBody {
    z: Vec<Part>,
    a: BTreeMap<u32, &'static str>,
    float: f32,
    raw: Box<RawValue>,
}

#[derive(Serialize)]
enum Part {
    Unit,
    Newtype(&'static str),
    Tuple(u32, bool),
    Object { z: u32, a: Option<u32> },
}

#[tokio::test]
async fn typed_request_serialization_keeps_sorted_wire_bytes() {
    let transport = Arc::new(Unimock::new(
        JsonHttpTransportMock::execute.some_call(matching!(_)).answers(&|_, request: &JsonHttpRequest| {
            assert_eq!(
                request.body.as_ref().unwrap().as_json_bytes().unwrap(),
                br#"{"a":{"10":"ten","2":"two"},"float":1.100000023841858,"raw":{"a":{"a":1,"z":2},"dup":3,"z":100.0},"z":["Unit",{"Newtype":"hello"},{"Tuple":[3,true]},{"Object":{"a":null,"z":4}}]}"#
            );
            Ok(JsonHttpResponse { status: 200, headers: Default::default(), body: json!({}) })
        }),
    ));
    let client = TransportBackedJsonHttpClient::new(transport);
    client
        .post("https://example.com/typed")
        .json(TypedBody {
            z: vec![
                Part::Unit,
                Part::Newtype("hello"),
                Part::Tuple(3, true),
                Part::Object { z: 4, a: None },
            ],
            a: BTreeMap::from([(2, "two"), (10, "ten")]),
            float: 1.1,
            raw: serde_json::from_str(r#"{ "z": 1e2, "a": { "z":2, "a":1 }, "dup":0, "dup":3 }"#)
                .unwrap(),
        })
        .unwrap()
        .send_value()
        .await
        .unwrap();
}

#[test]
fn out_of_range_typed_number_fails_before_dispatch() {
    let client = TransportBackedJsonHttpClient::new(Arc::new(Unimock::new(())));
    let result = client.post("https://example.com/typed").json(u128::MAX);
    assert!(
        matches!(result, Err(Error::SerializeRequest { source }) if source.to_string() == "number out of range")
    );
}

#[test]
fn optional_map_key_fails_before_dispatch() {
    let client = TransportBackedJsonHttpClient::new(Arc::new(Unimock::new(())));
    let result = client
        .post("https://example.com/typed")
        .json(BTreeMap::from([(Some("key"), true)]));
    assert!(
        matches!(result, Err(Error::SerializeRequest { source }) if source.to_string() == "key must be a string")
    );
}
