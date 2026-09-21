//! Judgment content conversion and serde tests.

use serde_json::{Map, Value, json};

use crate::{JudgmentContent, JudgmentError, JudgmentJsonType};

#[test]
fn content_uses_untagged_text_object_and_array_shapes() {
    let cases = [
        (JudgmentContent::from("plain text"), json!("plain text")),
        (
            JudgmentContent::Object(Map::from_iter([("priority".to_owned(), json!("high"))])),
            json!({"priority": "high"}),
        ),
        (
            JudgmentContent::Array(vec![json!("first"), json!({"second": true})]),
            json!(["first", {"second": true}]),
        ),
    ];

    for (content, expected) in cases {
        assert_eq!(serde_json::to_value(&content).unwrap(), expected);
        assert_eq!(
            serde_json::from_value::<JudgmentContent>(expected).unwrap(),
            content
        );
    }
}

#[test]
fn string_conversions_build_text_content() {
    assert_eq!(
        JudgmentContent::from("borrowed"),
        JudgmentContent::Text("borrowed".to_owned())
    );
    assert_eq!(
        JudgmentContent::from("owned".to_owned()),
        JudgmentContent::Text("owned".to_owned())
    );
}

#[test]
fn json_value_conversion_accepts_supported_shapes() {
    let cases = [
        (json!("text"), JudgmentContent::Text("text".to_owned())),
        (
            json!({"nested": {"value": 3}}),
            JudgmentContent::Object(Map::from_iter([("nested".to_owned(), json!({"value": 3}))])),
        ),
        (
            json!([1, "two"]),
            JudgmentContent::Array(vec![json!(1), json!("two")]),
        ),
    ];

    for (value, expected) in cases {
        assert_eq!(JudgmentContent::try_from(value).unwrap(), expected);
    }
}

#[test]
fn json_value_conversion_rejects_unsupported_shapes_with_typed_errors() {
    let cases = [
        (Value::Null, JudgmentJsonType::Null),
        (Value::Bool(true), JudgmentJsonType::Boolean),
        (json!(42), JudgmentJsonType::Number),
    ];

    for (value, expected_type) in cases {
        let error = JudgmentContent::try_from(value).unwrap_err();
        assert!(matches!(
            error,
            JudgmentError::UnsupportedContent { json_type } if json_type == expected_type
        ));
    }
}

#[test]
fn unsupported_json_types_use_stable_snake_case_serde_values() {
    for (json_type, expected) in [
        (JudgmentJsonType::Null, json!("null")),
        (JudgmentJsonType::Boolean, json!("boolean")),
        (JudgmentJsonType::Number, json!("number")),
    ] {
        assert_eq!(serde_json::to_value(json_type).unwrap(), expected);
        assert_eq!(
            serde_json::from_value::<JudgmentJsonType>(expected).unwrap(),
            json_type
        );
    }
}
