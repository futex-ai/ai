//! Judgment answer serde tests.

use std::collections::BTreeMap;

use serde_json::json;

use crate::JudgmentAnswer;

#[test]
fn every_answer_variant_uses_a_snake_case_type_tag() {
    let cases = [
        (
            JudgmentAnswer::Condition { probability: 0.75 },
            json!({"type": "condition", "probability": 0.75}),
        ),
        (
            JudgmentAnswer::Choice {
                selected: "billing".to_owned(),
                probabilities: BTreeMap::from([
                    ("billing".to_owned(), 0.8),
                    ("technical".to_owned(), 0.2),
                ]),
                confidence: 0.6,
            },
            json!({
                "type": "choice",
                "selected": "billing",
                "probabilities": {"billing": 0.8, "technical": 0.2},
                "confidence": 0.6
            }),
        ),
        (
            JudgmentAnswer::Score {
                expected: 1.25,
                probabilities: BTreeMap::from([(0, 0.1), (1, 0.55), (2, 0.35)]),
                confidence: 0.45,
            },
            json!({
                "type": "score",
                "expected": 1.25,
                "probabilities": {"0": 0.1, "1": 0.55, "2": 0.35},
                "confidence": 0.45
            }),
        ),
    ];

    for (answer, expected) in cases {
        assert_eq!(serde_json::to_value(&answer).unwrap(), expected);
        assert_eq!(
            serde_json::from_value::<JudgmentAnswer>(expected).unwrap(),
            answer
        );
    }
}

#[test]
fn score_probabilities_round_trip_through_json_text_with_string_keys() {
    let answer = JudgmentAnswer::Score {
        expected: 1.5,
        probabilities: BTreeMap::from([(0, 0.0), (1, 0.5), (2, 0.5)]),
        confidence: 0.5,
    };

    let text = serde_json::to_string(&answer).unwrap();

    assert!(text.contains("\"probabilities\":{\"0\":0.0,\"1\":0.5,\"2\":0.5}"));
    assert_eq!(
        serde_json::from_str::<JudgmentAnswer>(&text).unwrap(),
        answer
    );
}

#[test]
fn score_probabilities_reject_non_decimal_level_keys() {
    let error = serde_json::from_value::<JudgmentAnswer>(json!({
        "type": "score",
        "expected": 0.0,
        "probabilities": {"calm": 1.0},
        "confidence": 1.0
    }))
    .unwrap_err();

    assert!(error.to_string().contains("calm"));
}
