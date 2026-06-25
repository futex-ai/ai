//! Concrete turn state machine for the in-memory tool-calling runtime.

use ai_interface::{ConversationMessage, FinishReason, ModelRequest, TurnOutcomeLogEntry};
use async_trait::async_trait;

use crate::{Error, Result, ToolCallingRuntime};

use super::{
    failures::provider_error,
    response::{complete_model_request, validate_response_contract},
    tool_dispatch::dispatch_tool_call,
    types::{
        ModelResponseCheckpoint, NoopModelResponseCheckpoint, NoopTurnCheckpoint, RunOutcome,
        StepOutcome, ToolExecutionRecord, Turn, TurnCheckpoint,
    },
};

/// Default concrete turn handle.
pub struct ActiveTurn<'a> {
    runtime: &'a ToolCallingRuntime,
    max_steps: Option<usize>,
    steps_taken: usize,
    assistant_message: String,
    successful_tool_calls: Vec<ToolExecutionRecord>,
    terminal_outcome: Option<StepOutcome>,
}

impl<'a> ActiveTurn<'a> {
    pub(crate) fn new(runtime: &'a ToolCallingRuntime, max_steps: Option<usize>) -> Self {
        Self {
            runtime,
            max_steps,
            steps_taken: 0,
            assistant_message: String::new(),
            successful_tool_calls: Vec::new(),
            terminal_outcome: None,
        }
    }

    /// Executes one model round with caller-provided tool checkpoints.
    pub async fn step_with_checkpoint(
        &mut self,
        checkpoint: &mut dyn TurnCheckpoint,
    ) -> Result<StepOutcome> {
        let mut response_checkpoint = NoopModelResponseCheckpoint;
        self.step_with_checkpoints(checkpoint, &mut response_checkpoint)
            .await
    }

    /// Executes one model round with response and tool checkpoints.
    pub async fn step_with_checkpoints(
        &mut self,
        checkpoint: &mut dyn TurnCheckpoint,
        response_checkpoint: &mut dyn ModelResponseCheckpoint,
    ) -> Result<StepOutcome> {
        if let Some(outcome) = &self.terminal_outcome {
            return Ok(outcome.clone());
        }
        if let Some(outcome) = self.capped_outcome()? {
            return Ok(outcome);
        }

        checkpoint.check()?;
        let request = self.model_request();
        let mut response = complete_model_request(self.runtime, request).await?;
        self.steps_taken += 1;
        validate_response_contract(&response)?;
        checkpoint.check()?;
        response_checkpoint.checkpoint_response(&mut response)?;
        self.store_assistant_response(&response)?;
        if let Some(outcome) = self.terminal_response_outcome(&response)? {
            return Ok(outcome);
        }

        let tool_calls = response.tool_calls;
        let mut tool_results = Vec::new();
        let mut first_tool_error = None;
        checkpoint.check()?;
        for call in tool_calls {
            match dispatch_tool_call(self.runtime, call).await {
                Ok(record) => {
                    self.successful_tool_calls.push(record.clone());
                    tool_results.push(record);
                }
                Err(Error::Tool(error)) => {
                    if first_tool_error.is_none() {
                        first_tool_error = Some(error);
                    }
                }
                Err(error) => return Err(error),
            }
            checkpoint.check()?;
        }
        if let Some(error) = first_tool_error {
            return Err(Error::Tool(error));
        }
        Ok(self.stepped(tool_results))
    }

    /// Runs with caller-provided tool checkpoints until completion or cap.
    pub async fn run_with_checkpoint(
        &mut self,
        checkpoint: &mut dyn TurnCheckpoint,
    ) -> Result<RunOutcome> {
        loop {
            match self.step_with_checkpoint(checkpoint).await {
                Ok(StepOutcome::Stepped { .. }) => continue,
                Ok(StepOutcome::Completed {
                    assistant_message,
                    steps_taken,
                }) => {
                    return Ok(RunOutcome::Completed {
                        assistant_message,
                        steps_taken,
                    });
                }
                Ok(StepOutcome::Capped {
                    assistant_message,
                    steps_taken,
                    max_steps,
                }) => {
                    return Ok(RunOutcome::Capped {
                        assistant_message,
                        steps_taken,
                        max_steps,
                    });
                }
                Err(Error::Tool(_)) => continue,
                Err(error) => return Err(error),
            }
        }
    }

