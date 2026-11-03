//! Bounded parallel calls with ordered, synchronous result publication.

use std::{collections::VecDeque, task::Poll, time::Instant};

use ai_interface::{
    ConversationMessage, DynLogger, ToolActivityPhase, ToolCall, ToolCallLogEntry,
    ToolCallLogResult, ToolResult,
};
use futures::{StreamExt, future::BoxFuture, poll, stream::FuturesUnordered};

use crate::runtime::RuntimeConfig;
use crate::tool_output::envelope_json;
use crate::{AgentEvent, Error, Result, ToolExecutionRecord};

use super::{
    failures::{tool_error_log_result, tool_error_message},
    observer::log_tool_activity,
    pending::PendingMessages,
    tool_dispatch::dispatch_tool_call,
};

struct Completion {
    call: ToolCall,
    result: ToolResult<ToolExecutionRecord>,
    latency_ms: u128,
}

type CallFuture = BoxFuture<'static, Result<Completion>>;

pub(super) struct ToolBatch {
    config: RuntimeConfig,
    logger: DynLogger,
    calls: Vec<ToolCall>,
    next_start: usize,
    next_append: usize,
    pending: PendingMessages,
    running: FuturesUnordered<CallFuture>,
    ready: VecDeque<Result<Completion>>,
}

impl ToolBatch {
    pub(super) fn new(config: RuntimeConfig, logger: DynLogger, calls: Vec<ToolCall>) -> Self {
        Self {
            config,
            logger,
            pending: PendingMessages::new(calls.len()),
            calls,
            next_start: 0,
            next_append: 0,
            running: FuturesUnordered::new(),
            ready: VecDeque::new(),
        }
    }

    pub(super) async fn next_event(
        &mut self,
        conversation: &mut Vec<ConversationMessage>,
    ) -> Result<Option<AgentEvent>> {
        if let Some(completion) = self.ready.pop_front() {
            return self.finish(completion?, conversation).map(Some);
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
            )?;
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
            Some(completion) => self.finish(completion?, conversation).map(Some),
            None => Ok(None),
        }
    }

    pub(super) async fn drain(&mut self, conversation: &mut Vec<ConversationMessage>) {
        while let Some(completion) = self.ready.pop_front() {
            if let Ok(completion) = completion {
                let _ = self.finish(completion, conversation);
            }
        }
        while let Some(completion) = self.running.next().await {
            if let Ok(completion) = completion {
                let _ = self.finish(completion, conversation);
            }
        }
        self.flush(conversation, true);
    }

    pub(super) fn flush(&mut self, conversation: &mut Vec<ConversationMessage>, stopped: bool) {
        self.pending
            .flush(conversation, &mut self.next_append, stopped);
    }

    fn finish(
        &mut self,
        completion: Completion,
        conversation: &mut Vec<ConversationMessage>,
    ) -> Result<AgentEvent> {
        self.flush(conversation, false);
        let logged_result = match &completion.result {
            Ok(record) => ToolCallLogResult::Success {
                output: record.model_visible_output.clone(),
            },
            Err(error) => tool_error_log_result(error),
        };
        self.logger.log_tool_call(&ToolCallLogEntry {
            call: completion.call.clone(),
            tool_group: self
                .config
                .tools
                .groups_by_name
                .get(&completion.call.name)
                .cloned(),
            result: logged_result,
            latency_ms: completion.latency_ms,
        })?;
        log_tool_activity(
            &self.logger,
            &self.config,
            &completion.call,
            ToolActivityPhase::Completed,
        )?;
        Ok(AgentEvent::ToolFinished {
            call_id: completion.call.id,
            result: completion.result,
        })
    }
}

async fn execute(
    config: RuntimeConfig,
    call: ToolCall,
    pending: PendingMessages,
    index: usize,
) -> Result<Completion> {
    let started_at = Instant::now();
    let result = match dispatch_tool_call(config, call.clone(), pending.clone(), index).await {
        Ok(record) => Ok(record),
        Err(Error::Tool(error)) => Err(error),
        Err(error) => return Err(error),
    };
    let message = match &result {
        Ok(record) => ConversationMessage::tool(
            envelope_json(&record.model_visible_output, &call.name)?,
            call.name.clone(),
            call.id.clone(),
        ),
        Err(error) => tool_error_message(&call, error),
    };
    pending.publish(index, message, true);
    Ok(Completion {
        call,
        result,
        latency_ms: started_at.elapsed().as_millis(),
    })
}
