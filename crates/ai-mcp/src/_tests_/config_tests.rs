//! Configuration contract tests.

use std::time::Duration;

use crate::McpServerConfig;

#[test]
fn defaults_match_the_protocol_contract() {
    let config = McpServerConfig::new("calendar", "https://example.com/mcp");

    assert_eq!(config.server_key, "calendar");
    assert_eq!(config.url, "https://example.com/mcp");
    assert_eq!(config.request_timeout, Duration::from_secs(30));
    assert_eq!(config.tool_call_timeout, Duration::from_secs(120));
    assert_eq!(config.max_response_bytes, 1024 * 1024);
    assert_eq!(config.activity_verb, None);
}

#[test]
fn server_key_validation_accepts_only_the_approved_shape() {
    for key in ["a", "server_1", "remote-mcp", &"a".repeat(32)] {
        assert!(
            McpServerConfig::new(key, "https://example.com")
                .validate()
                .is_ok()
        );
    }

    for key in [
        "",
        "UPPER",
        "contains.dot",
        "contains space",
        &"a".repeat(33),
    ] {
        assert!(
            McpServerConfig::new(key, "https://example.com")
                .validate()
                .is_err()
        );
    }
}
