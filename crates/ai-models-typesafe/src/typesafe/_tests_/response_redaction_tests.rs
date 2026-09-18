//! TypeSafe answer-problem redaction tests.

use ai_interface::{JudgmentAnswerProblem, JudgmentQuestionKind};

use super::redact_answer_problem;

#[test]
fn redacts_unknown_and_missing_option_labels() {
    let secrets = ["typesafe-secret-key".to_owned()];

    assert_eq!(
        redact_answer_problem(
            JudgmentAnswerProblem::UnknownOption {
                label: "unexpected typesafe-secret-key".to_owned(),
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
                label: "typesafe-secret-key".to_owned(),
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
    let secrets = ["typesafe-secret-key".to_owned()];
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
            maximal: "typesafe-secret-key".to_owned(),
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
