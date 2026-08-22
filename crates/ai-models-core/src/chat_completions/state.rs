//! Internal indexed state for chat-completions delta merging.

use std::collections::BTreeMap;

use super::types::{
    ChatCompletionsChoice, ChatCompletionsMessage, ChatCompletionsResponse,
    ChatCompletionsStreamChunk, ChatCompletionsStreamError, ChatCompletionsToolCall,
    ChatCompletionsToolCallDelta, ChatCompletionsToolFunction, ChatCompletionsUsage,
};

#[derive(Debug, Default)]
pub(super) struct ChatCompletionsState {
    choices: BTreeMap<u64, ChoiceState>,
    usage: Option<ChatCompletionsUsage>,
}

impl ChatCompletionsState {
    pub(super) fn push_chunk(
        &mut self,
        chunk: ChatCompletionsStreamChunk,
    ) -> Result<(), ChatCompletionsStreamError> {
        for choice in chunk.choices {
            let state = self.choices.entry(choice.index).or_default();
            append(&mut state.content, choice.delta.content);
            append(&mut state.reasoning_content, choice.delta.reasoning_content);
            for tool_call in choice.delta.tool_calls {
                state.push_tool_call(choice.index, tool_call)?;
            }
            state.push_finish_reason(choice.index, choice.finish_reason)?;
        }
        if let Some(usage) = chunk.usage {
            self.usage = Some(usage);
        }
        Ok(())
    }

    pub(super) fn finish(self) -> Result<ChatCompletionsResponse, ChatCompletionsStreamError> {
        if self.choices.is_empty() {
            return Err(ChatCompletionsStreamError::MissingChoices);
        }
        let choices = self
            .choices
            .into_iter()
            .map(|(index, state)| state.finish(index))
            .collect::<Result<Vec<_>, _>>()?;
        let Some(usage) = self.usage else {
            return Err(ChatCompletionsStreamError::MissingUsage);
        };
        Ok(ChatCompletionsResponse { choices, usage })
    }
}

#[derive(Debug, Default)]
struct ChoiceState {
    content: Option<String>,
    reasoning_content: Option<String>,
    tool_calls: BTreeMap<u64, ToolCallState>,
    finish_reason: Option<String>,
}

impl ChoiceState {
    fn push_tool_call(
        &mut self,
        choice_index: u64,
        delta: ChatCompletionsToolCallDelta,
    ) -> Result<(), ChatCompletionsStreamError> {
        let tool_index = delta.index;
        let state = self.tool_calls.entry(tool_index).or_default();
        if let Some(id) = delta.id {
            set_tool_id(choice_index, tool_index, &mut state.id, id)?;
        }
        if let Some(name) = delta.function.name {
            set_tool_name(choice_index, tool_index, &mut state.name, name)?;
        }
        if let Some(arguments) = delta.function.arguments {
            state.arguments.push_str(&arguments);
        }
        Ok(())
    }

    fn push_finish_reason(
        &mut self,
        choice_index: u64,
        received: Option<String>,
    ) -> Result<(), ChatCompletionsStreamError> {
        let Some(received) = received else {
            return Ok(());
        };
        if let Some(existing) = self.finish_reason.as_ref()
            && existing != &received
        {
            return Err(ChatCompletionsStreamError::ConflictingFinishReason {
                choice_index,
                existing: existing.clone(),
                received,
            });
        }
        self.finish_reason = Some(received);
        Ok(())
    }

    fn finish(self, index: u64) -> Result<ChatCompletionsChoice, ChatCompletionsStreamError> {
        let Some(finish_reason) = self.finish_reason else {
            return Err(ChatCompletionsStreamError::MissingFinishReason {
                choice_index: index,
            });
        };
        let tool_calls = self
            .tool_calls
            .into_iter()
            .map(|(tool_index, state)| state.finish(index, tool_index))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(ChatCompletionsChoice {
            index,
            message: ChatCompletionsMessage {
                content: self.content,
                reasoning_content: self.reasoning_content,
                tool_calls,
            },
            finish_reason,
        })
    }
}

#[derive(Debug, Default)]
struct ToolCallState {
    id: Option<String>,
    name: Option<String>,
    arguments: String,
}

impl ToolCallState {
    fn finish(
        self,
        choice_index: u64,
        tool_index: u64,
    ) -> Result<ChatCompletionsToolCall, ChatCompletionsStreamError> {
        let Some(id) = self.id else {
            return Err(ChatCompletionsStreamError::MissingToolCallId {
                choice_index,
                tool_index,
            });
        };
        let Some(name) = self.name else {
            return Err(ChatCompletionsStreamError::MissingToolFunctionName {
                choice_index,
                tool_index,
            });
        };
        Ok(ChatCompletionsToolCall {
            id,
            function: ChatCompletionsToolFunction {
                name,
                arguments: self.arguments,
            },
        })
    }
}

fn append(target: &mut Option<String>, fragment: Option<String>) {
    if let Some(fragment) = fragment {
        target.get_or_insert_with(String::new).push_str(&fragment);
    }
}

fn set_tool_id(
    choice_index: u64,
    tool_index: u64,
    target: &mut Option<String>,
    received: String,
) -> Result<(), ChatCompletionsStreamError> {
    if let Some(existing) = target.as_ref()
        && existing != &received
    {
        return Err(ChatCompletionsStreamError::ConflictingToolCallId {
            choice_index,
            tool_index,
            existing: existing.clone(),
            received,
        });
    }
    *target = Some(received);
    Ok(())
}

fn set_tool_name(
    choice_index: u64,
    tool_index: u64,
    target: &mut Option<String>,
    received: String,
) -> Result<(), ChatCompletionsStreamError> {
    if let Some(existing) = target.as_ref()
        && existing != &received
    {
        return Err(ChatCompletionsStreamError::ConflictingToolFunctionName {
            choice_index,
            tool_index,
            existing: existing.clone(),
            received,
        });
    }
    *target = Some(received);
    Ok(())
}
