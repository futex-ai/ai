use std::sync::Arc;

use ai_interface::{
    DynModel, DynTool, FinishReason, ModelMock, ModelResponse, ModelUsage, ToolCall,
    ToolDefinition, ToolMock, ToolOutputEnvelope, ToolOutputRemainderUnavailableReason,
};
use serde_json::{Value, json};
use unimock::{MockFn, Unimock, matching};

use crate::{
    DynToolOutputStore, InMemoryToolOutputStore, StepOutcome, ToolOutputPolicy,
    ToolOutputStoreError, ToolOutputStoreMock, ToolOutputStoreWindow, Turn,
};

use super::super::support::{runtime_with_store_and_policy, user_message};

#[tokio::test]
async fn small_success_is_inline_and_keeps_handled_false_raw_output() {
    let store: DynToolOutputStore = Arc::new(Unimock::new(()));
    let runtime = runtime_with_store_and_policy(
        model_for_call(tool_call("call-1", "handled")),
        vec![output_tool(
            "handled",
            json!({ "ok": false, "reason": "handled" }),
        )],
        store,
        ToolOutputPolicy::default(),
    )
    .expect("runtime should build");

    let mut turn = runtime.send(user_message("start"), Some(1));
    let outcome = turn.step().await.expect("step should succeed");

    assert!(matches!(
        outcome,
        StepOutcome::Stepped {
            tool_results,
            ..
        } if tool_results[0].output_id.is_none()
            && tool_results[0].raw_output["ok"] == json!(false)
            && matches!(
                &tool_results[0].model_visible_output,
                ToolOutputEnvelope::Inline(inline) if inline.output()["ok"] == json!(false)
            )
    ));
}

#[tokio::test]
async fn exactly_inline_limit_serialized_bytes_stays_inline() {
    let store = Arc::new(InMemoryToolOutputStore::new());
    let output = Value::String("a".repeat(19_998));
    assert_eq!(serde_json::to_string(&output).unwrap().len(), 20_000);
    let runtime = runtime_with_store_and_policy(
        model_for_call(tool_call("call-1", "big")),
        vec![output_tool("big", output)],
        store.clone(),
        ToolOutputPolicy::default(),
    )
    .expect("runtime should build");

    let mut turn = runtime.send(user_message("start"), Some(1));
    let outcome = turn.step().await.expect("step should succeed");

    assert_eq!(store.reserved_bytes(), 0);
    assert!(matches!(
        outcome,
        StepOutcome::Stepped {
            tool_results,
            ..
        } if tool_results[0].output_id.is_none()
            && matches!(
                &tool_results[0].model_visible_output,
                ToolOutputEnvelope::Inline(inline) if inline.total_bytes() == 20_000
            )
    ));
}

#[tokio::test]
async fn per_output_overflow_degrades_as_success() {
    let policy = ToolOutputPolicy::new(4, 4, 5, 10).unwrap();
    let runtime = runtime_with_store_and_policy(
        model_for_call(tool_call("call-1", "big")),
        vec![output_tool("big", json!("abcd"))],
        Arc::new(InMemoryToolOutputStore::new()),
        policy,
    )
    .expect("runtime should build");

    let record = single_step_record(runtime).await;

    assert_degraded(record, ToolOutputRemainderUnavailableReason::OutputTooLarge);
}

#[tokio::test]
async fn aggregate_overflow_degrades_one_sibling_without_pinning_order() {
    let policy = ToolOutputPolicy::new(4, 4, 6, 6).unwrap();
    let runtime = runtime_with_store_and_policy(
        model_for_calls(vec![tool_call("call-1", "big"), tool_call("call-2", "big")]),
        vec![output_tool("big", json!("abcd"))],
        Arc::new(InMemoryToolOutputStore::new()),
        policy,
    )
    .expect("runtime should build");

    let mut turn = runtime.send(user_message("start"), Some(1));
    let outcome = turn.step().await.expect("step should succeed");

    let StepOutcome::Stepped { tool_results, .. } = outcome else {
        panic!("expected tool step");
    };
    assert_eq!(tool_results.len(), 2);
    assert!(tool_results.iter().any(|record| record.output_id.is_some()));
    assert!(tool_results.iter().any(|record| {
        matches!(
            &record.model_visible_output,
            ToolOutputEnvelope::Window(window)
                if window.remainder_unavailable()
                    == Some(&ToolOutputRemainderUnavailableReason::BudgetExhausted)
        )
    }));
    let tool_messages = runtime
        .conversation()
        .into_iter()
        .filter(|message| message.tool_call_id.is_some())
        .collect::<Vec<_>>();
    assert_eq!(tool_messages.len(), 2);
    assert_eq!(tool_messages[0].tool_call_id.as_deref(), Some("call-1"));
    assert_eq!(tool_messages[1].tool_call_id.as_deref(), Some("call-2"));
}

