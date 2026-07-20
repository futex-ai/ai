use std::sync::Arc;

use ai_interface::{
    FinishReason, Model, ModelMock, ModelResponse, ModelUsage, ToolCall, ToolDefinition,
    ToolInvocation, ToolMock, ToolResult,
};
use parking_lot::Mutex;
use serde_json::json;
use unimock::{MockFn, Unimock, matching};

use crate::{InMemoryToolOutputStore, StepOutcome, ToolCallingRuntime, ToolOutputPolicy, Turn};

use super::super::support::user_message;

#[tokio::test]
async fn tool_dispatch_passes_operation_id_to_tool_boundary() {
    let captured_operation_ids = Arc::new(Mutex::new(Vec::<String>::new()));
    let model: Arc<dyn Model> = Arc::new(Unimock::new(
        ModelMock::complete
            .next_call(matching!(_))
            .returns(Ok(ModelResponse {
                provider: "mock".to_owned(),
                model_id: "mock-model".to_owned(),
                catalog_model_id: None,
                thinking_level: None,
                assistant_message: "calling tool".to_owned(),
                tool_calls: vec![ToolCall {
                    id: "call-1".to_owned(),
                    name: "echo".to_owned(),
                    input: json!({ "message": "hello" }),
                    operation_id: Some("operation-1".to_owned()),
                }],
                finish_reason: FinishReason::ToolCalls,
                structured_output: None,
                provider_context: Vec::new(),
                usage: ModelUsage::default(),
            })),
    ));
    let runtime = ToolCallingRuntime::new(
        "system prompt",
        model,
        Arc::new(ai_interface::NoopLogger),
        vec![Arc::new(Unimock::new((
            ToolMock::definitions
                .next_call(matching!())
                .returns(vec![ToolDefinition {
                    name: "echo".to_owned(),
                    description: "Echo a typed message.".to_owned(),
                    input_schema: json!({ "type": "object" }),
                    activity_verb: None,
                }]),
            ToolMock::call_with_invocation
                .next_call(matching!(_))
                .answers_arc({
                    let captured_operation_ids = captured_operation_ids.clone();
                    Arc::new(move |_, invocation: ToolInvocation| -> ToolResult<_> {
                        captured_operation_ids
                            .lock()
                            .push(invocation.operation_id.clone());
                        Ok(json!({
                            "tool_name": invocation.tool_name,
                            "input": invocation.input,
                        }))
                    })
                }),
        )))],
        Arc::new(InMemoryToolOutputStore::new()),
        ToolOutputPolicy::default(),
    )
    .expect("runtime should build");

    let mut turn = runtime.send(user_message("start"), Some(1));
    let outcome = turn.step().await.expect("step should succeed");

    assert_eq!(
        captured_operation_ids.lock().as_slice(),
        &["operation-1".to_owned()]
    );
    assert!(matches!(
        outcome,
        StepOutcome::Stepped {
            tool_results,
            ..
        } if tool_results[0].operation_id == "operation-1"
    ));
}
