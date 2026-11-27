//! Judgment request validation tests.

use std::collections::BTreeMap;

use serde_json::{Map, json};

use crate::{
    ConversationMessage, ConversationRole, ErrorKind, JudgmentContent, JudgmentDescription,
    JudgmentError, JudgmentInput, JudgmentJson, JudgmentPart, JudgmentQuestion,
    JudgmentQuestionProblem, JudgmentRequest,
};

#[test]
fn request_uses_input_and_question_map_wire_shape() {
    let request = JudgmentRequest {
        input: JudgmentContent::try_from(json!({"ticket": "Payouts have failed"}))
            .unwrap()
            .into(),
        questions: BTreeMap::from([(
            "urgent".to_owned(),
            JudgmentQuestion::condition("Is the ticket urgent?", None),
        )]),
    };
    let expected = json!({
        "input": [{"type": "json", "value": {"ticket": "Payouts have failed"}}],
        "questions": {
            "urgent": {
                "type": "condition",
                "instructions": [{"type": "text", "text": "Is the ticket urgent?"}]
            }
        }
    });

    assert_eq!(serde_json::to_value(&request).unwrap(), expected);
    assert_eq!(
        serde_json::from_value::<JudgmentRequest>(expected).unwrap(),
        request
    );
}

#[test]
fn validation_rejects_every_empty_input_shape() {
    for input in [
        JudgmentContent { parts: vec![] }.into(),
        " \n\t".into(),
        JudgmentContent::try_from(json!({})).unwrap().into(),
        JudgmentContent::try_from(json!([])).unwrap().into(),
        JudgmentContent {
            parts: vec![
                JudgmentPart::Text {
                    text: " \n".to_owned(),
                },
                JudgmentPart::Json {
                    value: JudgmentJson::Object(Map::new()),
                },
                JudgmentPart::Json {
                    value: JudgmentJson::Array(Vec::new()),
                },
            ],
        }
        .into(),
        JudgmentInput::Conversation {
            system: Some("Directions alone".to_owned()),
            messages: Vec::new(),
        },
    ] {
        let result = request(input.clone(), valid_questions()).validate();
        assert!(
            matches!(&result, Err(JudgmentError::EmptyState { core }) if core.kind == ErrorKind::InvalidRequest),
            "{input:?}: {result:?}"
        );
    }
}

#[test]
fn validation_rejects_an_invalid_role_in_a_later_conversation_message() {
    let input = JudgmentInput::Conversation {
        system: None,
        messages: vec![
            ConversationMessage::user("ticket"),
            ConversationMessage::new(ConversationRole::Tool, vec![]),
        ],
    };

    let result = request(input, valid_questions()).validate();

    assert!(
        matches!(&result, Err(error)
            if error.core().kind == ErrorKind::InvalidRequest
            && error.core().provider == "local"
            && error.core().model_id == "judgment-validation"
            && error.core().message == "conversation parts do not match the message role"),
        "{result:?}"
    );
}

#[test]
fn validation_rejects_an_empty_question_map() {
    let error = request("ticket".into(), BTreeMap::new())
        .validate()
        .unwrap_err();

    assert!(matches!(error, JudgmentError::NoQuestions { .. }));
}

#[test]
fn validation_rejects_a_blank_question_map_key() {
    let questions = BTreeMap::from([(
        " \n".to_owned(),
        JudgmentQuestion::condition("Valid instructions", None),
    )]);

    assert_invalid_question(questions, " \n", JudgmentQuestionProblem::BlankId);
}

#[test]
fn validation_rejects_a_condition_without_instruction_parts() {
    let questions = BTreeMap::from([(
        "urgent".to_owned(),
        JudgmentQuestion::condition(JudgmentContent { parts: vec![] }, None),
    )]);

    assert_invalid_question(questions, "urgent", JudgmentQuestionProblem::NoInstructions);
}

#[test]
fn validation_rejects_a_choice_without_instruction_parts() {
    let questions = BTreeMap::from([(
        "team".to_owned(),
        JudgmentQuestion::choice(JudgmentContent { parts: vec![] }, [("billing", None)]),
    )]);

    assert_invalid_question(questions, "team", JudgmentQuestionProblem::NoInstructions);
}

