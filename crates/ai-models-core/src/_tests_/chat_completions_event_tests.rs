//! Event-observing Chat Completions accumulator tests.

use serde_json::json;

use crate::{
    ChatCompletionsAccumulator, ChatCompletionsDelta, ChatCompletionsStreamStatus,
    ChatCompletionsStreamUpdate,
};

#[test]
fn observes_ordered_primary_text_without_changing_buffered_output() {
    let chunks = [
        json!({
            "choices": [{
                "index": 0,
                "delta": {"reasoning_content": "Think", "content": "Hel"},
                "finish_reason": null
            }]
        }),
        json!({
            "choices": [{
                "index": 0,
                "delta": {"reasoning_content": "ing", "content": "lo"},
                "finish_reason": "stop"
            }],
            "usage": {"prompt_tokens": 4, "completion_tokens": 4}
        }),
    ];
    let mut observed = ChatCompletionsAccumulator::new();
    let mut buffered = ChatCompletionsAccumulator::new();
    let mut deltas = Vec::new();

    for chunk in &chunks {
        let update = observed
            .push_data_with_deltas(&chunk.to_string())
            .expect("valid chunk should be observed");
        let ChatCompletionsStreamUpdate::Chunk {
            deltas: chunk_deltas,
        } = update
        else {
            panic!("JSON chunk should not complete the stream");
        };
        deltas.extend(chunk_deltas);
        assert_eq!(
            buffered
                .push_data(&chunk.to_string())
                .expect("valid chunk should remain bufferable"),
            ChatCompletionsStreamStatus::Chunk
        );
    }
    assert_eq!(
        observed
            .push_data_with_deltas("[DONE]")
            .expect("observed stream should accept its sentinel"),
        ChatCompletionsStreamUpdate::Done
    );
    buffered
        .push_data("[DONE]")
        .expect("buffered stream should accept its sentinel");

    assert_eq!(
        deltas,
        vec![
            reasoning_delta("Think"),
            assistant_delta("Hel"),
            reasoning_delta("ing"),
            assistant_delta("lo"),
        ]
    );
    assert_eq!(
        observed.finish().expect("observed stream should finish"),
        buffered.finish().expect("buffered stream should finish")
    );
}

#[test]
fn observing_path_ignores_empty_and_non_primary_text_fragments() {
    let mut accumulator = ChatCompletionsAccumulator::new();
    let update = accumulator
        .push_data_with_deltas(
            &json!({
                "choices": [
                    {
                        "index": 1,
                        "delta": {"content": "alternate", "reasoning_content": "alternate"},
                        "finish_reason": "stop"
                    },
                    {
                        "index": 0,
                        "delta": {"content": "", "reasoning_content": ""},
                        "finish_reason": "stop"
                    }
                ],
                "usage": {"prompt_tokens": 1, "completion_tokens": 1}
            })
            .to_string(),
        )
        .expect("valid chunk should be observed");

    assert_eq!(
        update,
        ChatCompletionsStreamUpdate::Chunk { deltas: Vec::new() }
    );
}

fn assistant_delta(delta: &str) -> ChatCompletionsDelta {
    ChatCompletionsDelta::AssistantText {
        delta: delta.to_owned(),
    }
}

fn reasoning_delta(delta: &str) -> ChatCompletionsDelta {
    ChatCompletionsDelta::ReasoningText {
        delta: delta.to_owned(),
    }
}
