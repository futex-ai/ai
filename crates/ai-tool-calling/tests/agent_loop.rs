//! Public agent-loop regression contracts.

#[cfg(test)]
#[path = "_tests_/catalog_tests.rs"]
mod catalog_tests;
#[cfg(test)]
#[path = "_tests_/checkpoint_tests.rs"]
mod checkpoint_tests;
#[cfg(test)]
#[path = "_tests_/controlled.rs"]
mod controlled;
#[cfg(test)]
#[path = "_tests_/error_source_tests.rs"]
mod error_source_tests;
#[cfg(test)]
#[path = "_tests_/observer_tests.rs"]
mod observer_tests;
#[cfg(test)]
#[path = "_tests_/outcome_tests.rs"]
mod outcome_tests;
#[cfg(test)]
#[path = "_tests_/parallel_tests.rs"]
mod parallel_tests;
#[cfg(test)]
#[path = "_tests_/reader_tests.rs"]
mod reader_tests;
#[cfg(test)]
#[path = "_tests_/response_checkpoint_tests.rs"]
mod response_checkpoint_tests;
#[cfg(test)]
#[path = "_tests_/storage_stop_tests.rs"]
mod storage_stop_tests;
#[cfg(test)]
#[path = "_tests_/stream_tests.rs"]
mod stream_tests;
#[cfg(test)]
#[path = "_tests_/support.rs"]
mod support;
#[cfg(test)]
#[path = "_tests_/typed_tool_tests.rs"]
mod typed_tool_tests;
#[cfg(test)]
#[path = "_tests_/warning_tests.rs"]
mod warning_tests;
