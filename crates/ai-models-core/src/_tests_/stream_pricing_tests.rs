//! Pricing of usage events without changing any other event.

use std::sync::{Arc, atomic::AtomicUsize};

use ai_interface::{
    ErrorKind, FinishReason, Model, ModelError, test_support::model_response_stream,
};
use serde_json::json;

use crate::{ModelPricing, UsagePricingModel};

use super::wrapper_fixtures::{request, response, scripted_model, stream, text};

#[tokio::test]
async fn pricing_changes_only_usage_and_stays_terminal_after_a_read_error() {
    let mut response = response("unchanged");
    response.usage.input_tokens = 1_000_000;
    response.usage.total_tokens = 1_000_000;
    let model = UsagePricingModel::new(
        scripted_model(
            vec![
                Ok(model_response_stream(response)),
                Ok(stream(vec![
                    Ok(Some(text("partial"))),
                    Err(ModelError::interrupted("p", "m", "read failed")),
                ])),
            ],
            Arc::new(AtomicUsize::new(0)),
        ),
        ModelPricing {
            input_token_usd_micros_per_million: Some(25),
            ..Default::default()
        },
    );
    let mut stream = model.stream(&request()).await.unwrap();
    let first = stream
        .next()
        .await
        .unwrap()
        .map(|event| serde_json::to_value(event).unwrap());
    let usage = stream
        .next()
        .await
        .unwrap()
        .map(|event| serde_json::to_value(event).unwrap());

    assert_eq!(
        first,
        Some(json!({"TextDelta": {"part_index": 0, "delta": "unchanged"}}))
    );
    assert_eq!(
        usage,
        Some(json!({"Usage": {"usage": {
            "input_tokens": 1_000_000, "output_tokens": 0, "cached_input_tokens": 0,
            "reasoning_tokens": 0, "total_tokens": 1_000_000, "estimated_cost_microusd": 25,
            "cost_lines": [{"unit_kind": "input_token", "quantity": 1_000_000,
                "unit_price_usd_micros_per_million": 25, "cost_usd_micros": 25,
                "measurement_state": "measured"}]
        }}}))
    );
    let finish = stream.next().await.unwrap().unwrap();
    assert_eq!(
        serde_json::to_value(finish).unwrap(),
        json!({"Finish": {"finish_reason": FinishReason::Stop, "provider": "mock", "model_id": "mock"}})
    );
    assert!(matches!(stream.next().await, Ok(None)));
    assert!(matches!(stream.next().await, Ok(None)));

    let mut stream = model.stream(&request()).await.unwrap();
    let partial = stream.next().await.unwrap().unwrap();
    assert_eq!(
        serde_json::to_value(partial).unwrap(),
        json!({"TextDelta": {"part_index": 0, "delta": "partial"}})
    );
    assert!(
        matches!(stream.next().await, Err(ModelError::Failure { core, .. }) if core.kind == ErrorKind::Interrupted && core.message == "read failed")
    );
    assert!(matches!(stream.next().await, Ok(None)));
    assert!(matches!(stream.next().await, Ok(None)));
}
