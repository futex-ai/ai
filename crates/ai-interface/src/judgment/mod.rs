//! Provider-agnostic judgment contracts.

mod answer;
mod answer_problem;
mod content;
mod error;
mod error_core;
mod input;
mod model;
mod question;
mod request;
mod response;
mod support;

pub use answer::{JudgmentAnswer, deserialize_score_probabilities};
pub use answer_problem::JudgmentAnswerProblem;
pub use content::{
    JudgmentContent, JudgmentDescription, JudgmentJson, JudgmentJsonType, JudgmentPart,
};
pub use error::{JudgmentError, JudgmentQuestionProblem, JudgmentResult};
pub use input::JudgmentInput;
pub use model::{DynJudgmentModel, JudgmentModel};
pub use question::{
    JudgmentConditionCriteria, JudgmentLevel, JudgmentOption, JudgmentQuestion,
    JudgmentQuestionKind,
};
pub use request::JudgmentRequest;
pub use response::{
    EXPECTED_SCORE_TOLERANCE, JudgmentResponse, PROBABILITY_SUM_TOLERANCE,
    SELECTED_PROBABILITY_TOLERANCE,
};
pub use support::{
    JudgmentConversationPartKind, JudgmentInputFeature, JudgmentInputForm, JudgmentPartKind,
    JudgmentSupport,
};

#[cfg(any(test, doctest, feature = "test-support"))]
pub use model::JudgmentModelMock;
