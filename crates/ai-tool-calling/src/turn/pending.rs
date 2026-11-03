//! Completed tool messages that survive a pending write or a dropped stream.

use std::sync::Arc;

use ai_interface::ConversationMessage;
use parking_lot::Mutex;

#[derive(Clone)]
pub(super) struct PendingMessages {
    slots: Arc<Mutex<Vec<Slot>>>,
}

#[derive(Default)]
struct Slot {
    message: Option<ConversationMessage>,
    ready: bool,
}

impl PendingMessages {
    pub(super) fn new(count: usize) -> Self {
        Self {
            slots: Arc::new(Mutex::new((0..count).map(|_| Slot::default()).collect())),
        }
    }

    pub(super) fn publish(&self, index: usize, message: ConversationMessage, ready: bool) {
        if let Some(slot) = self.slots.lock().get_mut(index) {
            slot.message = Some(message);
            slot.ready = ready;
        }
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
