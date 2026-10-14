//! OpenAI Decisions request mapping tests.

use std::collections::BTreeMap;

use ai_interface::{JudgmentConditionCriteria, JudgmentContent, JudgmentQuestion, JudgmentRequest};
use serde_json::{Value, json};

use super::build_request;

#[test]
fn every_question_variant_maps_to_the_exact_wire_shape() {
    let request = JudgmentRequest {
        state: JudgmentContent::try_from(json!({
            "ticket": "Payouts have failed for three days",
            "attempts": 4,
            "escalated": true
        }))
        .expect("object state should be supported"),
        questions: BTreeMap::from([
            (
                "department".to_owned(),
                JudgmentQuestion::choice(
                    "Which team should handle this?",
                    [
                        ("billing", Some(JudgmentContent::from("Payments"))),
                        ("technical", None),
                    ],
                ),
            ),
            (
                "frustration".to_owned(),
                JudgmentQuestion::score(
                    "How frustrated is the author?",
                    ["Calm", "Frustrated", "Very angry"],
                ),
            ),
            (
                "is_urgent".to_owned(),
                JudgmentQuestion::condition(
                    "Does the ticket convey urgency?",
                    Some(JudgmentConditionCriteria::new(
                        Some(JudgmentContent::from("Time-sensitive")),
                        Some(JudgmentContent::try_from(json!(["no urgency"])).expect("array")),
                    )),
                ),
            ),
        ]),
    };

    let body = serialize(build_request("gpt-6-luna", &request));

    assert_eq!(
        body,
        json!({
            "model": "gpt-6-luna",
            "input": "{\"attempts\":4,\"escalated\":true,\"ticket\":\"Payouts have failed for three days\"}",
            "questions": [
                {
                    "type": "choice",
                    "name": "department",
                    "instructions": "Which team should handle this?",
                    "choices": [
                        {"value": "billing", "description": "Payments"},
                        {"value": "technical"}
                    ]
                },
                {
                    "type": "score",
                    "name": "frustration",
                    "instructions": "How frustrated is the author?",
                    "levels": [
                        {"label": "Calm"},
                        {"label": "Frustrated"},
                        {"label": "Very angry"}
                    ]
                },
                {
                    "type": "predicate",
                    "name": "is_urgent",
                    "instructions": "Does the ticket convey urgency?\n\nTrue when: Time-sensitive\nFalse when: [\"no urgency\"]"
                }
            ]
        })
    );
}

#[test]
fn absent_empty_and_partial_condition_criteria_fold_only_present_lines() {
    let request = JudgmentRequest {
        state: JudgmentContent::try_from(json!(["verbatim", null, 3]))
            .expect("array state should be supported"),
        questions: BTreeMap::from([
            (
                "absent".to_owned(),
                JudgmentQuestion::condition("No rubric", None),
            ),
            (
                "empty".to_owned(),
                JudgmentQuestion::condition(
                    "Empty rubric",
                    Some(JudgmentConditionCriteria::default()),
                ),
            ),
            (
                "no_only".to_owned(),
                JudgmentQuestion::condition(
                    "Negative rubric",
                    Some(JudgmentConditionCriteria::new(
                        None,
                        Some(JudgmentContent::from("no evidence")),
                    )),
                ),
            ),
            (
                "yes_only".to_owned(),
                JudgmentQuestion::condition(
                    "Positive rubric",
                    Some(JudgmentConditionCriteria::new(
                        Some(JudgmentContent::from("yes evidence")),
                        None,
                    )),
                ),
            ),
        ]),
    };

    let body = serialize(build_request("gpt-6-luna", &request));

    assert_eq!(body["input"], "[\"verbatim\",null,3]");
    assert_eq!(
        body["questions"],
        json!([
            {"type": "predicate", "name": "absent", "instructions": "No rubric"},
            {"type": "predicate", "name": "empty", "instructions": "Empty rubric"},
            {
                "type": "predicate",
                "name": "no_only",
                "instructions": "Negative rubric\n\nFalse when: no evidence"
            },
            {
                "type": "predicate",
                "name": "yes_only",
                "instructions": "Positive rubric\n\nTrue when: yes evidence"
            }
        ])
    );
}

#[test]
fn structured_instructions_and_descriptions_render_as_compact_json() {
    let choice_instructions = JudgmentContent::try_from(json!({
        "task": "route",
        "signals": ["invoice", "charge"]
    }))
    .expect("object instructions should be supported");
    let score_instructions = JudgmentContent::try_from(json!([
        "Assess urgency",
        {"window_hours": 24}
    ]))
    .expect("array instructions should be supported");
    let option_description = JudgmentContent::try_from(json!({
        "owner": "payments",
        "priority": 1
    }))
    .expect("object option description should be supported");
    let level_description = JudgmentContent::try_from(json!({
        "label": "critical",
        "requires_page": true
    }))
    .expect("object level description should be supported");
    let request = JudgmentRequest {
        state: "ticket".into(),
        questions: BTreeMap::from([
            (
                "choice".to_owned(),
                JudgmentQuestion::choice(
                    choice_instructions,
                    [("billing", Some(option_description))],
                ),
            ),
            (
                "score".to_owned(),
                JudgmentQuestion::score(
                    score_instructions,
                    [JudgmentContent::from("Routine"), level_description],
                ),
            ),
        ]),
    };

    let body = serialize(build_request("gpt-6-luna", &request));

    assert_eq!(
        body["questions"],
        json!([
            {
                "type": "choice",
                "name": "choice",
                "instructions": "{\"signals\":[\"invoice\",\"charge\"],\"task\":\"route\"}",
                "choices": [
                    {"value": "billing", "description": "{\"owner\":\"payments\",\"priority\":1}"}
                ]
            },
            {
                "type": "score",
                "name": "score",
                "instructions": "[\"Assess urgency\",{\"window_hours\":24}]",
                "levels": [
                    {"label": "Routine"},
                    {"label": "{\"label\":\"critical\",\"requires_page\":true}"}
                ]
            }
        ])
    );
}

#[test]
fn text_state_and_model_are_verbatim_without_extra_fields() {
    let request = JudgmentRequest {
        state: "  preserve surrounding whitespace  ".into(),
        questions: BTreeMap::from([(
            "condition".to_owned(),
            JudgmentQuestion::condition("Question", None),
        )]),
    };

    let body = serialize(build_request("gpt-7-unlisted", &request));

    assert_eq!(body["input"], "  preserve surrounding whitespace  ");
    assert_eq!(body["model"], "gpt-7-unlisted");
    assert_eq!(
        body.as_object()
            .expect("request should be an object")
            .keys()
            .collect::<Vec<_>>(),
        vec!["input", "model", "questions"]
    );
    assert_eq!(
        body["questions"][0]
            .as_object()
            .expect("question should be an object")
            .keys()
            .collect::<Vec<_>>(),
        vec!["instructions", "name", "type"]
    );
}

fn serialize(request: ai_interface::JudgmentResult<super::DecisionsRequest<'_>>) -> Value {
    serde_json::to_value(request.expect("request mapping should succeed"))
        .expect("wire request should serialize")
}
