//! Provider-agnostic judgment contracts.

mod answer;
mod answer_problem;
mod content;
mod error;
mod model;
mod question;
mod request;
mod response;

pub use answer::JudgmentAnswer;
pub use answer_problem::JudgmentAnswerProblem;
pub use content::{JudgmentContent, JudgmentJsonType};
pub use error::{JudgmentError, JudgmentQuestionProblem, JudgmentResult};
pub use model::{DynJudgmentModel, JudgmentModel};
pub use question::{JudgmentConditionCriteria, JudgmentQuestion, JudgmentQuestionKind};
pub use request::JudgmentRequest;
pub use response::{JudgmentResponse, PROBABILITY_SUM_TOLERANCE};

#[cfg(any(test, doctest, feature = "test-support"))]
pub use model::JudgmentModelMock;
