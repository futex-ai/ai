//! Ordered fallback content events and nested attempt resets.

use ai_interface::{ModelAttempt, ModelAttemptError, ModelError, ModelEvent};

use crate::MultiModel;

use super::fixtures::{Observed, collect, model, stream, success, text};

#[tokio::test]
async fn pre_delta_fallback_records_the_failed_attempt() {
    let model = MultiModel::new(vec![
        model(vec![Err(provider_error("first"))]),
        model(vec![Ok(success("done"))]),
    ]);
    let (result, events) = collect(&model).await;
    result.expect("fallback should succeed");

    assert_eq!(
        recorded_events(events),
        vec![fallback("first"), assistant_delta("done")]
    );
}

#[tokio::test]
async fn post_delta_fallback_emits_restart_before_the_next_lane() {
    let model = MultiModel::new(vec![
        model(vec![Ok(stream(vec![
            Ok(Some(text("partial"))),
            Ok(Some(ModelEvent::ReasoningDelta {
                part_index: 1,
                delta: "thought".to_owned(),
            })),
            Err(provider_error("first")),
        ]))]),
        model(vec![Ok(success("done"))]),
    ]);
    let (response, events) = collect(&model).await;
    let response = response.expect("fallback should succeed");

    assert_eq!(
        recorded_events(events),
        vec![
            assistant_delta("partial"),
            reasoning_delta("thought"),
            fallback("first"),
            assistant_delta("done"),
        ]
    );
    assert_eq!(response.assistant_message, "done");
}

#[tokio::test]
async fn final_lane_failure_does_not_emit_a_restart() {
    let model = MultiModel::new(vec![model(vec![Ok(stream(vec![
        Ok(Some(text("partial"))),
        Err(provider_error("only")),
    ]))])]);
    let (result, events) = collect(&model).await;
    result.expect_err("only lane should fail");

    assert_eq!(recorded_events(events), vec![assistant_delta("partial")]);
}

#[tokio::test]
async fn nested_restart_resets_outer_public_text_tracking() {
    let model = MultiModel::new(vec![
        model(vec![Ok(stream(vec![
            Ok(Some(text("discard"))),
            Ok(Some(ModelEvent::Fallback {
                error: provider_error("inner"),
            })),
            Err(provider_error("nested")),
        ]))]),
        model(vec![Ok(success("done"))]),
    ]);
    let (result, events) = collect(&model).await;
    result.expect("outer fallback should succeed");

    assert_eq!(
        recorded_events(events),
        vec![
            assistant_delta("discard"),
            fallback("inner"),
            fallback("nested"),
            assistant_delta("done"),
        ]
    );
}

fn recorded_events(events: Vec<Observed>) -> Vec<Observed> {
    events
        .into_iter()
        .filter(|event| !matches!(event, Observed::Usage(_) | Observed::Finish(..)))
        .collect()
}

fn assistant_delta(delta: &str) -> Observed {
    Observed::Text(0, delta.to_owned())
}

fn reasoning_delta(delta: &str) -> Observed {
    Observed::Reasoning(1, delta.to_owned())
}

fn fallback(model_id: &str) -> Observed {
    Observed::Fallback(ModelAttempt {
        error: ModelAttemptError::Provider {
            provider: "mock".to_owned(),
            model_id: model_id.to_owned(),
            message: "failed".to_owned(),
        },
    })
}

fn provider_error(model_id: &str) -> ModelError {
    ModelError::provider("mock", model_id, "failed")
}
