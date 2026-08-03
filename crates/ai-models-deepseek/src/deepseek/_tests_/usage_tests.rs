//! DeepSeek usage normalization tests.

use ai_interface::{Model, ModelUsage};
use json_http::JsonHttpResponse;
use serde_json::json;

use super::{
    DeepSeekModel,
    test_support::{recording_http_client, simple_request},
};

#[tokio::test]
async fn separates_cache_miss_hit_visible_and_reasoning_buckets() {
    let usage = complete_with_usage(json!({
        "prompt_tokens": 100,
        "prompt_cache_hit_tokens": 20,
        "prompt_cache_miss_tokens": 80,
        "completion_tokens": 50,
        "completion_tokens_details": {"reasoning_tokens": 30},
        "total_tokens": 150
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
async fn falls_back_to_prompt_subtraction_when_cache_miss_is_absent() {
    let usage = complete_with_usage(json!({
        "prompt_tokens": 100,
        "prompt_cache_hit_tokens": 20,
        "completion_tokens": 10
    }))
    .await;

    assert_eq!(usage.input_tokens, 80);
    assert_eq!(usage.cached_input_tokens, 20);
    assert_eq!(usage.output_tokens, 10);
    assert_eq!(usage.reasoning_tokens, 0);
    assert_eq!(usage.total_tokens, 110);
}

#[tokio::test]
async fn preserves_provider_total_and_reconstructs_missing_total_safely() {
    let provider_total = complete_with_usage(json!({
        "prompt_tokens": 10,
        "prompt_cache_hit_tokens": 2,
        "prompt_cache_miss_tokens": 8,
        "completion_tokens": 4,
        "completion_tokens_details": {"reasoning_tokens": 1},
        "total_tokens": 999
    }))
    .await;
    assert_eq!(provider_total.total_tokens, 999);

    let saturated = complete_with_usage(json!({
        "prompt_tokens": 10,
        "prompt_cache_hit_tokens": 20,
        "completion_tokens": 5,
        "completion_tokens_details": {"reasoning_tokens": 30}
    }))
    .await;
    assert_eq!(saturated.input_tokens, 0);
    assert_eq!(saturated.output_tokens, 0);
    assert_eq!(saturated.cached_input_tokens, 20);
    assert_eq!(saturated.reasoning_tokens, 30);
    assert_eq!(saturated.total_tokens, 50);
}

#[tokio::test]
async fn missing_usage_maps_to_unpriced_zeroes() {
    let (http_client, _) = recording_http_client(JsonHttpResponse {
        status: 200,
        body: response_body(None),
    });
    let usage = DeepSeekModel::new(http_client, "deepseek-key")
        .complete(&simple_request())
        .await
        .expect("response without usage should parse")
        .usage;

    assert_eq!(usage, ModelUsage::default());
}

async fn complete_with_usage(usage: serde_json::Value) -> ModelUsage {
    let (http_client, _) = recording_http_client(JsonHttpResponse {
        status: 200,
        body: response_body(Some(usage)),
    });
    DeepSeekModel::new(http_client, "deepseek-key")
        .complete(&simple_request())
        .await
        .expect("usage response should parse")
        .usage
}

fn response_body(usage: Option<serde_json::Value>) -> serde_json::Value {
    let mut body = json!({
        "choices": [{
            "finish_reason": "stop",
            "message": {"content": "Done"}
        }]
    });
    if let Some(usage) = usage {
        body["usage"] = usage;
    }
    body
}
