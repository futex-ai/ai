//! Credential-free MCP tool-adapter runtime smoke test.

use std::sync::{
    Arc, Mutex,
    atomic::{AtomicUsize, Ordering},
};

use ai_interface::test_support::model_response_stream;
use ai_interface::{
    ConversationMessage, DynModelEventStream, FinishReason, Model, ModelRequest, ModelResponse,
    ModelResult, ModelUsage, NoopLogger, Tool, ToolCall,
};
use ai_mcp::{
    McpClient, McpContentBlock, McpServerConfig, McpServerHandshake, McpToolCallOutcome,
    McpToolDescriptor, McpToolSet, Result,
};
use ai_tool_calling::{
    Agent, AgentEvent, AgentRequest, InMemoryToolOutputStore, RunOutcome, ToolCallingRuntime,
    ToolOutputPolicy,
};
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
        Arc::new(FixtureModel::default()),
        Arc::new(NoopLogger),
        vec![tool],
        Arc::new(InMemoryToolOutputStore::new()),
        ToolOutputPolicy::default(),
    )
    .unwrap();

    assert_eq!(runtime.tool_definitions()[0].name, "mcp__demo__echo");
    let mut conversation = vec![ConversationMessage::user("echo hello")];
    let mut events = runtime.run(
        &mut conversation,
        AgentRequest {
            system_prompt: "Use the tool.".to_owned(),
            max_steps: Some(3),
            ..Default::default()
        },
    );
    let mut outcome = None;
    while let Some(event) = events.next().await.unwrap() {
        if let AgentEvent::Finished(finished) = event {
            outcome = Some(finished);
        }
    }

    assert!(matches!(
        outcome,
        Some(RunOutcome::Completed {
            assistant_message,
            steps_taken: 2,
            ..
        }) if assistant_message == "complete"
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
    async fn stream(&self, request: &ModelRequest) -> ModelResult<DynModelEventStream> {
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
        Ok(model_response_stream(ModelResponse {
            provider: "fixture".to_owned(),
            model_id: "fixture".to_owned(),
            catalog_model_id: None,
            thinking_level: None,
            parts: ConversationMessage::assistant(assistant_message, tool_calls).parts,
            finish_reason,
            structured_output: None,
            usage: ModelUsage::default(),
            attempts: Vec::new(),
        }))
    }
}
