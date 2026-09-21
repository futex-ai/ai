//! Judgment model trait and dynamic alias.

use std::sync::Arc;

use async_trait::async_trait;

use super::{JudgmentRequest, JudgmentResponse, JudgmentResult};

#[cfg_attr(
    any(test, doctest, feature = "test-support"),
    unimock::unimock(api = JudgmentModelMock)
)]
#[async_trait]
/// Provider-agnostic boundary for evaluating typed questions against shared state.
pub trait JudgmentModel: Send + Sync {
    /// Evaluates every question in one request.
    async fn judge(&self, request: &JudgmentRequest) -> JudgmentResult<JudgmentResponse>;
}

/// Shared dynamic judgment model alias.
pub type DynJudgmentModel = Arc<dyn JudgmentModel>;
