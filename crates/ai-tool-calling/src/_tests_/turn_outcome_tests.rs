//! Terminal turn log outcomes and permanently exhausted streams.

use std::sync::{Arc, Mutex};

use ai_interface::{FinishReason, LoggerMock, ModelMock, ModelResponse, ModelUsage, TurnOutcome};
use unimock::{MockFn, Unimock, matching};

use crate::tests::event_support::request;
use crate::tests::support::{response_parts, stream_result};
use crate::{Agent, AgentEvent};

use super::support::{runtime_with_logger, user_message};

#[tokio::test]
async fn terminal_logs_distinguish_completed_truncated_and_capped() {
    for (reason, max_steps, expected) in [
        (FinishReason::Stop, 1, TurnOutcome::Completed),
        (FinishReason::Truncated, 1, TurnOutcome::Truncated),
        (FinishReason::Stop, 0, TurnOutcome::Capped),
    ] {
        let entries = Arc::new(Mutex::new(Vec::new()));
        let log_outcome = LoggerMock::log_turn_outcome
            .some_call(matching!(_))
            .answers_arc({
                let entries = entries.clone();
                Arc::new(move |_, entry| {
                    entries.lock().unwrap().push(entry.outcome);
                    Ok(())
                })
            });
        let (model, logger) =
            if max_steps == 0 {
                (Unimock::new(()), Unimock::new(log_outcome))
            } else {
                (
                    Unimock::new(ModelMock::stream.some_call(matching!(_)).answers_arc(
                        stream_result(Ok(ModelResponse {
                            attempts: Vec::new(),
                            provider: "mock".to_owned(),
                            model_id: "mock".to_owned(),
                            catalog_model_id: None,
                            thinking_level: None,
                            parts: response_parts("partial".to_owned(), Vec::new(), Vec::new()),
                            finish_reason: reason,
                            structured_output: None,
                            usage: ModelUsage::default(),
                        })),
                    )),
                    Unimock::new((
                        log_outcome,
                        LoggerMock::log_model_call
                            .some_call(matching!(_))
                            .returns(Ok(())),
                    )),
                )
            };
        let runtime = runtime_with_logger(Arc::new(model), Arc::new(logger), Vec::new()).unwrap();
        let mut conversation = vec![user_message("start")];
        let mut stream = runtime.run(&mut conversation, request(Some(max_steps)));
        loop {
            let event = stream.next().await.unwrap().expect("turn must finish");
            if matches!(event, AgentEvent::Finished(_)) {
                break;
            }
        }
        assert!(stream.next().await.unwrap().is_none());
        let entries = entries.lock().unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0], expected);
    }
}
