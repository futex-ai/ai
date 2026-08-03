//! End-to-end MCP OAuth integration coverage over in-process servers.

#[path = "oauth_integration/failure_tests.rs"]
mod failure_tests;
#[path = "oauth_integration/flow_tests.rs"]
mod flow_tests;
#[path = "oauth_integration/scope_tests.rs"]
mod scope_tests;

mod support;
