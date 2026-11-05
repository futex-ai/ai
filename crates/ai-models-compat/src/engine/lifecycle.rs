//! Optional deferred submission-and-retrieval boundary for a dialect.

use super::request_types::ChatCompletionsRequest;
use ai_interface::ModelResult;
use async_trait::async_trait;
use serde_json::Value;
use std::{sync::Arc, time::Duration};

pub(crate) type DynDeferredCompletion = Arc<dyn DeferredCompletion>;

#[async_trait]
pub(crate) trait DeferredCompletion: Send + Sync {
    async fn complete(
        &self,
        request: ChatCompletionsRequest,
        total_timeout: Duration,
    ) -> ModelResult<Value>;
}
