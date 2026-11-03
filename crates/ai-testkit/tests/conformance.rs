//! Offline provider conformance integration suite.

#[cfg(test)]
#[path = "_tests_/conformance_tests.rs"]
mod conformance_tests;

#[cfg(test)]
#[path = "_tests_/agent_conformance_tests.rs"]
mod agent_conformance_tests;

#[cfg(test)]
#[path = "_tests_/agent_support.rs"]
mod agent_support;
