//! Credentialed end-to-end checks for every judgment-capable catalog model.

mod catalog_tests;
mod layout_tests;
mod provider_tests;
mod retry_tests;
mod runner_tests;
mod validation_tests;
mod workflow_tests;

#[cfg(test)]
#[path = "_tests_/catalog_fixtures.rs"]
mod catalog_fixtures;
