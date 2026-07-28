//! Accepted MCP session DELETE status classification.

pub(crate) fn is_tolerated_delete_status(status: u16) -> bool {
    (200..300).contains(&status) || status == 405
}
