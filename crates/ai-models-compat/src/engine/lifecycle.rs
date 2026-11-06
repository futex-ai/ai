//! Optional deferred submission-and-retrieval boundary for a dialect.

use super::request_types::ChatCompletionsRequest;
use ai_interface::ModelResult;
use ai_models_core::ErrorContext;
use async_trait::async_trait;
use json_http::JsonHttpResponse;
use serde_json::Value;
use std::{sync::Arc, time::Duration};

pub(crate) type DynDeferredCompletion = Arc<dyn DeferredCompletion>;

#[async_trait]
pub(crate) trait DeferredCompletion: Send + Sync {
    async fn complete(
        &self,
        context: &ErrorContext,
        request: ChatCompletionsRequest,
        total_timeout: Duration,
    ) -> ModelResult<JsonHttpResponse<Value>>;
}
