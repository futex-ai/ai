//! Shared AI contract types and traits.

#![warn(unreachable_pub)]

mod audio_transcriber;
mod effort;
mod image_generator;
mod judgment;
mod logger;
mod messages;
mod mock_audio_transcriber;
mod mock_catalog;
mod mock_image_generator;
mod mock_judgment_model;
mod mock_model;
mod mock_video_generator;
mod model;
mod model_attempt;
mod model_capabilities;
mod model_catalog;
mod model_controls;
mod model_error;
mod model_event;
mod model_helpers;
mod model_response_fold;
mod model_spec;
mod model_stream;
pub mod output;
mod provider_id;
mod router;
mod router_error;
mod routing_tiers;
mod tools;
mod usage;
mod video_generator;

#[cfg(any(test, doctest, feature = "test-support"))]
pub mod test_support;

pub use audio_transcriber::{
    AudioTranscriber, AudioTranscriptionRequest, AudioTranscriptionResponse, DynAudioTranscriber,
    TranscriptionError, TranscriptionResult,
};
pub use effort::{Effort, EffortSet};
pub use image_generator::{
    DynImageGenerator, GeneratedImage, ImageGenerationAspect, ImageGenerationError,
    ImageGenerationInputImage, ImageGenerationQuality, ImageGenerationRequest,
    ImageGenerationResponse, ImageGenerationResult, ImageGenerator,
};
pub use judgment::{
    DynJudgmentModel, EXPECTED_SCORE_TOLERANCE, JudgmentAnswer, JudgmentAnswerProblem,
    JudgmentConditionCriteria, JudgmentContent, JudgmentError, JudgmentJsonType, JudgmentModel,
    JudgmentQuestion, JudgmentQuestionKind, JudgmentQuestionProblem, JudgmentRequest,
    JudgmentResponse, JudgmentResult, PROBABILITY_SUM_TOLERANCE, SELECTED_PROBABILITY_TOLERANCE,
    deserialize_score_probabilities,
};
pub use logger::{
    DynLogger, Logger, LoggerError, LoggerResult, ModelCallLogEntry, ModelCallLogResult,
    NoopLogger, ToolActivityLogEntry, ToolActivityPhase, ToolCallLogEntry, ToolCallLogResult,
    TurnOutcome, TurnOutcomeLogEntry,
};
pub use messages::{
    ConversationMessage, ConversationPart, ConversationRole, LegacyConversationMessage,
    LegacyMessageConverter, LegacyPartAttachment, LegacyPartOrder, LegacyPartTarget,
    LegacyProviderConverter, LegacyProviderItem, ProviderState, StoredMessageError,
    compose_legacy_message, read_stored_message,
};
pub use mock_audio_transcriber::MockAudioTranscriber;
pub use mock_catalog::{MOCK_CATALOG, MOCK_CUSTOM_DEFAULTS, MOCK_MODEL};
pub use mock_image_generator::MockImageGenerator;
pub use mock_judgment_model::MockJudgmentModel;
pub use mock_model::MockModel;
pub use mock_video_generator::MockVideoGenerator;
pub use model::{
    DynModel, FinishReason, Model, ModelRequest, ModelResponse, ModelResult, StructuredOutputSchema,
};
pub use model_attempt::{ModelAttempt, ModelAttemptError};
pub use model_capabilities::{CustomModelDefaults, ForcedToolChoice, Sampling};
pub use model_catalog::ModelCatalog;
pub use model_controls::{
    ModelCallControls, ModelCompletionMode, ModelControl, ModelExecutionControls,
    ModelGenerationControls, ModelToolChoice,
};
pub use model_error::ModelError;
pub use model_event::ModelEvent;
pub use model_response_fold::ModelResponseFold;
pub use model_spec::ModelSpec;
pub use model_stream::{DynModelEventStream, ModelEventStream};
pub use output::{
    ToolOutputEnvelope, ToolOutputEnvelopeError, ToolOutputEnvelopeResult, ToolOutputId,
    ToolOutputInlineEnvelope, ToolOutputReadRequest, ToolOutputRemainderUnavailableReason,
    ToolOutputWindowEnvelope,
};
pub use provider_id::{MOCK_PROVIDER, ProviderId};
pub use router::{
    DynModelRouter, ModelFeature, ModelPreference, ModelRequirement, ModelRouteRequest,
    ModelRouteRequestBuilder, ModelRouter,
};
pub use router_error::{ModelRouterError, ModelRouterResult};
pub use routing_tiers::{CostTier, IntelligenceScore, RoutingTiers, SpeedTier};
pub use tools::{DynTool, Tool, ToolCall, ToolDefinition, ToolError, ToolInvocation, ToolResult};
pub use usage::{ModelUsage, ModelUsageCostLine, ModelUsageMeasurementState, ModelUsageUnitKind};
pub use video_generator::{
    DynVideoGenerator, GeneratedVideo, VideoGenerationAspect, VideoGenerationDuration,
    VideoGenerationError, VideoGenerationInputImage, VideoGenerationRequest,
    VideoGenerationResolution, VideoGenerationResponse, VideoGenerationResult, VideoGenerator,
};

#[cfg(any(test, doctest, feature = "test-support"))]
pub use audio_transcriber::AudioTranscriberMock;
#[cfg(any(test, doctest, feature = "test-support"))]
pub use image_generator::ImageGeneratorMock;
#[cfg(any(test, doctest, feature = "test-support"))]
pub use judgment::JudgmentModelMock;
#[cfg(any(test, doctest, feature = "test-support"))]
pub use logger::LoggerMock;
#[cfg(any(test, doctest, feature = "test-support"))]
pub use messages::LegacyProviderConverterMock;
#[cfg(any(test, doctest, feature = "test-support"))]
pub use model::ModelMock;
#[cfg(any(test, doctest, feature = "test-support"))]
pub use model_stream::ModelEventStreamMock;
#[cfg(any(test, doctest, feature = "test-support"))]
pub use router::ModelRouterMock;
#[cfg(any(test, doctest, feature = "test-support"))]
pub use tools::ToolMock;
#[cfg(any(test, doctest, feature = "test-support"))]
pub use video_generator::VideoGeneratorMock;

#[cfg(test)]
#[path = "_tests_/mod.rs"]
mod tests;
