//! Persistent state retained while any next-event future is suspended.

use std::time::Instant;

use ai_interface::{
    ConversationMessage, DynLogger, DynModel, DynModelEventStream, ModelRequest, ModelResponseFold,
    ModelResult,
};
use futures::future::BoxFuture;

use crate::runtime::RuntimeConfig;
use crate::{
    AgentRequest, Error, ModelResponseCheckpoint, Result, RunOutcome, ToolCallingRuntime,
    TurnCheckpoint,
};

use super::super::batch::ToolBatch;

pub(crate) struct TurnStream<'a> {
    pub(super) model: DynModel,
    pub(super) logger: DynLogger,
    pub(super) config: RuntimeConfig,
    pub(super) conversation: &'a mut Vec<ConversationMessage>,
    pub(super) request: AgentRequest,
    pub(super) checkpoint: Option<&'a mut dyn TurnCheckpoint>,
    pub(super) response_checkpoint: Option<&'a mut dyn ModelResponseCheckpoint>,
    pub(super) stage: Stage,
    pub(super) opening: Option<BoxFuture<'static, ModelResult<DynModelEventStream>>>,
    pub(super) model_stream: Option<DynModelEventStream>,
    pub(super) fold: Option<ModelResponseFold>,
    pub(super) model_request: Option<ModelRequest>,
    pub(super) model_log_pending: bool,
    pub(super) started_at: Instant,
    pub(super) tools: Option<ToolBatch>,
    pub(super) steps_taken: usize,
    pub(super) step_start: usize,
    pub(super) assistant_message: String,
    pub(super) outcome: Option<RunOutcome>,
    pub(super) stopped_error: Option<Error>,
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub(super) enum Stage {
    Start,
    OpenModel,
    Model,
    ToolsStart,
    Tools,
    StepEnd,
    Finish,
    Drain,
    Done,
}

impl<'a> TurnStream<'a> {
    pub(crate) fn new(
        runtime: &ToolCallingRuntime,
        conversation: &'a mut Vec<ConversationMessage>,
        request: AgentRequest,
        checkpoint: Option<&'a mut dyn TurnCheckpoint>,
        response_checkpoint: Option<&'a mut dyn ModelResponseCheckpoint>,
    ) -> Self {
        Self {
            model: runtime.model.clone(),
            logger: runtime.logger.clone(),
            config: runtime.snapshot(),
            conversation,
            request,
            checkpoint,
            response_checkpoint,
            stage: Stage::Start,
            opening: None,
            model_stream: None,
            fold: None,
            model_request: None,
            model_log_pending: false,
            started_at: Instant::now(),
            tools: None,
            steps_taken: 0,
            step_start: 0,
            assistant_message: String::new(),
            outcome: None,
            stopped_error: None,
        }
    }

    pub(super) fn check(&mut self) -> Result<()> {
        match &mut self.checkpoint {
            Some(checkpoint) => checkpoint.check(),
            None => Ok(()),
        }
    }
}
