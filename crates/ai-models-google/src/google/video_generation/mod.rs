//! Google video generation implementation.

mod client;
mod deadline;
mod error;
mod lifecycle;
mod request;
mod response;
mod transport;

pub use client::GoogleVideoGenerator;

#[cfg(test)]
#[path = "../_tests_/video_generation/mod.rs"]
mod video_generation_tests;
