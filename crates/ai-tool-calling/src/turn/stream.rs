//! Pull-driven turn state and synchronous conversation appends.

use std::{future::Future, pin::Pin, time::Instant};

use ai_interface::{
    ConversationMessage, DynLogger, DynModel, DynModelEventStream, ModelCallLogEntry,
    ModelCallLogResult, ModelRequest, ModelResponse, ModelResponseFold, ToolActivityPhase,
    ToolCall, ToolCallLogEntry, ToolCallLogResult,
};
use async_trait::async_trait;

use crate::intrinsic::tool_output_read_definition;
use crate::runtime::RuntimeConfig;
use crate::tool_output::envelope_json;
use crate::{
    AgentEvent, AgentEventStream, AgentRequest, Error, ModelResponseCheckpoint, Result, RunOutcome,
    ToolCallingRuntime, ToolExecutionRecord, TurnCheckpoint,
};

use super::{
    event_copy::copy_for_fold,
    failures::{tool_error_log_result, tool_error_message},
    observer::{log_tool_activity, log_turn_outcome},
    response::{response_outcome, validate_response},
    tool_dispatch::dispatch_tool_call,
};

type ToolFuture = Pin<Box<dyn Future<Output = Result<ToolExecutionRecord>> + Send>>;

pub(crate) struct TurnStream<'a> {
    model: DynModel,
    logger: DynLogger,
    config: RuntimeConfig,
    conversation: &'a mut Vec<ConversationMessage>,
    request: AgentRequest,
    checkpoint: Option<&'a mut dyn TurnCheckpoint>,
    response_checkpoint: Option<&'a mut dyn ModelResponseCheckpoint>,
    stage: Stage,
    model_stream: Option<DynModelEventStream>,
    fold: Option<ModelResponseFold>,
    model_request: Option<ModelRequest>,
    started_at: Instant,
    tool_future: Option<ToolFuture>,
    calls: Vec<ToolCall>,
    call_index: usize,
    steps_taken: usize,
    step_start: usize,
    assistant_message: String,
    outcome: Option<RunOutcome>,
}

