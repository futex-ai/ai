//! Agent-loop tests and shared fixtures.

//! Agent-loop, output-store, policy, and provider-state regression tests.

mod conversation_runtime_tests;
mod deepseek_logging_redaction_tests;
#[cfg(test)]
#[path = "event_support.rs"]
mod event_support;
#[cfg(test)]
#[path = "intrinsic_catalog_tests.rs"]
mod intrinsic_catalog_tests;
mod intrinsic_reader_tests;
mod kimi_logging_redaction_tests;
mod model_controls_tests;
mod model_logging_redaction_tests;
mod output_id_generation_tests;
mod output_store_tests;
mod policy_tests;
mod qwen_logging_redaction_tests;
mod signed_replay_logging_tests;
mod support;
mod turn_error_logging_tests;
mod turn_execution;
mod turn_outcome_tests;
