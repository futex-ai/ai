//! Judgment error and answer-problem redaction tests.

use ai_interface::{JudgmentAnswerProblem, JudgmentError, JudgmentQuestionKind};

use super::{redact_answer_problem, redact_judgment_error};

const SECRET: &str = "provider-secret-key";

#[test]
fn provider_messages_are_redacted() {
    let error = redact_judgment_error(
        JudgmentError::provider("openai", "gpt-6-luna", "malformed provider-secret-key"),
        &[SECRET.to_owned()],
    );

    assert!(matches!(
        error,
        JudgmentError::Provider { provider, model_id, message }
            if provider == "openai" && model_id == "gpt-6-luna" && message == "malformed [redacted]"
    ));
}

#[test]
fn invalid_answer_labels_are_redacted_and_context_is_kept() {
    let error = redact_judgment_error(
        JudgmentError::invalid_answer(
            "typesafe",
            "jev-latest",
            "team",
            JudgmentAnswerProblem::UnknownOption {
                label: SECRET.to_owned(),
            },
        ),
        &[SECRET.to_owned()],
    );

    assert!(matches!(
        error,
        JudgmentError::InvalidAnswer {
            provider,
            model_id,
            id,
            problem: JudgmentAnswerProblem::UnknownOption { label },
        } if provider == "typesafe"
            && model_id == "jev-latest"
            && id == "team"
            && label == "[redacted]"
    ));
}

#[test]
fn errors_without_provider_text_are_unchanged() {
    let secrets = [SECRET.to_owned()];
    let refused = redact_judgment_error(
        JudgmentError::refused("openai", "gpt-6-luna", "urgent"),
        &secrets,
    );
    assert!(matches!(refused, JudgmentError::Refused { id, .. } if id == "urgent"));

    let rate_limited = redact_judgment_error(
        JudgmentError::rate_limited("openai", "gpt-6-luna", SECRET),
        &secrets,
    );
    assert!(matches!(
        rate_limited,
        JudgmentError::RateLimited { message, .. } if message == SECRET
    ));
}

#[test]
fn redacts_unknown_and_missing_option_labels() {
    let secrets = [SECRET.to_owned()];

    assert_eq!(
        redact_answer_problem(
            JudgmentAnswerProblem::UnknownOption {
                label: "unexpected provider-secret-key".to_owned(),
            },
            &secrets,
        ),
        JudgmentAnswerProblem::UnknownOption {
            label: "unexpected [redacted]".to_owned(),
        }
    );
    assert_eq!(
        redact_answer_problem(
            JudgmentAnswerProblem::MissingOption {
                label: SECRET.to_owned(),
            },
            &secrets,
        ),
        JudgmentAnswerProblem::MissingOption {
            label: "[redacted]".to_owned(),
        }
    );
}

#[test]
fn leaves_answer_problems_without_provider_labels_unchanged() {
    let secrets = [SECRET.to_owned()];
    let problems = [
        JudgmentAnswerProblem::Missing,
        JudgmentAnswerProblem::KindMismatch {
            expected: JudgmentQuestionKind::Choice,
            actual: JudgmentQuestionKind::Score,
        },
        JudgmentAnswerProblem::ProbabilityOutOfRange { value: 1.1 },
        JudgmentAnswerProblem::ConfidenceOutOfRange { value: -0.1 },
        JudgmentAnswerProblem::UnknownLevel { index: 3 },
        JudgmentAnswerProblem::MissingLevel { index: 2 },
        JudgmentAnswerProblem::ExpectedOutOfRange { value: 4.0 },
        JudgmentAnswerProblem::DistributionSum { sum: 0.5 },
        JudgmentAnswerProblem::SelectedNotMaximal {
            selected: "billing".to_owned(),
            maximal: SECRET.to_owned(),
        },
        JudgmentAnswerProblem::ExpectedInconsistent {
            expected: 0.0,
            weighted: 1.0,
        },
    ];

    for problem in problems {
        assert_eq!(redact_answer_problem(problem.clone(), &secrets), problem);
    }
}
