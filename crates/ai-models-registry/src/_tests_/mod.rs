//! Registry contracts through public construction and adapter boundaries.

mod build_tests;
mod catalog_tests;
mod converter_tests;
mod support;

#[cfg(any(
    feature = "anthropic",
    feature = "deepseek",
    feature = "google",
    feature = "kimi",
    feature = "minimax",
    feature = "openai",
    feature = "qwen",
    feature = "xai"
))]
mod model_support;
#[cfg(any(
    feature = "anthropic",
    feature = "deepseek",
    feature = "google",
    feature = "kimi",
    feature = "minimax",
    feature = "openai",
    feature = "qwen",
    feature = "xai"
))]
mod model_tests;

#[cfg(any(feature = "google", feature = "openai"))]
mod image_tests;
#[cfg(feature = "typesafe")]
mod judgment_tests;
#[cfg(any(feature = "google", feature = "openai", feature = "typesafe"))]
mod media_support;
#[cfg(feature = "openai")]
mod transcription_tests;
#[cfg(feature = "google")]
mod video_tests;
