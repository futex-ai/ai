//! Reserved output-reader names and conditional model definitions.

use std::sync::Arc;

use ai_interface::test_support::model_response_stream;
use ai_interface::{
    DynTool, FinishReason, Model, ModelMock, ModelRequest, ModelResponse, ModelUsage, NoopLogger,
    ToolDefinition, ToolMock,
};
use serde_json::json;
use unimock::{MockFn, Unimock, matching};

use crate::tests::event_support::{collect_events, request};
use crate::tests::support::{runtime_with_logger_store_and_policy, user_message};
use crate::{
    Agent, InMemoryToolOutputStore, ToolCallingRuntime, ToolOutputPolicy, ToolOutputStore,
    ToolOutputWriteRequest,
};

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
        model,
        Arc::new(NoopLogger),
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

#[tokio::test]
async fn intrinsic_reader_is_offered_with_stored_output_after_tool_replacement() {
    let store = Arc::new(InMemoryToolOutputStore::new());
    let policy = ToolOutputPolicy::new(5, 5, 100, 100).unwrap();
    store
        .write(ToolOutputWriteRequest {
            tool_name: "search".to_owned(),
            content: "abcdefghij".to_owned(),
            policy,
            first_window_length: 5,
        })
        .await
        .unwrap();
    let model: Arc<dyn Model> = Arc::new(Unimock::new(
        ModelMock::stream
            .each_call(matching!(_))
            .answers(&|_, request: &ModelRequest| {
                assert_reader_definition(request.tools.clone());
                Ok(model_response_stream(ModelResponse {
                    attempts: Vec::new(),
                    provider: "mock".to_owned(),
                    model_id: "mock".to_owned(),
                    catalog_model_id: None,
                    thinking_level: None,
                    parts: Vec::new(),
                    finish_reason: FinishReason::Stop,
                    structured_output: None,
                    usage: ModelUsage::default(),
                }))
            }),
    ));
    let runtime = runtime_with_logger_store_and_policy(
        model,
        Arc::new(NoopLogger),
        Vec::new(),
        store,
        policy,
    )
    .unwrap();
    assert!(runtime.tool_definitions().is_empty());
    let mut conversation = vec![user_message("start")];
    collect_events(runtime.run(&mut conversation, request(Some(1))))
        .await
        .unwrap();
    runtime
        .replace_tools(vec![catalog_only_tool("echo")])
        .unwrap();
    let definitions = runtime.tool_definitions();
    assert!(
        definitions
            .iter()
            .any(|definition| definition.name == "echo")
    );
    collect_events(runtime.run(&mut conversation, request(Some(1))))
        .await
        .unwrap();
}

fn catalog_only_tool(name: &str) -> DynTool {
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
