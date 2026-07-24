//! Shared AI contract types and traits.

#![warn(unreachable_pub)]

mod audio_transcriber;
mod logger;
mod messages;
mod mock_audio_transcriber;
mod mock_model;
mod model;
pub mod output;
mod router;
mod tools;
mod usage;

pub use audio_transcriber::{
    AudioTranscriber, AudioTranscriptionRequest, AudioTranscriptionResponse, DynAudioTranscriber,
    TranscriptionError, TranscriptionResult,
};
pub use logger::{
    DynLogger, Logger, LoggerError, LoggerResult, ModelCallLogEntry, ModelCallLogResult,
    NoopLogger, ToolActivityLogEntry, ToolActivityPhase, ToolCallLogEntry, ToolCallLogResult,
    TurnOutcomeLogEntry,
};
pub use messages::{
    ConversationContentPart, ConversationMessage, ConversationRole, OpenAiReasoningSummary,
    ProviderConversationItem,
};
pub use mock_audio_transcriber::MockAudioTranscriber;
pub use mock_model::MockModel;
pub use model::{
    DynModel, FinishReason, Model, ModelError, ModelRequest, ModelResponse, ModelResult,
    StructuredOutputSchema,
};
pub use output::{
    ToolOutputEnvelope, ToolOutputEnvelopeError, ToolOutputEnvelopeResult, ToolOutputId,
    ToolOutputInlineEnvelope, ToolOutputReadRequest, ToolOutputRemainderUnavailableReason,
    ToolOutputWindowEnvelope,
};
pub use router::{
    DynModelRouter, ModelFeature, ModelPreference, ModelRequirement, ModelRouteRequest,
    ModelRouteRequestBuilder, ModelRouter, ModelRouterError, ModelRouterResult, ProviderKind,
};
pub use tools::{DynTool, Tool, ToolCall, ToolDefinition, ToolError, ToolInvocation, ToolResult};
pub use usage::{ModelUsage, ModelUsageCostLine, ModelUsageMeasurementState, ModelUsageUnitKind};

#[cfg(any(test, doctest, feature = "test-support"))]
pub use audio_transcriber::AudioTranscriberMock;
#[cfg(any(test, doctest, feature = "test-support"))]
pub use logger::LoggerMock;
#[cfg(any(test, doctest, feature = "test-support"))]
pub use model::ModelMock;
#[cfg(any(test, doctest, feature = "test-support"))]
pub use router::ModelRouterMock;
#[cfg(any(test, doctest, feature = "test-support"))]
pub use tools::ToolMock;

#[cfg(test)]
#[path = "_tests_/mod.rs"]
mod tests;
