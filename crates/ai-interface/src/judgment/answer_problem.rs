//! Typed answer postcondition problems.

use std::fmt;

use super::JudgmentQuestionKind;

/// Typed postcondition failure for one judgment answer.
#[derive(Clone, Debug, PartialEq)]
pub enum JudgmentAnswerProblem {
    /// The response omitted the requested answer.
    Missing,
    /// The answer shape did not match its question.
    KindMismatch {
        /// Shape required by the question.
        expected: JudgmentQuestionKind,
        /// Shape returned by the provider.
        actual: JudgmentQuestionKind,
    },
    /// A probability was non-finite or outside zero to one.
    ProbabilityOutOfRange {
        /// Invalid probability.
        value: f64,
    },
    /// A confidence was non-finite or outside zero to one.
    ConfidenceOutOfRange {
        /// Invalid confidence.
        value: f64,
    },
    /// A choice answer named an option absent from the question.
    UnknownOption {
        /// Unrecognized option label.
        label: String,
    },
    /// A choice answer omitted a requested option probability.
    MissingOption {
        /// Requested option without a probability.
        label: String,
    },
    /// A score answer named an index absent from the question.
    UnknownLevel {
        /// Unrecognized level index.
        index: u32,
    },
    /// A score answer omitted a requested level probability.
    MissingLevel {
        /// Requested level without a probability.
        index: u32,
    },
    /// A score expectation was non-finite or outside the level range.
    ExpectedOutOfRange {
        /// Invalid expected score.
        value: f64,
    },
    /// A choice or score distribution did not sum close enough to one.
    DistributionSum {
        /// Observed probability sum.
        sum: f64,
    },
    /// A choice answer selected an option that is not the most probable one.
    SelectedNotMaximal {
        /// Requested option label the provider selected.
        selected: String,
        /// Requested option label with the highest reported probability.
        maximal: String,
    },
    /// A score expectation disagrees with its probability-weighted level.
    ExpectedInconsistent {
        /// Expected score reported by the provider.
        expected: f64,
        /// Probability-weighted level computed from the reported distribution.
        weighted: f64,
    },
}

impl fmt::Display for JudgmentAnswerProblem {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Missing => formatter.write_str("answer is missing"),
            Self::KindMismatch { expected, actual } => {
                write!(
                    formatter,
                    "answer kind mismatch: expected {expected:?}, got {actual:?}"
                )
            }
            Self::ProbabilityOutOfRange { value } => write!(
                formatter,
                "probability must be finite and between zero and one, got {value}"
            ),
            Self::ConfidenceOutOfRange { value } => write!(
                formatter,
                "confidence must be finite and between zero and one, got {value}"
            ),
            Self::UnknownOption { label } => {
                write!(formatter, "option `{label}` was not requested")
            }
            Self::MissingOption { label } => {
                write!(formatter, "probability for option `{label}` is missing")
            }
            Self::UnknownLevel { index } => {
                write!(formatter, "level index `{index}` was not requested")
            }
            Self::MissingLevel { index } => {
                write!(
                    formatter,
                    "probability for level index `{index}` is missing"
                )
            }
            Self::ExpectedOutOfRange { value } => write!(
                formatter,
                "expected score must be finite and within the requested levels, got {value}"
            ),
            Self::DistributionSum { sum } => {
                write!(
                    formatter,
                    "probability distribution must sum to one, got {sum}"
                )
            }
            Self::SelectedNotMaximal { selected, maximal } => write!(
                formatter,
                "selected option `{selected}` is less probable than option `{maximal}`"
            ),
            Self::ExpectedInconsistent { expected, weighted } => write!(
                formatter,
                "expected score {expected} disagrees with the probability-weighted level {weighted}"
            ),
        }
    }
}

#[cfg(test)]
#[path = "_tests_/answer_problem_tests.rs"]
mod answer_problem_tests;
