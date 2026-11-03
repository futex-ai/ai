//! Bounded tool messages and observer results retained until synchronous publication.

use std::{sync::Arc, time::Instant};

use ai_interface::{ConversationMessage, ToolCallLogResult};
use parking_lot::Mutex;

#[derive(Clone)]
pub(super) struct PendingMessages {
    slots: Arc<Mutex<Vec<Slot>>>,
}

#[derive(Default)]
struct Slot {
    message: Option<ConversationMessage>,
    log_result: Option<(ToolCallLogResult, Instant)>,
    ready: bool,
}

impl PendingMessages {
    pub(super) fn new(count: usize) -> Self {
        Self {
            slots: Arc::new(Mutex::new((0..count).map(|_| Slot::default()).collect())),
        }
    }

    pub(super) fn publish(
        &self,
        index: usize,
        message: ConversationMessage,
        log_result: ToolCallLogResult,
        ready: bool,
    ) {
        if let Some(slot) = self.slots.lock().get_mut(index) {
            slot.message = Some(message);
            slot.log_result = Some((log_result, Instant::now()));
            slot.ready = ready;
        }
    }

    pub(super) fn take_log_result(&self, index: usize) -> Option<(ToolCallLogResult, Instant)> {
        self.slots
            .lock()
            .get_mut(index)
            .and_then(|slot| slot.log_result.take())
    }

    pub(super) fn take_remaining_logs(&self) -> Vec<(usize, ToolCallLogResult, Instant)> {
        self.slots
            .lock()
            .iter_mut()
            .enumerate()
            .filter_map(|(index, slot)| {
                slot.log_result
                    .take()
                    .map(|(result, completed_at)| (index, result, completed_at))
            })
            .collect()
    }

    pub(super) fn flush(
        &self,
        conversation: &mut Vec<ConversationMessage>,
        cursor: &mut usize,
        stopped: bool,
    ) {
        let mut slots = self.slots.lock();
        while let Some(slot) = slots.get_mut(*cursor) {
            if !stopped && !slot.ready {
                break;
            }
            if let Some(message) = slot.message.take() {
                conversation.push(message);
            }
            *cursor += 1;
        }
    }
}
