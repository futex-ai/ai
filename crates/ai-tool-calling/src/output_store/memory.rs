//! Ephemeral in-memory tool output store.

use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use ai_interface::ToolOutputId;
use async_trait::async_trait;
use parking_lot::Mutex;
use thiserror::Error;
use uuid::Uuid;

use crate::ToolOutputPolicy;
use crate::output_store::windowing::{prefix_window, read_window};
use crate::output_store::{
    ToolOutputStore, ToolOutputStoreError, ToolOutputStoreReadRequest, ToolOutputStoreResult,
    ToolOutputStoreWindow, ToolOutputWriteRequest, ToolOutputWriteResult,
};

/// Ephemeral in-memory output store scoped to one active runtime run.
pub struct InMemoryToolOutputStore {
    inner: Arc<InMemoryToolOutputStoreInner>,
}

struct InMemoryToolOutputStoreInner {
    entries: Mutex<BTreeMap<ToolOutputId, StoredToolOutput>>,
    reserved_bytes: AtomicUsize,
    fail_next_write: AtomicBool,
}

#[derive(Clone)]
struct StoredToolOutput {
    tool_name: String,
    content: String,
}

impl InMemoryToolOutputStore {
    /// Builds an empty in-memory output store.
    pub fn new() -> Self {
        Self {
            inner: Arc::new(InMemoryToolOutputStoreInner {
                entries: Mutex::new(BTreeMap::new()),
                reserved_bytes: AtomicUsize::new(0),
                fail_next_write: AtomicBool::new(false),
            }),
        }
    }

    /// Returns the aggregate bytes currently reserved by stored outputs.
    pub fn reserved_bytes(&self) -> usize {
        self.inner.reserved_bytes.load(Ordering::SeqCst)
    }

    /// Causes the next write to fail after aggregate reservation.
    #[cfg(any(test, feature = "test-support"))]
    pub fn fail_next_write_for_test(&self) {
        self.inner.fail_next_write.store(true, Ordering::SeqCst);
    }
}

impl Default for InMemoryToolOutputStore {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ToolOutputStore for InMemoryToolOutputStore {
    async fn write(
        &self,
        request: ToolOutputWriteRequest,
    ) -> ToolOutputStoreResult<ToolOutputWriteResult> {
        let total_bytes = request.content.len();
        let first_window = prefix_window(
            request.tool_name.clone(),
            &request.content,
            request.first_window_length,
        );
        if total_bytes > request.policy.per_output_limit_bytes() {
            return Err(ToolOutputStoreError::PerOutputOverflow {
                requested_bytes: total_bytes,
                limit_bytes: request.policy.per_output_limit_bytes(),
                window: first_window,
            });
        }
        self.reserve(total_bytes, &request.policy, first_window.clone())?;
        if self.inner.fail_next_write.swap(false, Ordering::SeqCst) {
            self.rollback(total_bytes);
            return Err(ToolOutputStoreError::write_failure(
                first_window,
                InMemoryInjectedWriteFailure,
            ));
        }
        let output_id = ToolOutputId::from_opaque(format!("toolout_{}", Uuid::now_v7()));
        {
            let mut entries = self.inner.entries.lock();
            if entries.contains_key(&output_id) {
                self.rollback(total_bytes);
                return Err(ToolOutputStoreError::write_failure(
                    first_window,
                    InMemoryOutputIdCollision,
                ));
            }
            entries.insert(
                output_id.clone(),
                StoredToolOutput {
                    tool_name: request.tool_name,
                    content: request.content,
                },
            );
        }
        Ok(ToolOutputWriteResult {
            output_id,
            first_window,
        })
    }

    async fn read(
        &self,
        request: ToolOutputStoreReadRequest,
    ) -> ToolOutputStoreResult<ToolOutputStoreWindow> {
        let output = {
            let entries = self.inner.entries.lock();
            match entries.get(&request.output_id) {
                Some(output) => output.clone(),
                None => {
                    return Err(ToolOutputStoreError::UnavailableOutput {
                        output_id: request.output_id,
                    });
                }
            }
        };
        read_window(output.tool_name, &output.content, request)
    }
}

impl InMemoryToolOutputStore {
    fn reserve(
        &self,
        bytes: usize,
        policy: &ToolOutputPolicy,
        window: ToolOutputStoreWindow,
    ) -> ToolOutputStoreResult<()> {
        loop {
            let current = self.inner.reserved_bytes.load(Ordering::SeqCst);
            let next = match current.checked_add(bytes) {
                Some(next) => next,
                None => {
                    return Err(aggregate_exhausted_error(bytes, current, policy, window));
                }
            };
            if next > policy.aggregate_limit_bytes() {
                return Err(aggregate_exhausted_error(bytes, current, policy, window));
            }
            if self
                .inner
                .reserved_bytes
                .compare_exchange(current, next, Ordering::SeqCst, Ordering::SeqCst)
                .is_ok()
            {
                return Ok(());
            }
        }
    }

    fn rollback(&self, bytes: usize) {
        self.inner.reserved_bytes.fetch_sub(bytes, Ordering::SeqCst);
    }
}

fn aggregate_exhausted_error(
    requested_bytes: usize,
    current_bytes: usize,
    policy: &ToolOutputPolicy,
    window: ToolOutputStoreWindow,
) -> ToolOutputStoreError {
    let available_bytes = policy.aggregate_limit_bytes().saturating_sub(current_bytes);
    ToolOutputStoreError::AggregateExhausted {
        requested_bytes,
        available_bytes,
        limit_bytes: policy.aggregate_limit_bytes(),
        window,
    }
}

#[derive(Debug, Error)]
#[error("[ai_tool_calling/output_store] injected in-memory write failure")]
struct InMemoryInjectedWriteFailure;

#[derive(Debug, Error)]
#[error("[ai_tool_calling/output_store] generated duplicate output id")]
struct InMemoryOutputIdCollision;
