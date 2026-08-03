//! Single-use, expiring OAuth authorization state tracking.

use std::collections::BTreeMap;

use secrecy::{ExposeSecret, SecretString};
use sha2::{Digest, Sha256};
use tokio::sync::Mutex;

use crate::{Error, Result};

pub(crate) struct AuthorizationStateTracker {
    records: Mutex<BTreeMap<[u8; 32], StateRecord>>,
}

impl AuthorizationStateTracker {
    pub(crate) fn new() -> Self {
        Self {
            records: Mutex::new(BTreeMap::new()),
        }
    }

    pub(crate) async fn begin(
        &self,
        state: &SecretString,
        now: u64,
        lifetime_seconds: u64,
    ) -> Result<StateHandle> {
        let key = state_key(state.expose_secret());
        let mut records = self.records.lock().await;
        records.retain(|_, record| record.expires_at >= now);
        if records.contains_key(&key) {
            return Err(Error::StateCollision);
        }
        records.insert(
            key,
            StateRecord {
                expires_at: now.saturating_add(lifetime_seconds),
                consumed: false,
            },
        );
        Ok(StateHandle { key })
    }

    pub(crate) async fn consume(
        &self,
        handle: &StateHandle,
        expected: &SecretString,
        returned: Option<&SecretString>,
        now: u64,
    ) -> Result<()> {
        let mut records = self.records.lock().await;
        let Some(record) = records.get_mut(&handle.key) else {
            return Err(Error::StateReused);
        };
        if record.consumed {
            return Err(Error::StateReused);
        }
        record.consumed = true;
        if now > record.expires_at {
            return Err(Error::StateExpired);
        }
        let Some(returned) = returned else {
            return Err(Error::StateMissing);
        };
        if !constant_time_equal(
            expected.expose_secret().as_bytes(),
            returned.expose_secret().as_bytes(),
        ) {
            return Err(Error::StateMismatch);
        }
        Ok(())
    }

    pub(crate) async fn invalidate(&self, handle: &StateHandle) {
        if let Some(record) = self.records.lock().await.get_mut(&handle.key) {
            record.consumed = true;
        }
    }
}

pub(crate) struct StateHandle {
    key: [u8; 32],
}

struct StateRecord {
    expires_at: u64,
    consumed: bool,
}

fn state_key(state: &str) -> [u8; 32] {
    Sha256::digest(state.as_bytes()).into()
}

fn constant_time_equal(left: &[u8], right: &[u8]) -> bool {
    let maximum = left.len().max(right.len());
    let mut difference = left.len() ^ right.len();
    for index in 0..maximum {
        difference |= usize::from(left.get(index).copied().unwrap_or(0))
            ^ usize::from(right.get(index).copied().unwrap_or(0));
    }
    difference == 0
}
