//! Lossless preservation when an old provider's converter is not installed.

use serde_json::Value;

use crate::ProviderId;

use super::{
    legacy::{LegacyPartAttachment, LegacyPartTarget, LegacyProviderItem},
    state::ProviderState,
};

pub(super) fn preserve_item(item: &LegacyProviderItem) -> LegacyPartAttachment {
    let call_id = item
        .fields
        .get("call_id")
        .or_else(|| item.fields.get("tool_call_id"))
        .and_then(Value::as_str);
    let target = if let Some(call_id) = call_id {
        LegacyPartTarget::ToolCall {
            call_id: call_id.to_owned(),
        }
    } else if item.kind.ends_with("_message") && !item.kind.ends_with("_assistant_message") {
        LegacyPartTarget::Text
    } else {
        LegacyPartTarget::Reasoning {
            text: item
                .fields
                .get("reasoning_content")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_owned(),
        }
    };
    LegacyPartAttachment {
        target,
        provider_state: ProviderState {
            provider: provider_for_tag(&item.kind),
            model: String::new(),
            version: 1,
            payload: item.fields.clone(),
        },
    }
}

fn provider_for_tag(tag: &str) -> ProviderId {
    let suffixes = [
        "_assistant_message",
        "_legacy_function_call",
        "_function_call",
        "_message",
        "_reasoning",
        "_assistant",
    ];
    let provider = suffixes
        .iter()
        .find_map(|suffix| tag.strip_suffix(suffix))
        .unwrap_or(tag);
    ProviderId::from(provider.replace('_', ""))
}
