//! Credential-free MCP tool-adapter runtime smoke test.

use std::sync::{
    Arc, Mutex,
    atomic::{AtomicUsize, Ordering},
};

use ai_interface::{
    ConversationMessage, FinishReason, Model, ModelRequest, ModelResponse, ModelResult, ModelUsage,
    NoopLogger, Tool, ToolCall,
};
use ai_mcp::{
    McpClient, McpContentBlock, McpServerConfig, McpServerHandshake, McpToolCallOutcome,
    McpToolDescriptor, McpToolSet, Result,
};
use ai_tool_calling::{NoopTurnCheckpoint, RunOutcome, ToolCallingRuntime};
use async_trait::async_trait;
use serde_json::{Value, json};

#[tokio::test]
async fn runtime_advertises_and_dispatches_mcp_tools() {
    let called_names = Arc::new(Mutex::new(Vec::new()));
    let client = Arc::new(FixtureMcpClient {
        called_names: called_names.clone(),
    });
    let config = McpServerConfig::new("demo", "https://example.com/mcp");
    let tool_set = McpToolSet::new(client, &config, vec![descriptor()]).unwrap();
    let tool: Arc<dyn Tool> = Arc::new(tool_set);
    let runtime = ToolCallingRuntime::new(
        "Use the tool.",
        Arc::new(FixtureModel::default()),
        Arc::new(NoopLogger),
        vec![tool],
    )
    .unwrap();

    assert_eq!(runtime.tool_definitions()[0].name, "mcp__demo__echo");
    let mut turn = runtime.send(ConversationMessage::user("echo hello"), Some(3));
    let outcome = turn
        .run_with_checkpoint(&mut NoopTurnCheckpoint)
        .await
        .unwrap();

    assert!(matches!(
        outcome,
        RunOutcome::Completed {
            assistant_message,
            steps_taken: 2
        } if assistant_message == "complete"
    ));
    assert_eq!(called_names.lock().unwrap().as_slice(), ["echo"]);
}

fn descriptor() -> McpToolDescriptor {
    McpToolDescriptor {
        name: "echo".to_owned(),
        title: None,
        description: Some("Echo input.".to_owned()),
        input_schema: json!({"type":"object"}),
        output_schema: None,
    }
}

struct FixtureMcpClient {
    called_names: Arc<Mutex<Vec<String>>>,
}

#[async_trait]
impl McpClient for FixtureMcpClient {
    async fn ensure_initialized(&self) -> Result<McpServerHandshake> {
        unreachable!()
    }

    async fn list_tools(&self) -> Result<Vec<McpToolDescriptor>> {
        Ok(vec![descriptor()])
    }

    async fn call_tool(&self, name: &str, arguments: Value) -> Result<McpToolCallOutcome> {
        self.called_names.lock().unwrap().push(name.to_owned());
        Ok(McpToolCallOutcome {
            content: vec![McpContentBlock::Text {
                text: arguments.to_string(),
                annotations: None,
                meta: None,
            }],
            structured_content: None,
            is_error: false,
        })
    }

    fn tools_list_changed(&self) -> bool {
        false
    }

    async fn close(&self) -> Result<()> {
        Ok(())
    }
}

#[derive(Default)]
struct FixtureModel {
    calls: AtomicUsize,
}

#[async_trait]
impl Model for FixtureModel {
    async fn complete(&self, request: &ModelRequest) -> ModelResult<ModelResponse> {
        let call = self.calls.fetch_add(1, Ordering::SeqCst);
        assert_eq!(request.tools[0].name, "mcp__demo__echo");
        let (assistant_message, tool_calls, finish_reason) = if call == 0 {
            (
                String::new(),
                vec![ToolCall {
                    id: "call-1".to_owned(),
                    name: "mcp__demo__echo".to_owned(),
                    input: json!({"message":"hello"}),
                    operation_id: None,
                }],
                FinishReason::ToolCalls,
            )
        } else {
            ("complete".to_owned(), Vec::new(), FinishReason::Stop)
        };
        Ok(ModelResponse {
            provider: "fixture".to_owned(),
            model_id: "fixture".to_owned(),
            catalog_model_id: None,
            thinking_level: None,
            assistant_message,
            tool_calls,
            finish_reason,
            structured_output: None,
            provider_context: Vec::new(),
            usage: ModelUsage::default(),
        })
    }
}
