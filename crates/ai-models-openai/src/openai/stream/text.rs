//! Indexed text and reasoning fragments with typed terminal supplementation.

use std::collections::BTreeMap;

use ai_interface::{ModelEvent, ProviderState};

use crate::openai::response::types::{ResponsesContentPart, ResponsesOutputItem};

use super::types::{ContentLocation, ItemKey, ItemLocation};

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(super) enum ContentKind {
    Text,
    Reasoning,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct ContentKey {
    item: ItemKey,
    content_index: usize,
    kind: ContentKind,
}

#[derive(Clone, Debug)]
struct TextPart {
    part_index: usize,
    text: String,
    provider_state: Option<ProviderState>,
}

#[derive(Clone, Debug, Default)]
pub(super) struct TextParts {
    parts: BTreeMap<ContentKey, TextPart>,
}

impl TextParts {
    pub(super) fn push(
        &mut self,
        location: ContentLocation,
        kind: ContentKind,
        delta: String,
        next_part_index: &mut usize,
        events: &mut Vec<ModelEvent>,
    ) {
        if delta.is_empty() {
            return;
        }
        let key = ContentKey {
            item: location.item.key(),
            content_index: location.content_index,
            kind,
        };
        let part = self.parts.entry(key).or_insert_with(|| {
            let index = *next_part_index;
            *next_part_index = next_part_index.saturating_add(1);
            TextPart {
                part_index: index,
                text: String::new(),
                provider_state: None,
            }
        });
        part.text.push_str(&delta);
        events.push(match kind {
            ContentKind::Text => ModelEvent::TextDelta {
                part_index: part.part_index,
                delta,
                provider_state: None,
            },
            ContentKind::Reasoning => ModelEvent::ReasoningDelta {
                part_index: part.part_index,
                delta,
                provider_state: None,
            },
        });
    }

    pub(super) fn supplement_item(
        &mut self,
        output_index: usize,
        item: &ResponsesOutputItem,
        state: Option<ProviderState>,
        next_part_index: &mut usize,
        events: &mut Vec<ModelEvent>,
    ) {
        match item {
            ResponsesOutputItem::Message { id, content, .. } => {
                for (content_index, part) in content.iter().enumerate() {
                    let text = match part {
                        ResponsesContentPart::OutputText { text } => text,
                        ResponsesContentPart::Refusal { refusal } => refusal,
                        ResponsesContentPart::Other => continue,
                    };
                    self.supplement(
                        ContentLocation {
                            item: ItemLocation {
                                output_index: Some(output_index),
                                item_id: id.clone(),
                            },
                            content_index,
                        },
                        ContentKind::Text,
                        text,
                        next_part_index,
                        events,
                    );
                }
            }
            ResponsesOutputItem::Reasoning { id, summary, .. } => {
                for (content_index, part) in summary.iter().enumerate() {
                    self.supplement(
                        ContentLocation {
                            item: ItemLocation {
                                output_index: Some(output_index),
                                item_id: Some(id.clone()),
                            },
                            content_index,
                        },
                        ContentKind::Reasoning,
                        &part.text,
                        next_part_index,
                        events,
                    );
                }
            }
            ResponsesOutputItem::FunctionCall { .. } | ResponsesOutputItem::Other => {}
        }
        if let Some(state) = state {
            self.attach_state(output_index, item, state, next_part_index, events);
        }
    }

    fn supplement(
        &mut self,
        location: ContentLocation,
        kind: ContentKind,
        text: &str,
        next_part_index: &mut usize,
        events: &mut Vec<ModelEvent>,
    ) {
        let key = ContentKey {
            item: location.item.key(),
            content_index: location.content_index,
            kind,
        };
        if self.parts.contains_key(&key) {
            return;
        }
        let candidates = [
            location
                .item
                .item_id
                .as_ref()
                .map(|id| ItemKey::Id(id.clone())),
            Some(ItemKey::Anonymous),
        ];
        for item in candidates.into_iter().flatten() {
            if let Some(part) = self.parts.remove(&ContentKey {
                item,
                content_index: location.content_index,
                kind,
            }) {
                self.parts.insert(key, part);
                return;
            }
        }
        self.push(location, kind, text.to_owned(), next_part_index, events);
    }

    fn attach_state(
        &mut self,
        output_index: usize,
        item: &ResponsesOutputItem,
        state: ProviderState,
        next_part_index: &mut usize,
        events: &mut Vec<ModelEvent>,
    ) {
        let kind = match item {
            ResponsesOutputItem::Message { .. } => ContentKind::Text,
            ResponsesOutputItem::Reasoning { .. } => ContentKind::Reasoning,
            _ => return,
        };
        let key = self
            .parts
            .keys()
            .find(|key| key.item == ItemKey::Index(output_index) && key.kind == kind)
            .map(|key| key.content_index)
            .unwrap_or_default();
        let part = self
            .parts
            .entry(ContentKey {
                item: ItemKey::Index(output_index),
                content_index: key,
                kind,
            })
            .or_insert_with(|| {
                let part_index = *next_part_index;
                *next_part_index = next_part_index.saturating_add(1);
                TextPart {
                    part_index,
                    text: String::new(),
                    provider_state: None,
                }
            });
        if part.provider_state.as_ref() == Some(&state) {
            return;
        }
        part.provider_state = Some(state.clone());
        match events.last_mut() {
            Some(
                ModelEvent::TextDelta {
                    part_index,
                    provider_state,
                    ..
                }
                | ModelEvent::ReasoningDelta {
                    part_index,
                    provider_state,
                    ..
                },
            ) if *part_index == part.part_index => *provider_state = Some(state),
            _ => events.push(match kind {
                ContentKind::Text => ModelEvent::TextDelta {
                    part_index: part.part_index,
                    delta: String::new(),
                    provider_state: Some(state),
                },
                ContentKind::Reasoning => ModelEvent::ReasoningDelta {
                    part_index: part.part_index,
                    delta: String::new(),
                    provider_state: Some(state),
                },
            }),
        }
    }

    pub(super) fn assistant_message(&self) -> String {
        let mut parts = self
            .parts
            .iter()
            .filter(|(key, _)| key.kind == ContentKind::Text)
            .map(|(_, part)| (part.part_index, part.text.as_str()))
            .collect::<Vec<_>>();
        parts.sort_by_key(|(index, _)| *index);
        parts.into_iter().map(|(_, text)| text).collect()
    }
}
