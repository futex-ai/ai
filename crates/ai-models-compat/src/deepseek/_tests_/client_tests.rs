//! DeepSeek transport and local-codec error tests.

use ai_interface::ErrorKind;

use std::sync::Arc;

use ai_interface::{Model, ModelError};
use json_http::{JsonHttpAuth, JsonHttpAuthMock};
use serde_json::Value;
use unimock::{MockFn, Unimock, matching};

use crate::deepseek::{DEEPSEEK_V4_PRO, DeepSeekModel};

use super::{
    client::request_error,
    test_support::{simple_request, transport_failure_http_client, unused_http_client},
};

#[tokio::test]
async fn transport_and_auth_failures_are_transient_without_credentials() {
    let transport_error = DeepSeekModel::new(transport_failure_http_client("offline"), "secret")
        .complete(&simple_request())
        .await
        .expect_err("transport failure should return an error");
    assert!(matches!(transport_error.core().kind, ErrorKind::Transient));
    assert!(!transport_error.to_string().contains("secret"));

    let auth: Arc<dyn JsonHttpAuth> = Arc::new(Unimock::new(
        JsonHttpAuthMock::apply_headers
            .next_call(matching!(_))
            .returns(Err(json_http::Error::auth("auth hook rejected"))),
    ));
    let auth_error = DeepSeekModel::with_auth(unused_http_client(), auth)
        .complete(&simple_request())
        .await
        .expect_err("auth failure should return an error");
    assert!(matches!(auth_error.core().kind, ErrorKind::Transient));
}

#[test]
fn local_codec_failures_remain_internal() {
    let source = serde_json::from_str::<Value>("{").expect_err("invalid JSON");
    let serialization = request_error(
        json_http::Error::SerializeRequest { source },
        DEEPSEEK_V4_PRO.provider_model_id.as_ref(),
    );
    let source = serde_json::from_str::<Value>("{").expect_err("invalid JSON");
    let deserialization = request_error(
        json_http::Error::DeserializeResponse {
            body: Value::Null,
            source,
        },
        DEEPSEEK_V4_PRO.provider_model_id.as_ref(),
    );

    assert!(
        matches!(serialization, ModelError::Internal { ref core, .. } if core.kind == ErrorKind::Internal)
    );
    assert!(
        matches!(deserialization, ModelError::Internal { ref core, .. } if core.kind == ErrorKind::InvalidOutput)
    );
}
