//! Typed tools derive schema, reject bad arguments, and serialize output.

use std::sync::{Arc, Mutex};

use ai_interface::{Tool, ToolError, ToolInvocation, ToolResult};
use ai_tool_calling::{TypedTool, TypedToolAdapter};
use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::json;
use unimock::{MockFn, Unimock, matching};

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
/// Typed arguments exposed to the generated test mock.
pub struct Arguments {
    message: String,
    repeat: u16,
}

#[derive(Serialize)]
/// Typed output exposed to the generated test mock.
pub struct Output {
    message: String,
    count: u16,
}

#[unimock::unimock(api = BehaviorMock)]
trait Behavior: Send + Sync {
    fn execute(&self, args: Arguments, operation_id: &str) -> ToolResult<Output>;
}

struct RepeatTool {
    behavior: Unimock,
}

#[async_trait]
impl TypedTool for RepeatTool {
    type Args = Arguments;
    type Output = Output;

    fn name(&self) -> &str {
        "repeat"
    }
    fn description(&self) -> &str {
        "Repeat a message."
    }
    fn activity_verb(&self) -> Option<&str> {
        Some("Repeating")
    }
    fn group(&self) -> Option<&'static str> {
        Some("text")
    }

    async fn execute(&self, args: Arguments, operation_id: &str) -> ToolResult<Output> {
        self.behavior.execute(args, operation_id)
    }
}

#[test]
fn the_input_schema_comes_from_the_rust_argument_type() {
    let tool = TypedToolAdapter::new(RepeatTool {
        behavior: Unimock::new(()),
    });
    let definitions = tool.definitions();
    assert_eq!(definitions.len(), 1);
    assert_eq!(definitions[0].name, "repeat");
    assert_eq!(definitions[0].input_schema["type"], "object");
    assert_eq!(
        definitions[0].input_schema["properties"]["message"]["type"],
        "string"
    );
    assert_eq!(
        definitions[0].input_schema["properties"]["repeat"]["type"],
        "integer"
    );
    assert_eq!(definitions[0].input_schema["additionalProperties"], false);
    let required = definitions[0].input_schema["required"].as_array().unwrap();
    assert!(required.contains(&json!("message")));
    assert!(required.contains(&json!("repeat")));
    assert_eq!(definitions[0].activity_verb.as_deref(), Some("Repeating"));
    assert_eq!(tool.group_for_tool("repeat"), Some("text"));
}

#[tokio::test]
async fn undecodable_arguments_fail_before_the_tool_runs() {
    let tool = TypedToolAdapter::new(RepeatTool {
        behavior: Unimock::new(()),
    });
    for input in [
        json!({"message":"hi"}),
        json!({"message":"hi","repeat":"private-input"}),
        json!({"message":"hi","repeat":1,"unknown":true}),
    ] {
        let result = tool
            .call_with_invocation(ToolInvocation {
                tool_name: "repeat".to_owned(),
                input,
                operation_id: "operation".to_owned(),
            })
            .await;
        assert!(
            matches!(result, Err(ToolError::InvalidArguments { .. })),
            "bad arguments reached execution: {result:?}"
        );
        assert!(!format!("{result:?}").contains("private-input"));
    }
}

#[tokio::test]
async fn decoded_arguments_and_operation_id_reach_the_tool_and_output_becomes_json() {
    let invocations = Arc::new(Mutex::new(Vec::new()));
    let behavior = Unimock::new(
        BehaviorMock::execute
            .some_call(matching!(_, _))
            .answers_arc({
                let invocations = invocations.clone();
                Arc::new(move |_, args: Arguments, operation_id: &str| {
                    invocations.lock().unwrap().push((
                        args.message,
                        args.repeat,
                        operation_id.to_owned(),
                    ));
                    Ok(Output {
                        message: "hello hello".to_owned(),
                        count: 2,
                    })
                })
            }),
    );
    let tool = TypedToolAdapter::new(RepeatTool { behavior });
    let output = tool
        .call_with_invocation(ToolInvocation {
            tool_name: "repeat".to_owned(),
            input: json!({"message":"hello","repeat":2}),
            operation_id: "operation-7".to_owned(),
        })
        .await
        .unwrap();
    assert_eq!(output, json!({"message":"hello hello","count":2}));
    assert_eq!(
        *invocations.lock().unwrap(),
        [("hello".to_owned(), 2, "operation-7".to_owned())]
    );
}