#[tokio::test]
async fn store_write_failure_degrades_as_success() {
    #[derive(Debug, thiserror::Error)]
    #[error("[ai_tool_calling/tests] write failed")]
    struct WriteFailed;

    let policy = ToolOutputPolicy::new(4, 4, 20, 20).unwrap();
    let failed_window = ToolOutputStoreWindow {
        tool_name: "big".to_owned(),
        offset: 0,
        content: "\"abc".to_owned(),
        returned_bytes: 4,
        total_bytes: 6,
        truncated: true,
        next_offset: Some(4),
    };
    let store: DynToolOutputStore = Arc::new(Unimock::new(
        ToolOutputStoreMock::write
            .next_call(matching!(_))
            .answers_arc(Arc::new(move |_, _| {
                Err(ToolOutputStoreError::write_failure(
                    failed_window.clone(),
                    WriteFailed,
                ))
            })),
    ));
    let runtime = runtime_with_store_and_policy(
        model_for_call(tool_call("call-1", "big")),
        vec![output_tool("big", json!("abcd"))],
        store,
        policy,
    )
    .expect("runtime should build");

    let record = single_step_record(runtime).await;

    assert_degraded(
        record,
        ToolOutputRemainderUnavailableReason::StoreUnavailable,
    );
}

#[tokio::test]
async fn multibyte_first_window_snaps_to_utf8_boundary() {
    let policy = ToolOutputPolicy::new(4, 4, 100, 100).unwrap();
    let runtime = runtime_with_store_and_policy(
        model_for_call(tool_call("call-1", "unicode")),
        vec![output_tool("unicode", json!("ééé"))],
        Arc::new(InMemoryToolOutputStore::new()),
        policy,
    )
    .expect("runtime should build");

    let record = single_step_record(runtime).await;

    assert!(matches!(
        record.model_visible_output,
        ToolOutputEnvelope::Window(window)
            if window.content() == "\"é"
                && window.returned_bytes() == 3
                && window.next_offset() == Some(3)
    ));
}

async fn single_step_record(runtime: crate::ToolCallingRuntime) -> crate::ToolExecutionRecord {
    let mut turn = runtime.send(user_message("start"), Some(1));
    let outcome = turn.step().await.expect("step should succeed");
    let StepOutcome::Stepped {
        mut tool_results, ..
    } = outcome
    else {
        panic!("expected tool step");
    };
    tool_results.remove(0)
}

fn assert_degraded(
    record: crate::ToolExecutionRecord,
    reason: ToolOutputRemainderUnavailableReason,
) {
    assert!(record.output_id.is_none());
    assert!(matches!(
        record.model_visible_output,
        ToolOutputEnvelope::Window(window)
            if window.output_id().is_none()
                && window.next_offset().is_none()
                && window.remainder_unavailable() == Some(&reason)
                && window.truncated()
    ));
}

fn output_tool(name: &str, output: Value) -> DynTool {
    Arc::new(Unimock::new((
        ToolMock::definitions
            .next_call(matching!())
            .returns(vec![ToolDefinition {
                name: name.to_owned(),
                description: "Return a static value.".to_owned(),
                input_schema: json!({ "type": "object" }),
                activity_verb: None,
            }]),
        ToolMock::call
            .each_call(matching!(_, _))
            .answers_arc(Arc::new(move |_, _, _| Ok(output.clone()))),
    )))
}

fn model_for_call(call: ToolCall) -> DynModel {
    model_for_calls(vec![call])
}

fn model_for_calls(tool_calls: Vec<ToolCall>) -> DynModel {
    Arc::new(Unimock::new(
        ModelMock::complete
            .next_call(matching!(_))
            .returns(Ok(ModelResponse {
                provider: "mock".to_owned(),
                model_id: "mock-model".to_owned(),
                catalog_model_id: None,
                thinking_level: None,
                assistant_message: "calling tool".to_owned(),
                tool_calls,
                finish_reason: FinishReason::ToolCalls,
                structured_output: None,
                provider_context: Vec::new(),
                usage: ModelUsage::default(),
            })),
    ))
}

fn tool_call(id: &str, name: &str) -> ToolCall {
    ToolCall {
        id: id.to_owned(),
        name: name.to_owned(),
        input: json!({}),
        operation_id: None,
    }
}
