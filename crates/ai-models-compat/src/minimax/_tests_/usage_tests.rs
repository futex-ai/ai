//! MiniMax usage normalization tests.

use ai_interface::{Model, ModelUsage};
use json_http::JsonHttpResponse;
use serde_json::json;

use super::{
    MiniMaxModel,
    support::{recording_http_client, simple_request},
};

#[tokio::test]
async fn separates_cached_and_reasoning_usage_buckets() {
    let usage = complete_with_usage(json!({
        "prompt_tokens": 100,
        "completion_tokens": 50,
        "total_tokens": 150,
        "prompt_tokens_details": {"cached_tokens": 20},
        "completion_tokens_details": {"reasoning_tokens": 30}
    }))
    .await;

    assert_eq!(
        usage,
        ModelUsage {
            input_tokens: 80,
            output_tokens: 20,
            cached_input_tokens: 20,
            reasoning_tokens: 30,
            total_tokens: 150,
            estimated_cost_microusd: 0,
            cost_lines: Vec::new(),
        }
    );
}

#[tokio::test]
async fn saturates_buckets_and_reconstructs_missing_total() {
    let usage = complete_with_usage(json!({
        "prompt_tokens": 10,
        "completion_tokens": 5,
        "prompt_tokens_details": {"cached_tokens": 20},
        "completion_tokens_details": {"reasoning_tokens": 30}
    }))
    .await;

    assert_eq!(usage.input_tokens, 0);
    assert_eq!(usage.output_tokens, 0);
    assert_eq!(usage.cached_input_tokens, 20);
    assert_eq!(usage.reasoning_tokens, 30);
    assert_eq!(usage.total_tokens, 50);
}

async fn complete_with_usage(usage: serde_json::Value) -> ModelUsage {
    let (http_client, _) = recording_http_client([JsonHttpResponse {
        status: 200,
        body: json!({
            "choices": [{
                "finish_reason": "stop",
                "message": {"content": "Done"}
            }],
            "usage": usage
        }),
    }]);
    MiniMaxModel::new(http_client, "MiniMax-M3", "minimax-key")
        .complete(&simple_request())
        .await
        .expect("usage response should parse")
        .usage
}
