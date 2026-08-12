//! OpenAI image generation implementation.

mod client;
mod error;
mod request;
mod response;

pub use client::OpenAiImageGenerator;

#[cfg(test)]
#[path = "../_tests_/image_generation/mod.rs"]
mod image_generation_tests;
