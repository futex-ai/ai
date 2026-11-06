//! Model and tool stages of one append-only turn.

use ai_interface::ModelResponseFold;

use crate::{AgentEvent, Result, RunOutcome};

use super::{
    super::{event_copy::copy_for_fold, observer::log_turn_outcome},
    state::{Stage, TurnStream},
};

impl TurnStream<'_> {
    pub(super) async fn advance(&mut self) -> Result<Option<AgentEvent>> {
        loop {
            match self.stage {
                Stage::Done | Stage::Drain => return Ok(None),
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
                    self.start_model();
                }
                Stage::OpenModel => {
                    if let Some(opening) = &mut self.opening {
                        self.model_stream = Some(opening.await?);
                    }
                    self.opening = None;
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
                    let result = match &mut self.tools {
                        Some(tools) => tools.next_event(self.conversation).await,
                        None => Ok(None),
                    };
                    let checkpoint_due = match &mut self.tools {
                        Some(tools) => tools.take_checkpoint_due(),
                        None => false,
                    };
                    let checkpoint = if checkpoint_due { self.check() } else { Ok(()) };
                    let event = match result {
                        Ok(event) => {
                            checkpoint?;
                            event
                        }
                        Err(error) => return Err(error),
                    };
                    match event {
                        Some(event) => {
                            return Ok(Some(event));
                        }
                        None => {
                            self.tools = None;
                            self.stage = Stage::StepEnd;
                        }
                    }
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
                        log_turn_outcome(&self.logger, &outcome, self.request.max_steps);
                        return Ok(Some(AgentEvent::Finished(outcome)));
                    }
                }
            }
        }
    }
}
