# ai-interface examples

These examples use the built-in mocks of `ai-interface` to show its main
boundaries. They need no provider, credential, or network access. The
[crate README](../README.md) gives the model spec and effort example.

## Model calls

`Model::stream` returns the ordered events of one call. `Model::complete`
reads the same events and folds them into one `ModelResponse`. A consumer
that shows events as they arrive can use `ModelResponseFold`, the fold of
`complete`, to get the same response. A `Fallback` event voids the events
of the failed attempt.

```rust
use ai_interface::{
    ConversationMessage, MockModel, Model, ModelEvent, ModelRequest, ModelResponse,
    ModelResponseFold,
};

fn request() -> ModelRequest {
    ModelRequest {
        system_prompt: "You are concise.".to_owned(),
        messages: vec![ConversationMessage::user("Summarize the status")],
        tools: Vec::new(),
        response_schema: None,
        controls: Default::default(),
    }
}

async fn call_model() -> ai_interface::ModelResult<String> {
    let response = MockModel::new("done").complete(&request()).await?;
    Ok(response.assistant_message)
}

async fn stream_and_fold() -> ai_interface::ModelResult<(String, ModelResponse)> {
    let mut stream = MockModel::new("done").stream(&request()).await?;
    let mut fold = ModelResponseFold::new();
    let mut shown = String::new();
    while let Some(event) = stream.next().await? {
        match &event {
            ModelEvent::TextDelta { delta, .. } => shown.push_str(delta),
            ModelEvent::Fallback { .. } => shown.clear(),
            _ => {}
        }
        fold.push(event)?;
    }
    Ok((shown, fold.finish()?))
}
```

## Test models

With the `test-support` feature, a test model can return fixed events.
`test_support::model_response_stream` turns a response fixture into the
events that fold back into it. `test_support::model_event_stream` boxes a
list of events.

```rust
use ai_interface::{
    DynModelEventStream, Model, ModelRequest, ModelResponse, ModelResult,
    test_support::model_response_stream,
};
use async_trait::async_trait;

struct FixedModel {
    response: ModelResponse,
}

#[async_trait]
impl Model for FixedModel {
    async fn stream(&self, _request: &ModelRequest) -> ModelResult<DynModelEventStream> {
        Ok(model_response_stream(self.response.clone()))
    }
}
```

## Image and video generation

```rust
use ai_interface::{
    ImageGenerationRequest, ImageGenerator, MockImageGenerator, MockVideoGenerator,
    VideoGenerationRequest, VideoGenerator,
};

async fn generate_image() -> ai_interface::ImageGenerationResult<Vec<u8>> {
    let generator = MockImageGenerator::default();
    let response = generator
        .generate(&ImageGenerationRequest {
            prompt: "A geometric blue bird".to_owned(),
            ..ImageGenerationRequest::default()
        })
        .await?;

    Ok(response.image.data)
}

async fn generate_video() -> ai_interface::VideoGenerationResult<Vec<u8>> {
    let generator = MockVideoGenerator::default();
    let response = generator
        .generate(&VideoGenerationRequest {
            prompt: "A geometric blue bird takes flight".to_owned(),
            ..VideoGenerationRequest::default()
        })
        .await?;

    Ok(response.video.data)
}
```

## Judgment

```rust
use std::collections::BTreeMap;

use ai_interface::{
    JudgmentModel, JudgmentQuestion, JudgmentRequest, JudgmentResponse, MockJudgmentModel,
};

async fn judge_state() -> ai_interface::JudgmentResult<JudgmentResponse> {
    MockJudgmentModel
        .judge(&JudgmentRequest {
            state: "The payout has failed for three days.".into(),
            questions: BTreeMap::from([(
                "urgent".to_owned(),
                JudgmentQuestion::condition("Is this urgent?", None),
            )]),
        })
        .await
}
```

## Tool output envelopes

A tool returns raw JSON. The runtime sends the model only a bounded
envelope, and the model reads a stored output in windows.

```rust
use ai_interface::{ToolOutputEnvelope, ToolOutputId, ToolOutputReadRequest};
use serde_json::json;

fn serialize_inline_tool_output() -> serde_json::Result<String> {
    let raw_output = json!({ "matches": ["alpha"] });
    let total_bytes = serde_json::to_string(&raw_output)?.len();
    let envelope = ToolOutputEnvelope::inline("search", raw_output, total_bytes);

    serde_json::to_string(&envelope)
}

fn read_next_window(output_id: &str) -> ToolOutputReadRequest {
    ToolOutputReadRequest {
        output_id: ToolOutputId::from_opaque(output_id),
        offset: Some(20_000),
        length: None,
    }
}
```

## Provider replay context

Each adapter replays only its own provider context on a later request.

```rust
use ai_interface::{
    DeepSeekToolCallContext, MiniMaxReasoningDetail, ProviderConversationItem, ProviderId,
};

fn minimax_replay_detail(text: &str) -> (ProviderId, MiniMaxReasoningDetail) {
    (
        ProviderId::new("minimax"),
        MiniMaxReasoningDetail {
            kind: Some("reasoning.text".to_owned()),
            id: None,
            format: None,
            index: Some(0),
            text: Some(text.to_owned()),
        },
    )
}

fn deepseek_replay_item() -> ProviderConversationItem {
    ProviderConversationItem::DeepSeekAssistantMessage {
        content: String::new(),
        reasoning_content: Some("private provider state".to_owned()),
        tool_calls: vec![DeepSeekToolCallContext {
            id: "call_1".to_owned(),
            name: "memory_read".to_owned(),
            arguments: "{\"path\":\"root\"}".to_owned(),
        }],
    }
}
```
