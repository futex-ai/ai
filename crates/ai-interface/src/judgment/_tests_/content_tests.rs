//! Judgment content conversion and serde tests.

use serde_json::{Map, Value, json};

use crate::{
    ErrorKind, JudgmentContent, JudgmentDescription, JudgmentError, JudgmentJson, JudgmentJsonType,
    JudgmentPart,
};

#[test]
fn content_uses_tagged_parts_in_order() {
    let content = JudgmentContent {
        parts: vec![
            JudgmentPart::Text {
                text: "plain text".to_owned(),
            },
            JudgmentPart::Json {
                value: JudgmentJson::Object(Map::from_iter([(
                    "priority".to_owned(),
                    json!("high"),
                )])),
            },
            JudgmentPart::Image {
                mime_type: "image/png".to_owned(),
                data_base64: "AQID".to_owned(),
            },
            JudgmentPart::Json {
                value: JudgmentJson::Array(vec![json!("first"), json!({"second": true})]),
            },
        ],
    };
    let expected = json!([
        {"type": "text", "text": "plain text"},
        {"type": "json", "value": {"priority": "high"}},
        {"type": "image", "mime_type": "image/png", "data_base64": "AQID"},
        {"type": "json", "value": ["first", {"second": true}]}
    ]);

    assert_eq!(serde_json::to_value(&content).unwrap(), expected);
    assert_eq!(
        serde_json::from_value::<JudgmentContent>(expected).unwrap(),
        content
    );
}

#[test]
fn json_parts_reject_scalar_values() {
    for value in [Value::Null, json!(true), json!(42), json!("text")] {
        let result = serde_json::from_value::<JudgmentPart>(json!({
            "type": "json",
            "value": value
        }));
        assert!(
            matches!(&result, Err(error) if error.is_data()),
            "{value}: {result:?}"
        );
    }
}

#[test]
fn string_conversions_build_text_content() {
    assert_eq!(
        JudgmentContent::from("borrowed"),
        JudgmentContent {
            parts: vec![JudgmentPart::Text {
                text: "borrowed".to_owned()
            }],
        }
    );
    assert_eq!(
        JudgmentContent::from("owned".to_owned()),
        JudgmentContent {
            parts: vec![JudgmentPart::Text {
                text: "owned".to_owned()
            }],
        }
    );
}

#[test]
fn json_value_conversion_accepts_supported_shapes() {
    let cases = [
        (
            json!("text"),
            JudgmentPart::Text {
                text: "text".to_owned(),
            },
        ),
        (
            json!({"nested": {"value": 3}}),
            JudgmentPart::Json {
                value: JudgmentJson::Object(Map::from_iter([(
                    "nested".to_owned(),
                    json!({"value": 3}),
                )])),
            },
        ),
        (
            json!([1, "two"]),
            JudgmentPart::Json {
                value: JudgmentJson::Array(vec![json!(1), json!("two")]),
            },
        ),
    ];

    for (value, part) in cases {
        assert_eq!(
            JudgmentContent::try_from(value).unwrap(),
            JudgmentContent { parts: vec![part] }
        );
    }
}

#[test]
fn descriptions_round_trip_as_text_or_json() {
    assert_eq!(
        JudgmentDescription::from("borrowed"),
        JudgmentDescription::Text("borrowed".to_owned())
    );
    assert_eq!(
        JudgmentDescription::from("owned".to_owned()),
        JudgmentDescription::Text("owned".to_owned())
    );
    for (description, expected) in [
        (JudgmentDescription::Text("text".to_owned()), json!("text")),
        (
            JudgmentDescription::Json(JudgmentJson::Object(Map::from_iter([(
                "rule".to_owned(),
                json!(true),
            )]))),
            json!({"rule": true}),
        ),
        (
            JudgmentDescription::Json(JudgmentJson::Array(vec![json!(1), json!("two")])),
            json!([1, "two"]),
        ),
    ] {
        assert_eq!(serde_json::to_value(&description).unwrap(), expected);
        assert_eq!(
            serde_json::from_value::<JudgmentDescription>(expected.clone()).unwrap(),
            description
        );
        assert_eq!(
            JudgmentDescription::try_from(expected).unwrap(),
            description
        );
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
        let serde_result = serde_json::from_value::<JudgmentDescription>(value.clone());
        assert!(
            matches!(&serde_result, Err(error) if error.is_data()),
            "{value}: {serde_result:?}"
        );
        for error in [
            JudgmentContent::try_from(value.clone()).unwrap_err(),
            JudgmentDescription::try_from(value).unwrap_err(),
        ] {
            assert!(matches!(
                error,
                JudgmentError::UnsupportedContent { json_type, core } if json_type == expected_type
                    && core.kind == ErrorKind::InvalidRequest && core.provider == "local"
                    && core.model_id == "judgment-validation"
            ));
        }
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
