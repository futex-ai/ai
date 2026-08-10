//! Collision-safe provider-compatible names for MCP tools.

use std::collections::BTreeSet;

use crate::McpToolDescriptor;

const MAX_TOOL_NAME_BYTES: usize = 64;

pub(crate) fn prefixed_names(server_key: &str, descriptors: &[McpToolDescriptor]) -> Vec<String> {
    let prefix = format!("mcp__{server_key}__");
    let available = MAX_TOOL_NAME_BYTES.saturating_sub(prefix.len());
    let mut used = BTreeSet::new();
    descriptors
        .iter()
        .map(|descriptor| unique_name(&prefix, &sanitize(&descriptor.name), available, &mut used))
        .collect()
}

fn sanitize(original: &str) -> String {
    original
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '_' | '-') {
                character
            } else {
                '_'
            }
        })
        .collect()
}

fn unique_name(
    prefix: &str,
    sanitized: &str,
    available: usize,
    used: &mut BTreeSet<String>,
) -> String {
    let base = truncate_ascii(sanitized, available);
    let first = format!("{prefix}{base}");
    if used.insert(first.clone()) {
        return first;
    }
    let mut sequence = 2_u64;
    loop {
        let suffix = format!("_{sequence}");
        let stem_limit = available.saturating_sub(suffix.len());
        let candidate = format!("{prefix}{}{suffix}", truncate_ascii(sanitized, stem_limit));
        if used.insert(candidate.clone()) {
            return candidate;
        }
        sequence = sequence.saturating_add(1);
    }
}

fn truncate_ascii(value: &str, max_bytes: usize) -> &str {
    &value[..value.len().min(max_bytes)]
}