#[test]
fn validation_rejects_a_score_without_instruction_parts() {
    let questions = BTreeMap::from([(
        "frustration".to_owned(),
        JudgmentQuestion::score(JudgmentContent { parts: vec![] }, ["low", "high"]),
    )]);

    assert_invalid_question(
        questions,
        "frustration",
        JudgmentQuestionProblem::NoInstructions,
    );
}

#[test]
fn validation_accepts_one_blank_text_instruction_part() {
    let questions = BTreeMap::from([("urgent".to_owned(), JudgmentQuestion::condition("", None))]);

    request("ticket".into(), questions).validate().unwrap();
}

#[test]
fn validation_rejects_a_choice_without_options() {
    let questions = BTreeMap::from([(
        "team".to_owned(),
        JudgmentQuestion::choice(
            "Choose a team",
            std::iter::empty::<(&str, Option<JudgmentDescription>)>(),
        ),
    )]);

    assert_invalid_question(questions, "team", JudgmentQuestionProblem::NoOptions);
}

#[test]
fn validation_rejects_a_blank_choice_option_label() {
    let questions = BTreeMap::from([(
        "team".to_owned(),
        JudgmentQuestion::choice("Choose a team", [(" \t", None)]),
    )]);

    assert_invalid_question(questions, "team", JudgmentQuestionProblem::BlankOptionLabel);
}

#[test]
fn validation_rejects_an_exact_duplicate_choice_option_label() {
    let questions = BTreeMap::from([(
        "team".to_owned(),
        JudgmentQuestion::choice(
            "Choose a team",
            [
                ("billing", None),
                ("technical", None),
                ("billing", Some("Payments".into())),
            ],
        ),
    )]);

    assert_invalid_question(
        questions,
        "team",
        JudgmentQuestionProblem::DuplicateOptionLabel,
    );
}

#[test]
fn validation_rejects_scores_with_fewer_than_two_levels() {
    for levels in [Vec::new(), vec!["only"]] {
        let questions = BTreeMap::from([(
            "frustration".to_owned(),
            JudgmentQuestion::score("Rate frustration", levels.clone()),
        )]);

        assert_invalid_question(
            questions,
            "frustration",
            JudgmentQuestionProblem::TooFewLevels {
                levels: levels.len(),
            },
        );
    }
}

#[test]
fn validation_accepts_every_valid_question_kind() {
    for input in [
        "ticket".into(),
        JudgmentContent {
            parts: vec![JudgmentPart::Image {
                mime_type: "image/png".to_owned(),
                data_base64: "AQID".to_owned(),
            }],
        }
        .into(),
        JudgmentInput::Conversation {
            system: None,
            messages: vec![
                ConversationMessage::user("ticket"),
                ConversationMessage::tool("ok", "lookup", "call-1"),
            ],
        },
    ] {
        request(input, valid_questions()).validate().unwrap();
    }
}

fn request(input: JudgmentInput, questions: BTreeMap<String, JudgmentQuestion>) -> JudgmentRequest {
    JudgmentRequest { input, questions }
}

fn valid_questions() -> BTreeMap<String, JudgmentQuestion> {
    BTreeMap::from([
        (
            "condition".to_owned(),
            JudgmentQuestion::condition("Is it valid?", None),
        ),
        (
            "choice".to_owned(),
            JudgmentQuestion::choice("Choose", [("yes", None), ("Yes", None), (" yes ", None)]),
        ),
        (
            "score".to_owned(),
            JudgmentQuestion::score("Score", ["low", "high"]),
        ),
    ])
}

fn assert_invalid_question(
    questions: BTreeMap<String, JudgmentQuestion>,
    expected_id: &str,
    expected_problem: JudgmentQuestionProblem,
) {
    let result = request("ticket".into(), questions).validate();
    assert!(
        matches!(
            &result,
            Err(JudgmentError::InvalidQuestion { id, problem, .. })
                if id == expected_id && *problem == expected_problem
        ),
        "{result:?}"
    );
}
