use std::sync::Arc;

use ai_interface::{
    ConversationRole, FinishReason, Logger, LoggerMock, Model, ModelMock, ModelRequest,
    ModelResponse, ModelUsage, ToolActivityLogEntry, ToolActivityPhase, ToolCall, ToolCallLogEntry,
    ToolCallLogResult, ToolDefinition, ToolMock, ToolOutputEnvelope,
};
use serde_json::json;
use unimock::{MockFn, Unimock, matching};

use crate::{
    InMemoryToolOutputStore, StepOutcome, ToolCallingRuntime, ToolOutputPolicy, ToolOutputStore,
    ToolOutputWriteRequest, Turn,
};

use super::support::{runtime, runtime_with_logger_store_and_policy, user_message};

#[test]
fn reserved_reader_name_is_rejected_for_injected_tools() {
    let tool = Arc::new(Unimock::new(
        ToolMock::definitions
            .next_call(matching!())
            .returns(vec![ToolDefinition {
                name: "tool_output_read".to_owned(),
                description: "Not allowed.".to_owned(),
                input_schema: json!({ "type": "object" }),
                activity_verb: None,
            }]),
    ));
    let model: Arc<dyn Model> = Arc::new(Unimock::new(()));
    let error = match ToolCallingRuntime::new(
        "system prompt",
        model,
        Arc::new(ai_interface::NoopLogger),
        vec![tool],
        Arc::new(InMemoryToolOutputStore::new()),
        ToolOutputPolicy::default(),
    ) {
        Ok(_) => panic!("reserved tool name should fail construction"),
        Err(error) => error,
    };

    assert!(
        matches!(error, crate::Error::ReservedToolDefinition { name } if name == "tool_output_read")
    );
}

#[test]
fn intrinsic_reader_definition_is_always_present_after_tool_replacement() {
    let model: Arc<dyn Model> = Arc::new(Unimock::new(()));
    let runtime = runtime(model, Vec::new()).expect("runtime should build");

    assert_reader_definition(runtime.tool_definitions());
    runtime
        .replace_tools(vec![catalog_only_tool("echo")])
        .expect("replacement should accept normal tools");
    let definitions = runtime.tool_definitions();

    assert!(
        definitions
            .iter()
            .any(|definition| definition.name == "echo")
    );
    assert_reader_definition(definitions);
}

fn catalog_only_tool(name: &str) -> ai_interface::DynTool {
    Arc::new(Unimock::new(
        ToolMock::definitions
            .next_call(matching!())
            .returns(vec![ToolDefinition {
                name: name.to_owned(),
                description: "Catalog-only test tool.".to_owned(),
                input_schema: json!({ "type": "object" }),
                activity_verb: None,
            }]),
    ))
}

#[tokio::test]
async fn intrinsic_reader_reads_requested_id_without_recursive_wrapping() {
    let store = Arc::new(InMemoryToolOutputStore::new());
    let policy = ToolOutputPolicy::new(5, 5, 100, 100).unwrap();
    let written = store
        .write(ToolOutputWriteRequest {
            tool_name: "search".to_owned(),
            content: "abcdefghij".to_owned(),
            policy,
            first_window_length: 5,
        })
        .await
        .unwrap();
    let output_id = written.output_id.clone();
    let model: Arc<dyn Model> = Arc::new(Unimock::new(
        ModelMock::complete
            .next_call(matching!(_))
            .returns(Ok(tool_call_response(ToolCall {
                id: "read-call".to_owned(),
                name: "tool_output_read".to_owned(),
                input: json!({ "output_id": output_id.clone(), "offset": 5 }),
                operation_id: Some("read-operation".to_owned()),
            }))),
    ));
    let logger: Arc<dyn Logger> = Arc::new(Unimock::new((
        LoggerMock::log_model_call
            .next_call(matching!(_))
            .returns(Ok(())),
        LoggerMock::log_tool_activity
            .next_call(matching!(_))
            .answers(&|_, entry: &ToolActivityLogEntry| {
                assert_eq!(entry.tool_name, "tool_output_read");
                assert_eq!(entry.phase, ToolActivityPhase::Started);
                Ok(())
            }),
        LoggerMock::log_tool_call.next_call(matching!(_)).answers(
            &|_, entry: &ToolCallLogEntry| {
                assert!(matches!(
                    &entry.result,
                    ToolCallLogResult::Success {
                        output: ToolOutputEnvelope::Window(window)
                    } if window.output_id().is_some()
                ));
                Ok(())
            },
        ),
        LoggerMock::log_tool_activity
            .next_call(matching!(_))
            .answers(&|_, entry: &ToolActivityLogEntry| {
                assert_eq!(entry.tool_name, "tool_output_read");
                assert_eq!(entry.phase, ToolActivityPhase::Completed);
                Ok(())
            }),
    )));
    let runtime = runtime_with_logger_store_and_policy(model, logger, Vec::new(), store, policy)
        .expect("runtime should build");

    let mut turn = runtime.send(user_message("start"), Some(1));
    let outcome = turn.step().await.expect("read should succeed");

    assert!(matches!(
        outcome,
        StepOutcome::Stepped {
            tool_results,
            ..
        } if tool_results[0].output_id.as_ref() == Some(&output_id)
            && tool_results[0].operation_id == "read-operation"
            && tool_results[0].raw_output
                == serde_json::to_value(&tool_results[0].model_visible_output).unwrap()
    ));
}