#[derive(Clone, Copy)]
enum Stage {
    Start,
    Model,
    ToolsStart,
    Tools,
    StepEnd,
    Finish,
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
            model_stream: None,
            fold: None,
            model_request: None,
            started_at: Instant::now(),
            tool_future: None,
            calls: Vec::new(),
            call_index: 0,
            steps_taken: 0,
            step_start: 0,
            assistant_message: String::new(),
            outcome: None,
        }
    }

    fn check(&mut self) -> Result<()> {
        match &mut self.checkpoint {
            Some(checkpoint) => checkpoint.check(),
            None => Ok(()),
        }
    }

    async fn advance(&mut self) -> Result<Option<AgentEvent>> {
        loop {
            match self.stage {
                Stage::Done => return Ok(None),
                Stage::Start => {
                    if let Some(max_steps) = self
                        .request
                        .max_steps
                        .filter(|max| self.steps_taken >= *max)
                    {
                        self.outcome = Some(RunOutcome::Capped {
                            assistant_message: self.assistant_message.clone(),
                            steps_taken: self.steps_taken,
                            max_steps,
                        });
                        self.stage = Stage::Finish;
                        continue;
                    }
                    self.check()?;
                    self.step_start = self.conversation.len();
                    let mut tools = self.config.tools.definitions.clone();
                    tools.push(tool_output_read_definition());
                    self.model_request = Some(ModelRequest {
                        system_prompt: self.request.system_prompt.clone(),
                        messages: self.conversation.clone(),
                        tools,
                        controls: self.request.controls.clone(),
                        response_schema: None,
                    });
                    self.started_at = Instant::now();
                    if let Some(request) = &self.model_request {
                        self.model_stream = Some(self.model.stream(request).await?);
                    }
                    self.fold = Some(ModelResponseFold::new());
                    self.stage = Stage::Model;
                }
                Stage::Model => {
                    let event = match &mut self.model_stream {
                        Some(stream) => stream.next().await?,
                        None => return Ok(None),
                    };
                    if let Some(event) = event {
                        if let Some(fold) = &mut self.fold {
                            fold.push(copy_for_fold(&event)?)?;
                        }
                        return Ok(Some(AgentEvent::Model(event)));
                    }
                    self.model_stream = None;
                    if let Some(fold) = self.fold.take() {
                        let response = fold.finish()?;
                        return self.response(response).map(Some);
                    }
                }
                Stage::ToolsStart => {
                    self.check()?;
                    self.stage = Stage::Tools;
                }
                Stage::Tools => {
                    if self.call_index >= self.calls.len() {
                        self.stage = Stage::StepEnd;
                        continue;
                    }
                    let call = self.calls[self.call_index].clone();
                    if self.tool_future.is_none() {
                        log_tool_activity(
                            &self.logger,
                            &self.config,
                            &call,
                            ToolActivityPhase::Started,
                        )?;
                        self.started_at = Instant::now();
                        self.tool_future = Some(Box::pin(dispatch_tool_call(
                            self.config.clone(),
                            call.clone(),
                        )));
                        return Ok(Some(AgentEvent::ToolStarted {
                            call_id: call.id,
                            tool_name: call.name,
                        }));
                    }
                    let result = match &mut self.tool_future {
                        Some(future) => future.await,
                        None => continue,
                    };
                    self.tool_future = None;
                    let result = match result {
                        Ok(record) => Ok(record),
                        Err(Error::Tool(error)) => Err(error),
                        Err(error) => return Err(error),
                    };
                    let (message, logged_result) = match &result {
                        Ok(record) => (
                            ConversationMessage::tool(
                                envelope_json(&record.model_visible_output, &call.name)?,
                                call.name.clone(),
                                call.id.clone(),
                            ),
                            ToolCallLogResult::Success {
                                output: record.model_visible_output.clone(),
                            },
                        ),
                        Err(error) => (
                            tool_error_message(&call, error),
                            tool_error_log_result(error),
                        ),
                    };
                    self.conversation.push(message);
                    self.logger.log_tool_call(&ToolCallLogEntry {
                        call: call.clone(),
                        tool_group: self.config.tools.groups_by_name.get(&call.name).cloned(),
                        result: logged_result,
                        latency_ms: self.started_at.elapsed().as_millis(),
                    })?;
                    log_tool_activity(
                        &self.logger,
                        &self.config,
                        &call,
                        ToolActivityPhase::Completed,
                    )?;
                    self.call_index += 1;
                    self.check()?;
                    return Ok(Some(AgentEvent::ToolFinished {
                        call_id: call.id,
                        result,
                    }));
                }
                Stage::StepEnd => {
                    self.stage = if self.outcome.is_some() {
                        Stage::Finish
                    } else {
                        Stage::Start
                    };
                    return Ok(Some(AgentEvent::StepFinished {
                        messages: self.conversation[self.step_start..].to_vec(),
                    }));
                }
                Stage::Finish => {
                    self.stage = Stage::Done;
                    if let Some(outcome) = self.outcome.take() {
                        log_turn_outcome(&self.logger, &outcome, self.request.max_steps)?;
                        return Ok(Some(AgentEvent::Finished(outcome)));
                    }
                }
            }
        }
    }

    fn response(&mut self, mut response: ModelResponse) -> Result<AgentEvent> {
        self.steps_taken += 1;
        if let Some(request) = &self.model_request {
            self.logger.log_model_call(&ModelCallLogEntry {
                request: request.log_copy(),
                result: ModelCallLogResult::Success {
                    response: Box::new(response.log_copy()),
                },
                latency_ms: self.started_at.elapsed().as_millis(),
            })?;
        }
        validate_response(&response)?;
        self.check()?;
        if let Some(checkpoint) = &mut self.response_checkpoint {
            checkpoint.checkpoint_response(&mut response)?;
        }
        validate_response(&response)?;
        let text = response.text();
        if !text.trim().is_empty() {
            self.assistant_message = text;
        }
        if !response.parts.is_empty() {
            self.conversation.push(response.to_assistant_message());
        }
        self.calls = response.tool_calls();
        self.call_index = 0;
        self.outcome = response_outcome(&response, &self.assistant_message, self.steps_taken);
        self.stage = if self.outcome.is_some() {
            Stage::StepEnd
        } else {
            Stage::ToolsStart
        };
        Ok(AgentEvent::Response(response))
    }
}

#[async_trait]
impl AgentEventStream for TurnStream<'_> {
    async fn next(&mut self) -> Result<Option<AgentEvent>> {
        match self.advance().await {
            Ok(event) => Ok(event),
            Err(error) => {
                self.stage = Stage::Done;
                self.tool_future = None;
                self.model_stream = None;
                Err(error)
            }
        }
    }
}
