//! Smoke-test orchestration.

use std::future::Future;
use std::sync::Arc;
use std::task::{Context, Poll, Waker};

use ai_interface::{ConversationMessage, DynModel, NoopLogger, Tool};
use ai_models_anthropic::{AnthropicModel, CLAUDE_SONNET_4_6};
use ai_models_google::{GEMINI_2_5_PRO, GoogleModel};
use ai_models_openai::{GPT_5_5, OpenAiAudioTranscriber, OpenAiModel};
use ai_models_xai::{GROK_4_20_REASONING, XaiModel};
use ai_tool_calling::{
    InMemoryToolOutputStore, RunOutcome, ToolCallingRuntime, ToolOutputPolicy, Turn,
};
use json_http::{JsonHttpClient, ReqwestJsonHttpClient};

use crate::error::{Error, Result};

use super::mcp::{build_mcp_tool, build_oauth_mcp_client};
use super::pagination::{SmokeModelError, SmokePaginationModel, SmokeTool};

/// Runs all credential-free construction and pagination checks.
pub(crate) fn run() -> Result<()> {
    let client: Arc<dyn JsonHttpClient> = Arc::new(ReqwestJsonHttpClient::new());
    let _anthropic = AnthropicModel::new(client.clone(), CLAUDE_SONNET_4_6, "anthropic-key");
    let _google = GoogleModel::new(client.clone(), GEMINI_2_5_PRO, "google-key");
    let _openai = OpenAiModel::new(client.clone(), GPT_5_5, "openai-key");
    let _xai = XaiModel::new(client, GROK_4_20_REASONING, "xai-key");
    let _transcriber = OpenAiAudioTranscriber::new("gpt-4o-mini-transcribe", "openai-key");

    let model: DynModel = Arc::new(SmokePaginationModel::new());
    let tool: Arc<dyn Tool> = Arc::new(SmokeTool);
    let mcp_tool: Arc<dyn Tool> = Arc::new(build_mcp_tool()?);
    let _oauth_mcp_client = build_oauth_mcp_client()?;
    let runtime = match ToolCallingRuntime::new(
        "Use registered tools when helpful.",
        model,
        Arc::new(NoopLogger),
        vec![tool, mcp_tool],
        Arc::new(InMemoryToolOutputStore::new()),
        ToolOutputPolicy::default(),
    ) {
        Ok(runtime) => runtime,
        Err(source) => return Err(Error::SmokeRuntime { source }),
    };

    let outcome = match block_on(async {
        let mut turn = runtime.send(ConversationMessage::user("Run the smoke flow."), Some(5));
        turn.run().await
    }) {
        Ok(outcome) => outcome,
        Err(source) => return Err(Error::SmokeRuntime { source }),
    };
    match outcome {
        RunOutcome::Completed { steps_taken: 4, .. } => Ok(()),
        _ => Err(Error::SmokeRuntime {
            source: ai_tool_calling::Error::checkpoint(SmokeModelError::UnexpectedOutcome),
        }),
    }
}

fn block_on<T>(future: impl Future<Output = T>) -> T {
    let waker = Waker::noop();
    let mut context = Context::from_waker(waker);
    let mut future = std::pin::pin!(future);
    loop {
        match future.as_mut().poll(&mut context) {
            Poll::Ready(output) => return output,
            Poll::Pending => std::thread::yield_now(),
        }
    }
}
