//! Judgment question DTO and constructor tests.

use std::collections::BTreeMap;

use serde_json::json;

use crate::{JudgmentConditionCriteria, JudgmentContent, JudgmentQuestion, JudgmentQuestionKind};

#[test]
fn condition_criteria_omits_absent_descriptions() {
    let criteria =
        JudgmentConditionCriteria::new(Some("explicit yes".into()), Some("explicit no".into()));

    assert_eq!(
        serde_json::to_value(&criteria).unwrap(),
        json!({"yes": "explicit yes", "no": "explicit no"})
    );
    assert_eq!(
        serde_json::from_value::<JudgmentConditionCriteria>(json!({
            "yes": "explicit yes",
            "no": "explicit no"
        }))
        .unwrap(),
        criteria
    );
    assert_eq!(
        serde_json::to_value(JudgmentConditionCriteria::new(
            Some("explicit yes".into()),
            None,
        ))
        .unwrap(),
        json!({"yes": "explicit yes"})
    );
    assert_eq!(
        serde_json::from_value::<JudgmentConditionCriteria>(json!({})).unwrap(),
        JudgmentConditionCriteria::default()
    );
}

#[test]
fn every_question_variant_uses_a_snake_case_type_tag() {
    let cases = [
        (
            JudgmentQuestion::condition("Is this urgent?", None),
            json!({
                "type": "condition",
                "instructions": "Is this urgent?"
            }),
        ),
        (
            JudgmentQuestion::choice(
                "Choose a team",
                [
                    ("billing", Some(JudgmentContent::from("Payments"))),
                    ("technical", None),
                ],
            ),
            json!({
                "type": "choice",
                "instructions": "Choose a team",
                "options": {
                    "billing": "Payments",
                    "technical": null
                }
            }),
        ),
        (
            JudgmentQuestion::score(
                "Rate frustration",
                [Some(JudgmentContent::from("Calm")), None],
            ),
            json!({
                "type": "score",
                "instructions": "Rate frustration",
                "levels": ["Calm", null]
            }),
        ),
    ];

    for (question, expected) in cases {
        assert_eq!(serde_json::to_value(&question).unwrap(), expected);
        assert_eq!(
            serde_json::from_value::<JudgmentQuestion>(expected).unwrap(),
            question
        );
    }
}

#[test]
fn question_kinds_use_stable_snake_case_serde_values() {
    for (kind, expected) in [
        (JudgmentQuestionKind::Condition, json!("condition")),
        (JudgmentQuestionKind::Choice, json!("choice")),
        (JudgmentQuestionKind::Score, json!("score")),
    ] {
        assert_eq!(serde_json::to_value(kind).unwrap(), expected);
        assert_eq!(
            serde_json::from_value::<JudgmentQuestionKind>(expected).unwrap(),
            kind
        );
    }
}

#[test]
fn every_question_variant_reports_its_kind() {
    let cases = [
        (
            JudgmentQuestion::condition("condition", None),
            JudgmentQuestionKind::Condition,
        ),
        (
            JudgmentQuestion::choice("choice", [("yes", None)]),
            JudgmentQuestionKind::Choice,
        ),
        (
            JudgmentQuestion::score("score", [None, None]),
            JudgmentQuestionKind::Score,
        ),
    ];

    for (question, expected) in cases {
        assert_eq!(question.kind(), expected);
    }
}

#[test]
fn questions_round_trip_structured_instructions_and_descriptions() {
    let cases = [
        json!({
            "type": "condition",
            "instructions": {"field": "ticket", "rule": "urgent"},
            "criteria": {
                "yes": {"signals": ["blocked", "deadline"]},
                "no": ["informational", {"resolved": true}]
            }
        }),
        json!({
            "type": "choice",
            "instructions": ["route", {"using": "department"}],
            "options": {
                "billing": {"owns": ["payments", "refunds"]},
                "technical": {"owns": ["availability"]}
            }
        }),
        json!({
            "type": "score",
            "instructions": {"measure": "frustration"},
            "levels": [
                {"label": "calm"},
                {"label": "frustrated"},
                {"label": "angry"}
            ]
        }),
    ];

    for expected in cases {
        let question = serde_json::from_value::<JudgmentQuestion>(expected.clone()).unwrap();
        assert_eq!(serde_json::to_value(question).unwrap(), expected);
    }
}

#[test]
fn constructors_preserve_supplied_values_without_validation() {
    let criteria = JudgmentConditionCriteria::new(
        Some(JudgmentContent::from("yes")),
        Some(JudgmentContent::from("no")),
    );
    assert_eq!(
        JudgmentQuestion::condition("condition", Some(criteria.clone())),
        JudgmentQuestion::Condition {
            instructions: JudgmentContent::from("condition"),
            criteria: Some(criteria),
        }
    );

    assert_eq!(
        JudgmentQuestion::choice("choice", [("", None), ("second", Some("two".into()))]),
        JudgmentQuestion::Choice {
            instructions: JudgmentContent::from("choice"),
            options: BTreeMap::from([
                (String::new(), None),
                ("second".to_owned(), Some(JudgmentContent::from("two"))),
            ]),
        }
    );

    assert_eq!(
        JudgmentQuestion::score("score", [Some("only".into())]),
        JudgmentQuestion::Score {
            instructions: JudgmentContent::from("score"),
            levels: vec![Some(JudgmentContent::from("only"))],
        }
    );
}
