//! Session-bound HTTP status classification.

use std::collections::BTreeMap;

const SESSION_HEADER: &str = "mcp-session-id";

pub(crate) fn has_session_header(headers: &BTreeMap<String, String>) -> bool {
    headers
        .keys()
        .any(|name| name.eq_ignore_ascii_case(SESSION_HEADER))
}

pub(crate) fn is_expired_session_status(status: u16, session_bound: bool) -> bool {
    status == 404 && session_bound
}
