//! UTF-8 byte window calculations for tool output stores.

use crate::output_store::{
    ToolOutputStoreError, ToolOutputStoreReadRequest, ToolOutputStoreResult, ToolOutputStoreWindow,
};

pub(super) fn read_window(
    tool_name: impl Into<String>,
    content: &str,
    request: ToolOutputStoreReadRequest,
) -> ToolOutputStoreResult<ToolOutputStoreWindow> {
    let tool_name = tool_name.into();
    let offset = request.offset.unwrap_or(0);
    let total_bytes = content.len();
    if offset > total_bytes || !content.is_char_boundary(offset) {
        return Err(ToolOutputStoreError::InvalidOffset {
            output_id: request.output_id,
            offset,
            total_bytes,
        });
    }
    let requested_length = request.length.unwrap_or(request.policy.read_limit_bytes());
    if requested_length == 0 {
        return Err(ToolOutputStoreError::InvalidLength {
            length: requested_length,
        });
    }
    let length = requested_length.min(request.policy.read_limit_bytes());
    if offset == total_bytes {
        return Ok(ToolOutputStoreWindow {
            tool_name,
            offset,
            content: String::new(),
            returned_bytes: 0,
            total_bytes,
            truncated: false,
            next_offset: None,
        });
    }
    let window = prefix_window_at(tool_name, content, offset, length);
    if window.returned_bytes == 0 {
        return Err(ToolOutputStoreError::NoCompleteCharacterFits {
            offset,
            length,
            minimum_usable_length: next_character_len(content, offset),
        });
    }
    Ok(window)
}

pub(super) fn prefix_window(
    tool_name: impl Into<String>,
    content: &str,
    length: usize,
) -> ToolOutputStoreWindow {
    prefix_window_at(tool_name, content, 0, length)
}

fn prefix_window_at(
    tool_name: impl Into<String>,
    content: &str,
    offset: usize,
    length: usize,
) -> ToolOutputStoreWindow {
    let total_bytes = content.len();
    let mut end = offset.saturating_add(length).min(total_bytes);
    while end > offset && !content.is_char_boundary(end) {
        end -= 1;
    }
    let returned = content[offset..end].to_owned();
    let truncated = end < total_bytes;
    ToolOutputStoreWindow {
        tool_name: tool_name.into(),
        offset,
        returned_bytes: returned.len(),
        content: returned,
        total_bytes,
        truncated,
        next_offset: truncated.then_some(end),
    }
}

fn next_character_len(content: &str, offset: usize) -> usize {
    match content[offset..].chars().next() {
        Some(character) => character.len_utf8(),
        None => 1,
    }
}
