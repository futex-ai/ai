//! Model requests, response checkpoints, and cancellation-safe error draining.

use std::time::Instant;

use ai_interface::{ModelRequest, ModelResponse};
use async_trait::async_trait;

use crate::intrinsic::tool_output_read_definition;
use crate::{AgentEvent, AgentEventStream, Error, Result};

use super::{
    super::{
        batch::ToolBatch,
        observer::{log_model_error, log_model_success},
        response::{response_outcome, validate_response},
    },
    state::{Stage, TurnStream},
};

impl TurnStream<'_> {
    pub(super) fn start_model(&mut self) {
        self.step_start = self.conversation.len();
        let mut tools = self.config.tools.definitions.clone();
        if self.config.output_store.has_outputs() {
            tools.push(tool_output_read_definition());
        }
        let request = ModelRequest {
            system_prompt: self.request.system_prompt.clone(),
            messages: self.conversation.clone(),
            tools,
            controls: self.request.controls.clone(),
            response_schema: self.request.final_answer_schema.clone(),
        };
        self.model_request = Some(request.clone());
        self.model_log_pending = true;
        self.started_at = Instant::now();
        let model = self.model.clone();
        self.opening = Some(Box::pin(async move { model.stream(&request).await }));
        self.stage = Stage::OpenModel;
    }

    pub(super) fn response(&mut self, mut response: ModelResponse) -> Result<AgentEvent> {
        self.steps_taken += 1;
        self.model_log_pending = false;
        if let Some(request) = &self.model_request {
            log_model_success(
                &self.logger,
                request,
                &response,
                self.started_at.elapsed().as_millis(),
            );
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
        self.tools = Some(ToolBatch::new(
            self.config.clone(),
            self.logger.clone(),
            response.tool_calls(),
        ));
        self.outcome = response_outcome(&response, &self.assistant_message, self.steps_taken);
        self.stage = if self.outcome.is_some() {
            Stage::StepEnd
        } else {
            Stage::ToolsStart
        };
        Ok(AgentEvent::Response(response))
    }

    fn record_error(&mut self, error: Error) {
        if self.model_log_pending {
            self.model_log_pending = false;
            if let Some(request) = &self.model_request {
                log_model_error(
                    &self.logger,
                    request,
                    &error,
                    self.started_at.elapsed().as_millis(),
                );
            }
        }
        self.stopped_error = Some(error);
        self.stage = Stage::Drain;
    }

    async fn finish_error(&mut self) -> Result<Option<AgentEvent>> {
        self.opening = None;
        self.model_stream = None;
        if let Some(tools) = &mut self.tools {
            match &mut self.checkpoint {
                Some(checkpoint) => {
                    tools
                        .drain(self.conversation, Some(&mut **checkpoint))
                        .await
                }
                None => tools.drain(self.conversation, None).await,
            }
        }
        self.tools = None;
        self.stage = Stage::Done;
        match self.stopped_error.take() {
            Some(error) => Err(error),
            None => Ok(None),
        }
    }
}

#[async_trait]
impl AgentEventStream for TurnStream<'_> {
    async fn next(&mut self) -> Result<Option<AgentEvent>> {
        if self.stage == Stage::Drain {
            return self.finish_error().await;
        }
        match self.advance().await {
            Ok(event) => Ok(event),
            Err(error) => {
                self.record_error(error);
                self.finish_error().await
            }
        }
    }
}

impl Drop for TurnStream<'_> {
    fn drop(&mut self) {
        let completed = match &mut self.tools {
            Some(tools) => tools.flush(self.conversation, true),
            None => 0,
        };
        if let Some(checkpoint) = &mut self.checkpoint {
            for _ in 0..completed {
                let _ = checkpoint.check();
            }
        }
    }
}
