//! Judgment error contract tests.

use thiserror::Error;

use crate::{
    ErrorCore, ErrorKind, JudgmentAnswerProblem, JudgmentError, JudgmentJsonType,
    JudgmentQuestionProblem,
};

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
        JudgmentError::unsupported_content(JudgmentJsonType::Null),
        JudgmentError::empty_state(),
        JudgmentError::no_questions(),
        JudgmentError::invalid_question("question", JudgmentQuestionProblem::BlankId),
    ];

    for error in errors {
        assert!(error.to_string().starts_with("[ai_interface/judgment] "));
    }
}

#[test]
fn provider_error_constructors_preserve_typed_context_and_display_contract() {
    for (kind, message) in [
        (ErrorKind::RateLimited, "slow down"),
        (ErrorKind::Transient, "retry"),
        (ErrorKind::RejectedKey, "rejected"),
    ] {
        let error = JudgmentError::new(ErrorCore::new(kind, "typesafe", "jev-latest", message));
        assert_eq!(error.core().kind, kind);
        assert_eq!(error.core().provider, "typesafe");
        assert_eq!(error.core().model_id, "jev-latest");
        assert_eq!(error.core().message, message);
        assert_eq!(
            error.to_string(),
            format!(
                "[ai_interface/judgment] {kind:?} for `typesafe` model `jev-latest`: {message}"
            )
        );
    }
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
        JudgmentError::InvalidQuestion { id, problem, .. }
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
            core,
            id,
            problem: JudgmentAnswerProblem::Missing,
        } if core.provider == "typesafe" && core.model_id == "jev-latest" && id == "urgent"
            && core.kind == ErrorKind::InvalidOutput
    ));
}

#[test]
fn internal_error_preserves_contract_and_caller_locations() {
    let expected_line = line!() + 1;
    let error = JudgmentError::internal("provider", "model", TestSourceError);
    assert_eq!(
        error.to_string(),
        "[ai_interface/judgment] Internal for `provider` model `model`: internal error"
    );
    let JudgmentError::Internal {
        source: internal, ..
    } = error
    else {
        panic!("expected internal error");
    };

    assert_eq!(
        internal.defined_at().module_path(),
        "ai_interface::judgment::error"
    );
    assert_eq!(internal.caller_at().file(), file!());
    assert_eq!(internal.caller_at().line(), expected_line);
    assert_eq!(internal.source_ref().to_string(), "test source");

    let core = ErrorCore::new(
        ErrorKind::InvalidOutput,
        "provider",
        "model",
        "malformed output",
    );
    let expected_line = line!() + 1;
    let error = JudgmentError::with_source(core, TestSourceError);
    assert_eq!(error.core().kind, ErrorKind::InvalidOutput);
    assert_eq!(error.core().provider, "provider");
    assert_eq!(error.core().model_id, "model");
    let JudgmentError::Internal {
        source: internal, ..
    } = error
    else {
        panic!("expected the tracked source");
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
