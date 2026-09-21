//! Judgment answer serde tests.

use std::collections::BTreeMap;

use serde_json::json;

use crate::{JudgmentAnswer, JudgmentQuestionKind};

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
fn every_answer_variant_reports_its_kind() {
    let cases = [
        (
            JudgmentAnswer::Condition { probability: 1.0 },
            JudgmentQuestionKind::Condition,
        ),
        (
            JudgmentAnswer::Choice {
                selected: "yes".to_owned(),
                probabilities: BTreeMap::from([("yes".to_owned(), 1.0)]),
                confidence: 1.0,
            },
            JudgmentQuestionKind::Choice,
        ),
        (
            JudgmentAnswer::Score {
                expected: 0.0,
                probabilities: BTreeMap::from([(0, 1.0)]),
                confidence: 1.0,
            },
            JudgmentQuestionKind::Score,
        ),
    ];

    for (answer, expected) in cases {
        assert_eq!(answer.kind(), expected);
    }
}

#[test]
fn score_probabilities_reject_noncanonical_and_duplicate_text_keys() {
    let cases = [
        (
            r#"{"0":0.2,"00":0.8}"#,
            "a canonical unsigned decimal level index",
        ),
        (r#"{"0":0.5,"0":0.5}"#, "a unique level index"),
        (r#"{"+1":1.0}"#, "a canonical unsigned decimal level index"),
        (r#"{"-1":1.0}"#, "a canonical unsigned decimal level index"),
        (
            r#"{"4294967296":1.0}"#,
            "a canonical unsigned decimal level index",
        ),
        (r#"{" 1":1.0}"#, "a canonical unsigned decimal level index"),
    ];

    for (probabilities, expected) in cases {
        for type_first in [true, false] {
            let error =
                serde_json::from_str::<JudgmentAnswer>(&score_json(probabilities, type_first))
                    .unwrap_err();
            assert!(error.to_string().contains(expected), "{error}");
        }
    }
}

#[test]
fn score_probabilities_accept_canonical_text_keys_with_tag_in_any_position() {
    for type_first in [true, false] {
        let answer = serde_json::from_str::<JudgmentAnswer>(&score_json(
            r#"{"0":1.0,"10":0.0}"#,
            type_first,
        ))
        .unwrap();

        assert_eq!(
            answer,
            JudgmentAnswer::Score {
                expected: 0.0,
                probabilities: BTreeMap::from([(0, 1.0), (10, 0.0)]),
                confidence: 1.0,
            }
        );
    }
}

fn score_json(probabilities: &str, type_first: bool) -> String {
    if type_first {
        format!(
            r#"{{"type":"score","expected":0.0,"probabilities":{probabilities},"confidence":1.0}}"#
        )
    } else {
        format!(
            r#"{{"expected":0.0,"probabilities":{probabilities},"confidence":1.0,"type":"score"}}"#
        )
    }
}
