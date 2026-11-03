//! Bounded parallel calls with ordered, synchronous result publication.

use std::{collections::VecDeque, task::Poll, time::Instant};

use ai_interface::{
    ConversationMessage, DynLogger, ToolActivityPhase, ToolCall, ToolCallLogEntry,
    ToolCallLogResult, ToolResult,
};
use futures::{StreamExt, future::BoxFuture, poll, stream::FuturesUnordered};

use crate::runtime::RuntimeConfig;
use crate::tool_output::envelope_json;
use crate::{AgentEvent, Error, Result, ToolExecutionRecord, TurnCheckpoint};

use super::{
    failures::{tool_error_log_result, tool_error_message},
    observer::{log_tool_activity, log_tool_call},
    pending::PendingMessages,
    tool_dispatch::dispatch_tool_call,
};

struct Completion {
    index: usize,
    result: Result<ToolResult<ToolExecutionRecord>>,
}

type CallFuture = BoxFuture<'static, Completion>;

pub(super) struct ToolBatch {
    config: RuntimeConfig,
    logger: DynLogger,
    calls: Vec<ToolCall>,
    next_start: usize,
    next_append: usize,
    pending: PendingMessages,
    running: FuturesUnordered<CallFuture>,
    ready: VecDeque<Completion>,
    started_at: Vec<Option<Instant>>,
    checkpoint_due: bool,
}

impl ToolBatch {
    pub(super) fn new(config: RuntimeConfig, logger: DynLogger, calls: Vec<ToolCall>) -> Self {
        let count = calls.len();
        Self {
            config,
            logger,
            pending: PendingMessages::new(count),
            calls,
            next_start: 0,
            next_append: 0,
            running: FuturesUnordered::new(),
            ready: VecDeque::new(),
            started_at: vec![None; count],
            checkpoint_due: false,
        }
    }

    pub(super) async fn next_event(
        &mut self,
        conversation: &mut Vec<ConversationMessage>,
    ) -> Result<Option<AgentEvent>> {
        if let Some(completion) = self.ready.pop_front() {
            return self.finish(completion, conversation).map(Some);
        }
        if self.next_start < self.calls.len() && self.running.len() < self.config.tool_call_limit {
            let index = self.next_start;
            self.next_start += 1;
            let call = self.calls[index].clone();
            log_tool_activity(
                &self.logger,
                &self.config,
                &call,
                ToolActivityPhase::Started,
            );
            self.started_at[index] = Some(Instant::now());
            let mut future = Box::pin(execute(
                self.config.clone(),
                call.clone(),
                self.pending.clone(),
                index,
            ));
            match poll!(&mut future) {
                Poll::Ready(completion) => self.ready.push_back(completion),
                Poll::Pending => self.running.push(future),
            }
            self.flush(conversation, false);
            return Ok(Some(AgentEvent::ToolStarted {
                call_id: call.id,
                tool_name: call.name,
            }));
        }
        match self.running.next().await {
            Some(completion) => self.finish(completion, conversation).map(Some),
            None => Ok(None),
        }
    }

    pub(super) fn take_checkpoint_due(&mut self) -> bool {
        std::mem::take(&mut self.checkpoint_due)
    }

    pub(super) async fn drain(
        &mut self,
        conversation: &mut Vec<ConversationMessage>,
        mut checkpoint: Option<&mut dyn TurnCheckpoint>,
    ) {
        while let Some(completion) = self.ready.pop_front() {
            let _ = self.finish(completion, conversation);
            if let Some(checkpoint) = &mut checkpoint {
                let _ = checkpoint.check();
            }
        }
        while let Some(completion) = self.running.next().await {
            let _ = self.finish(completion, conversation);
            if let Some(checkpoint) = &mut checkpoint {
                let _ = checkpoint.check();
            }
        }
        self.flush(conversation, true);
    }

    pub(super) fn flush(
        &mut self,
        conversation: &mut Vec<ConversationMessage>,
        stopped: bool,
    ) -> usize {
        self.pending
            .flush(conversation, &mut self.next_append, stopped);
        if stopped {
            let remaining = self.pending.take_remaining_logs();
            let completed = remaining.len();
            for (index, result, completed_at) in remaining {
                self.observe(index, result, completed_at);
            }
            return completed;
        }
        0
    }

    fn observe(&self, index: usize, result: ToolCallLogResult, completed_at: Instant) {
        if let Some(call) = self.calls.get(index) {
            let latency_ms = self
                .started_at
                .get(index)
                .and_then(|started| *started)
                .map(|started| completed_at.saturating_duration_since(started).as_millis())
                .unwrap_or(0);
            log_tool_call(
                &self.logger,
                &ToolCallLogEntry {
                    call: call.clone(),
                    tool_group: self.config.tools.groups_by_name.get(&call.name).cloned(),
                    result,
                    latency_ms,
                },
            );
            log_tool_activity(
                &self.logger,
                &self.config,
                call,
                ToolActivityPhase::Completed,
            );
        }
    }

    fn finish(
        &mut self,
        completion: Completion,
        conversation: &mut Vec<ConversationMessage>,
    ) -> Result<AgentEvent> {
        self.checkpoint_due = true;
        self.flush(conversation, false);
        if let Some((result, completed_at)) = self.pending.take_log_result(completion.index) {
            self.observe(completion.index, result, completed_at);
        } else if let Err(error) = &completion.result {
            self.observe(
                completion.index,
                ToolCallLogResult::Error {
                    message: error.to_string(),
                    debug: format!("{error:?}"),
                },
                Instant::now(),
            );
        }
        let call_id = self.calls[completion.index].id.clone();
        Ok(AgentEvent::ToolFinished {
            call_id,
            result: completion.result?,
        })
    }
}

async fn execute(
    config: RuntimeConfig,
    call: ToolCall,
    pending: PendingMessages,
    index: usize,
) -> Completion {
    let result = prepare_completion(config, call, pending, index).await;
    Completion { index, result }
}

async fn prepare_completion(
    config: RuntimeConfig,
    call: ToolCall,
    pending: PendingMessages,
    index: usize,
) -> Result<ToolResult<ToolExecutionRecord>> {
    let result = match dispatch_tool_call(config, call.clone(), pending.clone(), index).await {
        Ok(record) => Ok(record),
        Err(Error::Tool(error)) => Err(error),
        Err(error) => return Err(error),
    };
    let (message, log_result) = match &result {
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
    pending.publish(index, message, log_result, true);
    Ok(result)
}
