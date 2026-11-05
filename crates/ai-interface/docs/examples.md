# ai-interface examples

These examples show the main boundaries of `ai-interface`. Most of them use
its built-in mocks. They need no provider, credential, or network access. The
[crate README](../README.md) gives the model spec and effort example.

## Model calls

`Model::stream` returns the ordered events of one call. `Model::complete`
reads the same events and folds them into one `ModelResponse`. A consumer
that shows events as they arrive can use `ModelResponseFold`, the fold of
`complete`, to get the same response. Parts can interleave, so the consumer
collects each text part by its `part_index`. A `Fallback` event voids the
events of the failed attempt.

```rust
use std::collections::BTreeMap;

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
    Ok(response.text())
}

async fn stream_and_fold() -> ai_interface::ModelResult<(String, ModelResponse)> {
    let mut stream = MockModel::new("done").stream(&request()).await?;
    let mut fold = ModelResponseFold::new();
    let mut text_parts = BTreeMap::<usize, String>::new();
    while let Some(event) = stream.next().await? {
        match &event {
            ModelEvent::TextDelta { part_index, delta, .. } => {
                text_parts.entry(*part_index).or_default().push_str(delta);
            }
            ModelEvent::Fallback { .. } => text_parts.clear(),
            _ => {}
        }
        fold.push(event)?;
    }
    let shown = text_parts.into_values().collect::<String>();
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

## Conversation parts and provider state

A message is a role and ordered parts. A response part can carry provider
state that only its adapter reads. Keep every part of a response in the
conversation, so that the adapter can replay its own state on the next
request. Log only the log copy, which keeps every part but drops every
provider state and every reasoning text.

```rust
use ai_interface::{
    ConversationMessage, ConversationPart, ModelRequest, ModelResponse,
};

fn image_question(png_base64: &str) -> ConversationMessage {
    ConversationMessage::user_with_parts(vec![
        ConversationPart::text("What does this image show?"),
        ConversationPart::image("image/png", png_base64),
    ])
}

fn continue_after_tools(response: &ModelResponse) -> Vec<ConversationMessage> {
    // The assistant message keeps every part and its provider state.
    let mut messages = vec![response.to_assistant_message()];
    for call in response.tool_calls() {
        messages.push(ConversationMessage::tool("{\"ok\":true}", &call.name, &call.id));
    }
    messages
}

fn loggable(request: &ModelRequest) -> ModelRequest {
    request.log_copy()
}
```

## Stored messages

`ConversationMessage` writes stored format version 2. A host reads old
version 1 messages through the converters that the provider crates export.

```rust
use std::sync::Arc;

use ai_interface::{
    ConversationMessage, LegacyMessageConverter, LegacyProviderConverter, StoredMessageError,
};
use serde_json::Value;

fn save(message: &ConversationMessage) -> serde_json::Result<Value> {
    serde_json::to_value(message)
}

fn load(
    converters: Vec<Arc<dyn LegacyProviderConverter>>,
    stored: Value,
) -> Result<ConversationMessage, StoredMessageError> {
    LegacyMessageConverter::new(converters).read(stored)
}
```
