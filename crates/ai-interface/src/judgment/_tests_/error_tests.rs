//! Judgment error contract tests.

use thiserror::Error;

use crate::{JudgmentAnswerProblem, JudgmentError, JudgmentJsonType, JudgmentQuestionProblem};

#[test]
fn question_problems_have_specific_display_messages() {
    let cases = [
        (
            JudgmentQuestionProblem::BlankId,
            "question id must not be blank",
        ),
        (
            JudgmentQuestionProblem::NoOptions,
            "choice must include at least one option",
        ),
        (
            JudgmentQuestionProblem::BlankOptionLabel,
            "choice option labels must not be blank",
        ),
        (
            JudgmentQuestionProblem::TooFewLevels { levels: 1 },
            "score must include at least two levels, got 1",
        ),
    ];

    for (problem, expected) in cases {
        assert_eq!(problem.to_string(), expected);
    }
}

#[test]
fn local_errors_use_the_judgment_display_prefix() {
    let errors = [
        JudgmentError::UnsupportedContent {
            json_type: JudgmentJsonType::Null,
        },
        JudgmentError::EmptyState,
        JudgmentError::NoQuestions,
        JudgmentError::invalid_question("question", JudgmentQuestionProblem::BlankId),
    ];

    for error in errors {
        assert!(error.to_string().starts_with("[ai_interface/judgment] "));
    }
}

#[test]
fn provider_error_constructors_preserve_typed_context_and_display_contract() {
    let rate_limited = JudgmentError::rate_limited("typesafe", "jev-latest", "slow down");
    assert_eq!(
        rate_limited.to_string(),
        "[ai_interface/judgment] provider rate limit for `typesafe` model `jev-latest`: slow down"
    );
    assert!(matches!(
        rate_limited,
        JudgmentError::RateLimited { provider, model_id, message }
            if provider == "typesafe" && model_id == "jev-latest" && message == "slow down"
    ));

    let transient = JudgmentError::transient_provider("typesafe", "jev-latest", "retry");
    assert_eq!(
        transient.to_string(),
        "[ai_interface/judgment] transient provider failure for `typesafe` model `jev-latest`: retry"
    );
    assert!(matches!(
        transient,
        JudgmentError::TransientProvider { provider, model_id, message }
            if provider == "typesafe" && model_id == "jev-latest" && message == "retry"
    ));

    let provider = JudgmentError::provider("typesafe", "jev-latest", "rejected");
    assert_eq!(
        provider.to_string(),
        "[ai_interface/judgment] provider failure for `typesafe` model `jev-latest`: rejected"
    );
    assert!(matches!(
        provider,
        JudgmentError::Provider { provider, model_id, message }
            if provider == "typesafe" && model_id == "jev-latest" && message == "rejected"
    ));
}

#[test]
fn invalid_question_constructor_preserves_id_and_problem() {
    let error = JudgmentError::invalid_question(
        "frustration",
        JudgmentQuestionProblem::TooFewLevels { levels: 1 },
    );

    assert_eq!(
        error.to_string(),
        "[ai_interface/judgment] invalid question `frustration`: score must include at least two levels, got 1"
    );
    assert!(matches!(
        error,
        JudgmentError::InvalidQuestion { id, problem }
            if id == "frustration"
                && problem == JudgmentQuestionProblem::TooFewLevels { levels: 1 }
    ));
}

#[test]
fn invalid_answer_constructor_preserves_context_and_problem() {
    let error = JudgmentError::invalid_answer(
        "typesafe",
        "jev-latest",
        "urgent",
        JudgmentAnswerProblem::Missing,
    );

    assert_eq!(
        error.to_string(),
        "[ai_interface/judgment] invalid answer `urgent` from `typesafe` model `jev-latest`: answer is missing"
    );
    assert!(matches!(
        error,
        JudgmentError::InvalidAnswer {
            provider,
            model_id,
            id,
            problem: JudgmentAnswerProblem::Missing,
        } if provider == "typesafe" && model_id == "jev-latest" && id == "urgent"
    ));
}

#[test]
fn internal_error_preserves_contract_and_caller_locations() {
    let expected_line = line!() + 1;
    let error = JudgmentError::internal(TestSourceError);
    assert_eq!(error.to_string(), "[ai_interface/judgment] internal error");
    let JudgmentError::Internal(internal) = error else {
        panic!("expected internal error");
    };

    assert_eq!(
        internal.defined_at().module_path(),
        "ai_interface::judgment::error"
    );
    assert_eq!(internal.caller_at().file(), file!());
    assert_eq!(internal.caller_at().line(), expected_line);
    assert_eq!(internal.source_ref().to_string(), "test source");
}

#[derive(Debug, Error)]
#[error("test source")]
struct TestSourceError;
