//! MiniMax HTTP-success provider-code tests.

use ai_interface::{Model, ModelError};
use json_http::JsonHttpResponse;
use serde_json::json;

use super::{
    MiniMaxModel,
    support::{recording_http_client, simple_request},
};

#[tokio::test]
async fn classifies_documented_base_response_codes() {
    for code in [1002, 1041, 2045, 2056] {
        assert_code_matches(code, |error| {
            matches!(error, ModelError::RateLimited { .. })
        })
        .await;
    }
    for code in [1000, 1001, 1013, 1024, 1033] {
        assert_code_matches(code, |error| {
            matches!(error, ModelError::TransientProvider { .. })
        })
        .await;
    }
    assert_code_matches(1039, |error| {
        matches!(error, ModelError::ContextLimitExceeded { .. })
    })
    .await;
    for code in [1004, 1008, 1026, 1027, 2013, 2049, 9999] {
        assert_code_matches(code, |error| matches!(error, ModelError::Provider { .. })).await;
    }
}

#[tokio::test]
async fn accepts_missing_or_zero_base_response() {
    for base_resp in [None, Some(json!({"status_code": 0, "status_msg": "ok"}))] {
        let mut body = success_body();
        if let Some(base_resp) = base_resp {
            body["base_resp"] = base_resp;
        }
        let (http_client, _) = recording_http_client([JsonHttpResponse { status: 200, body }]);
        MiniMaxModel::new(http_client, "MiniMax-M3", "minimax-key")
            .complete(&simple_request())
            .await
            .expect("successful base response should parse");
    }
}

async fn assert_code_matches(code: i64, predicate: impl FnOnce(&ModelError) -> bool) {
    let (http_client, _) = recording_http_client([JsonHttpResponse {
        status: 200,
        body: json!({
            "base_resp": {
                "status_code": code,
                "status_msg": "provider detail"
            }
        }),
    }]);
    let error = MiniMaxModel::new(http_client, "MiniMax-M3", "minimax-key")
        .complete(&simple_request())
        .await
        .expect_err("non-zero base response should fail");

    assert!(
        predicate(&error),
        "unexpected error for code {code}: {error}"
    );
    assert!(error.to_string().contains(&code.to_string()));
    assert!(error.to_string().contains("provider detail"));
}

fn success_body() -> serde_json::Value {
    json!({
        "choices": [{
            "finish_reason": "stop",
            "message": {"content": "Done"}
        }]
    })
}
