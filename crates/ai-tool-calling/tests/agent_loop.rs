//! Public agent-loop regression contracts.

#[cfg(test)]
#[path = "_tests_/controlled.rs"]
mod controlled;
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
#[path = "_tests_/stream_tests.rs"]
mod stream_tests;
#[cfg(test)]
#[path = "_tests_/support.rs"]
mod support;
