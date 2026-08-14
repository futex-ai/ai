//! OpenAI video generation implementation.

mod client;
mod error;
mod request;
mod response;

pub use client::OpenAiVideoGenerator;

#[cfg(test)]
#[path = "../_tests_/video_generation/mod.rs"]
mod video_generation_tests;
