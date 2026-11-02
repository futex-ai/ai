//! MiniMax HTTP-success provider-code tests.

use ai_interface::ErrorKind;

use ai_interface::{Model, ModelError};
use json_http::JsonHttpResponse;
use serde_json::json;

use crate::minimax::MINIMAX_M3;

use super::{
    MiniMaxModel,
    support::{recording_http_client, simple_request},
};

#[tokio::test]
async fn classifies_documented_base_response_codes() {
    for code in [1002, 1041, 2045, 2056] {
        assert_code_matches(code, |error| {
            matches!(error.core().kind, ErrorKind::RateLimited)
        })
        .await;
    }
    for code in [1000, 1001, 1013, 1024, 1033] {
        assert_code_matches(code, |error| {
            matches!(error.core().kind, ErrorKind::Transient)
        })
        .await;
    }
    assert_code_matches(1039, |error| {
        matches!(error.core().kind, ErrorKind::ContextLimitExceeded)
    })
    .await;
    for (code, expected) in [
        (1004, ErrorKind::RejectedKey),
        (1008, ErrorKind::RejectedKey),
        (1026, ErrorKind::InvalidRequest),
        (1027, ErrorKind::InvalidOutput),
        (2013, ErrorKind::InvalidRequest),
        (2049, ErrorKind::RejectedKey),
        (9999, ErrorKind::InvalidRequest),
        (2022, ErrorKind::UnknownModel),
    ] {
        assert_code_matches(
            code,
            |error| matches!(error.core().kind, kind if kind == expected),
        )
        .await;
    }
}

#[tokio::test]
async fn accepts_missing_or_zero_base_response() {
    for base_resp in [None, Some(json!({"status_code": 0, "status_msg": "ok"}))] {
        let mut body = success_body();
        if let Some(base_resp) = base_resp {
            body["base_resp"] = base_resp;
        }
        let (http_client, _) = recording_http_client([JsonHttpResponse {
            headers: Default::default(),
            status: 200,
            body,
        }]);
        MiniMaxModel::new(http_client, MINIMAX_M3, "minimax-key")
            .complete(&simple_request())
            .await
            .expect("successful base response should parse");
    }
}

async fn assert_code_matches(code: i64, predicate: impl FnOnce(&ModelError) -> bool) {
    let (http_client, _) = recording_http_client([JsonHttpResponse {
        headers: Default::default(),
        status: 200,
        body: json!({
            "base_resp": {
                "status_code": code,
                "status_msg": "provider detail"
            }
        }),
    }]);
    let error = MiniMaxModel::new(http_client, MINIMAX_M3, "minimax-key")
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