#[tokio::test]
async fn replaced_output_store_makes_previous_ids_unavailable_and_run_recovers() {
    let old_store = Arc::new(InMemoryToolOutputStore::new());
    let policy = ToolOutputPolicy::new(5, 5, 100, 100).unwrap();
    let written = old_store
        .write(ToolOutputWriteRequest {
            tool_name: "search".to_owned(),
            content: "abcdefghij".to_owned(),
            policy,
            first_window_length: 5,
        })
        .await
        .unwrap();
    let output_id = written.output_id.clone();
    let model: Arc<dyn Model> = Arc::new(Unimock::new((
        ModelMock::complete
            .next_call(matching!(_))
            .returns(Ok(tool_call_response(ToolCall {
                id: "read-call".to_owned(),
                name: "tool_output_read".to_owned(),
                input: json!({ "output_id": output_id }),
                operation_id: None,
            }))),
        ModelMock::complete
            .next_call(matching!(_))
            .answers(&|_, request: &ModelRequest| {
                let content = request
                    .messages
                    .iter()
                    .find(|message| message.role == ConversationRole::Tool)
                    .expect("tool error should be retained")
                    .content
                    .clone();
                assert!(content.contains("\"ok\":false"));
                assert!(content.contains("output is no longer available"));
                assert!(content.contains("the original tool call itself succeeded"));
                assert!(content.contains("confirm with the user"));
                assert!(!content.contains("original tool call failed"));
                Ok(stop_response("recovered"))
            }),
    )));
    let runtime = runtime_with_logger_store_and_policy(
        model,
        Arc::new(ai_interface::NoopLogger),
        Vec::new(),
        old_store,
        policy,
    )
    .expect("runtime should build");
    runtime.replace_output_store(Arc::new(InMemoryToolOutputStore::new()));

    let mut turn = runtime.send(user_message("start"), Some(3));
    let outcome = turn.run().await.expect("run should recover");

    assert!(matches!(
        outcome,
        crate::RunOutcome::Completed {
            assistant_message,
            steps_taken: 2,
        } if assistant_message == "recovered"
    ));
}

fn assert_reader_definition(definitions: Vec<ToolDefinition>) {
    let definition = definitions
        .iter()
        .find(|definition| definition.name == "tool_output_read")
        .expect("intrinsic reader should be exposed");
    assert!(
        definition
            .description
            .contains("only when the task requires")
    );
    assert!(
        definition
            .description
            .contains("narrowing the original query")
    );
    assert!(definition.description.contains("bytes, not tokens"));
    assert_eq!(definition.input_schema["required"], json!(["output_id"]));
}

fn tool_call_response(call: ToolCall) -> ModelResponse {
    ModelResponse {
        provider: "mock".to_owned(),
        model_id: "mock-model".to_owned(),
        catalog_model_id: None,
        thinking_level: None,
        assistant_message: "calling tool".to_owned(),
        tool_calls: vec![call],
        finish_reason: FinishReason::ToolCalls,
        structured_output: None,
        provider_context: Vec::new(),
        usage: ModelUsage::default(),
    }
}

fn stop_response(message: &str) -> ModelResponse {
    ModelResponse {
        provider: "mock".to_owned(),
        model_id: "mock-model".to_owned(),
        catalog_model_id: None,
        thinking_level: None,
        assistant_message: message.to_owned(),
        tool_calls: Vec::new(),
        finish_reason: FinishReason::Stop,
        structured_output: None,
        provider_context: Vec::new(),
        usage: ModelUsage::default(),
    }
}
