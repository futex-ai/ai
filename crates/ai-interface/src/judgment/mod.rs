//! Provider-agnostic judgment contracts.

mod answer;
mod content;
mod error;
mod model;
mod question;
mod request;
mod response;

pub use answer::JudgmentAnswer;
pub use content::{JudgmentContent, JudgmentJsonType};
pub use error::{JudgmentError, JudgmentQuestionProblem, JudgmentResult};
pub use model::{DynJudgmentModel, JudgmentModel};
pub use question::{JudgmentConditionCriteria, JudgmentQuestion};
pub use request::JudgmentRequest;
pub use response::JudgmentResponse;

#[cfg(any(test, doctest, feature = "test-support"))]
pub use model::JudgmentModelMock;
