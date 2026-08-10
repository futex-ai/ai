//! Atomic in-memory OAuth credential store for integration tests.

use std::{
    collections::BTreeMap,
    sync::atomic::{AtomicUsize, Ordering},
};

use ai_mcp_oauth::{
    OAuthClientRegistration, OAuthCredentialKey, OAuthCredentialStore, OAuthRegistrationKey,
    OAuthTokenSet, Result,
};
use async_trait::async_trait;
use tokio::sync::Mutex;

#[derive(Default)]
pub(crate) struct MemoryCredentialStore {
    registrations: Mutex<BTreeMap<OAuthRegistrationKey, OAuthClientRegistration>>,
    tokens: Mutex<BTreeMap<OAuthCredentialKey, OAuthTokenSet>>,
    token_deletions: AtomicUsize,
}

impl MemoryCredentialStore {
    pub(crate) async fn insert_tokens(&self, key: OAuthCredentialKey, tokens: OAuthTokenSet) {
        self.tokens.lock().await.insert(key, tokens);
    }

    pub(crate) async fn tokens(&self, key: &OAuthCredentialKey) -> Option<OAuthTokenSet> {
        self.tokens.lock().await.get(key).cloned()
    }

    pub(crate) async fn registration_count(&self) -> usize {
        self.registrations.lock().await.len()
    }

    pub(crate) fn token_deletion_count(&self) -> usize {
        self.token_deletions.load(Ordering::SeqCst)
    }
}

#[async_trait]
impl OAuthCredentialStore for MemoryCredentialStore {
    async fn load_registration(
        &self,
        key: &OAuthRegistrationKey,
    ) -> Result<Option<OAuthClientRegistration>> {
        Ok(self.registrations.lock().await.get(key).cloned())
    }

    async fn save_registration(
        &self,
        key: &OAuthRegistrationKey,
        value: &OAuthClientRegistration,
    ) -> Result<()> {
        self.registrations
            .lock()
            .await
            .insert(key.clone(), value.clone());
        Ok(())
    }

    async fn load_tokens(&self, key: &OAuthCredentialKey) -> Result<Option<OAuthTokenSet>> {
        Ok(self.tokens.lock().await.get(key).cloned())
    }

    async fn save_tokens(&self, key: &OAuthCredentialKey, value: &OAuthTokenSet) -> Result<()> {
        self.tokens.lock().await.insert(key.clone(), value.clone());
        Ok(())
    }

    async fn delete_tokens(&self, key: &OAuthCredentialKey) -> Result<()> {
        self.token_deletions.fetch_add(1, Ordering::SeqCst);
        self.tokens.lock().await.remove(key);
        Ok(())
    }

    async fn delete_registration(&self, key: &OAuthRegistrationKey) -> Result<()> {
        self.registrations.lock().await.remove(key);
        Ok(())
    }
}
