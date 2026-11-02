//! Indexed text and reasoning fragments with typed terminal supplementation.

use std::collections::BTreeMap;

use ai_interface::ModelEvent;

use crate::openai::response::types::{ResponsesContentPart, ResponsesOutputItem};

use super::types::{ContentLocation, ItemKey, ItemLocation};

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(super) enum ContentKind {
    Text,
    Reasoning,
}

#[derive(Debug, Eq, Ord, PartialEq, PartialOrd)]
struct ContentKey {
    item: ItemKey,
    content_index: usize,
    kind: ContentKind,
}

#[derive(Debug)]
struct TextPart {
    part_index: usize,
    text: String,
}

#[derive(Debug, Default)]
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
            }
        });
        part.text.push_str(&delta);
        events.push(match kind {
            ContentKind::Text => ModelEvent::TextDelta {
                part_index: part.part_index,
                delta,
            },
            ContentKind::Reasoning => ModelEvent::ReasoningDelta {
                part_index: part.part_index,
                delta,
            },
        });
    }

    pub(super) fn supplement_item(
        &mut self,
        output_index: usize,
        item: &ResponsesOutputItem,
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
