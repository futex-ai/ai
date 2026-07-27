//! MiniMax transport and HTTP error tests.

use std::{collections::BTreeMap, sync::Arc};

use ai_interface::{Model, ModelError};
use async_trait::async_trait;
use json_http::{
    JsonHttpAuth, JsonHttpClient, JsonHttpRequest, JsonHttpResponse, JsonHttpTransportMock,
    StaticHeaderAuth, TransportBackedJsonHttpClient,
};
use serde_json::json;
use unimock::{MockFn, Unimock, matching};

use super::{MiniMaxModel, support::simple_request};

#[tokio::test]
async fn classifies_http_and_transport_failures() {
    let http_error = model_with_transport_answer(|_| {
        Ok(JsonHttpResponse {
            status: 429,
            body: json!({"error": {"message": "slow down"}}),
        })
    })
    .complete(&simple_request())
    .await
    .expect_err("HTTP rate limit should fail");
    assert!(matches!(http_error, ModelError::RateLimited { .. }));

    let transport_error =
        model_with_transport_answer(|_| Err(json_http::Error::transport("connection reset")))
            .complete(&simple_request())
            .await
            .expect_err("transport error should fail");
    assert!(matches!(
        transport_error,
        ModelError::TransientProvider { .. }
    ));
}

#[tokio::test]
async fn classifies_auth_and_response_shape_failures() {
    let http_client = Arc::new(TransportBackedJsonHttpClient::new(Arc::new(Unimock::new(
        (),
    ))));
    let auth_error = MiniMaxModel::with_auth(http_client, "MiniMax-M3", Arc::new(FailingAuth))
        .complete(&simple_request())
        .await
        .expect_err("auth hook error should fail");
    assert!(matches!(auth_error, ModelError::TransientProvider { .. }));

    let malformed_error = model_with_transport_answer(|_| {
        Ok(JsonHttpResponse {
            status: 200,
            body: json!({"choices": "not-an-array"}),
        })
    })
    .complete(&simple_request())
    .await
    .expect_err("malformed typed response should fail");
    assert!(matches!(malformed_error, ModelError::Internal { .. }));
}

fn model_with_transport_answer(
    answer: impl Fn(&JsonHttpRequest) -> json_http::Result<JsonHttpResponse<serde_json::Value>>
    + Send
    + Sync
    + 'static,
) -> MiniMaxModel {
    MiniMaxModel::with_auth(
        transport_client(answer),
        "MiniMax-M3",
        Arc::new(StaticHeaderAuth::bearer_token("minimax-key")),
    )
}

fn transport_client(
    answer: impl Fn(&JsonHttpRequest) -> json_http::Result<JsonHttpResponse<serde_json::Value>>
    + Send
    + Sync
    + 'static,
) -> Arc<dyn JsonHttpClient> {
    let answer = Arc::new(answer);
    let transport = Arc::new(Unimock::new(
        JsonHttpTransportMock::execute
            .each_call(matching!(_))
            .answers_arc(Arc::new(move |_, request: &JsonHttpRequest| {
                answer(request)
            })),
    ));
    Arc::new(TransportBackedJsonHttpClient::new(transport))
}

struct FailingAuth;

#[async_trait]
impl JsonHttpAuth for FailingAuth {
    async fn apply_headers(
        &self,
        _headers: &mut BTreeMap<String, String>,
    ) -> json_http::Result<()> {
        Err(json_http::Error::auth("credential unavailable"))
    }
}