    fn capped_outcome(&mut self) -> Result<Option<StepOutcome>> {
        let Some(max_steps) = self
            .max_steps
            .filter(|max_steps| self.steps_taken >= *max_steps)
        else {
            return Ok(None);
        };
        let outcome = self.store_terminal_outcome(StepOutcome::Capped {
            assistant_message: self.assistant_message.clone(),
            steps_taken: self.steps_taken,
            max_steps,
        });
        self.runtime
            .logger
            .log_turn_outcome(&self.terminal_outcome(false))?;
        Ok(Some(outcome))
    }

    fn model_request(&self) -> ModelRequest {
        let (system_prompt, messages, tools) = self.runtime.request_snapshot();
        ModelRequest {
            system_prompt,
            messages,
            tools,
            response_schema: None,
        }
    }

    fn store_assistant_response(&mut self, response: &ai_interface::ModelResponse) -> Result<()> {
        if response.finish_reason != FinishReason::ToolCalls && !response.tool_calls.is_empty() {
            return Err(provider_error(
                response,
                "model returned tool calls without a tool-call finish reason",
            ));
        }
        if !response.assistant_message.trim().is_empty() {
            self.assistant_message = response.assistant_message.clone();
        }
        if !response.assistant_message.trim().is_empty() || !response.tool_calls.is_empty() {
            self.runtime.append_message(ConversationMessage::assistant(
                response.assistant_message.clone(),
                response.tool_calls.clone(),
            ));
        }
        Ok(())
    }

    fn terminal_response_outcome(
        &mut self,
        response: &ai_interface::ModelResponse,
    ) -> Result<Option<StepOutcome>> {
        match &response.finish_reason {
            FinishReason::Stop | FinishReason::Truncated => {
                let outcome = self.store_terminal_outcome(StepOutcome::Completed {
                    assistant_message: self.assistant_message.clone(),
                    steps_taken: self.steps_taken,
                });
                self.runtime
                    .logger
                    .log_turn_outcome(&self.terminal_outcome(true))?;
                Ok(Some(outcome))
            }
            FinishReason::Filtered => Err(provider_error(
                response,
                "model response was filtered by the provider",
            )),
            FinishReason::Other(raw) => Err(provider_error(response, raw.clone())),
            FinishReason::ToolCalls if response.tool_calls.is_empty() => Err(provider_error(
                response,
                "model reported tool calls without any tool call payloads",
            )),
            FinishReason::ToolCalls => Ok(None),
        }
    }

    fn terminal_outcome(&self, completed: bool) -> TurnOutcomeLogEntry {
        TurnOutcomeLogEntry {
            assistant_message: self.assistant_message.clone(),
            steps_taken: self.steps_taken,
            completed,
            max_steps: self.max_steps,
        }
    }

    fn store_terminal_outcome(&mut self, outcome: StepOutcome) -> StepOutcome {
        self.terminal_outcome = Some(outcome.clone());
        outcome
    }

    fn stepped(&self, tool_results: Vec<ToolExecutionRecord>) -> StepOutcome {
        StepOutcome::Stepped {
            assistant_message: self.assistant_message.clone(),
            steps_taken: self.steps_taken,
            tool_results,
        }
    }
}

#[async_trait]
impl Turn for ActiveTurn<'_> {
    async fn step(&mut self) -> Result<StepOutcome> {
        let mut checkpoint = NoopTurnCheckpoint;
        self.step_with_checkpoint(&mut checkpoint).await
    }

    async fn run(&mut self) -> Result<RunOutcome> {
        let mut checkpoint = NoopTurnCheckpoint;
        self.run_with_checkpoint(&mut checkpoint).await
    }

    fn steps_taken(&self) -> usize {
        self.steps_taken
    }

    fn successful_tool_calls(&self) -> &[ToolExecutionRecord] {
        &self.successful_tool_calls
    }
}
