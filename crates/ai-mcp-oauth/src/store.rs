//! Host-owned secure credential persistence boundary.

use std::sync::Arc;

use async_trait::async_trait;

use crate::{
    OAuthClientRegistration, OAuthCredentialKey, OAuthRegistrationKey, OAuthTokenSet, Result,
};

/// Shared secure OAuth credential store.
pub type DynOAuthCredentialStore = Arc<dyn OAuthCredentialStore>;

#[cfg_attr(
    any(test, doctest, feature = "test-support"),
    unimock::unimock(api = OAuthCredentialStoreMock)
)]
#[async_trait]
/// Persists registrations and token sets atomically in host-controlled storage.
pub trait OAuthCredentialStore: Send + Sync {
    /// Loads one cached dynamic client registration.
    async fn load_registration(
        &self,
        key: &OAuthRegistrationKey,
    ) -> Result<Option<OAuthClientRegistration>>;

    /// Atomically saves one dynamic client registration.
    async fn save_registration(
        &self,
        key: &OAuthRegistrationKey,
        value: &OAuthClientRegistration,
    ) -> Result<()>;

    /// Loads one resource-bound token set.
    async fn load_tokens(&self, key: &OAuthCredentialKey) -> Result<Option<OAuthTokenSet>>;

    /// Atomically replaces one resource-bound token set.
    async fn save_tokens(&self, key: &OAuthCredentialKey, value: &OAuthTokenSet) -> Result<()>;

    /// Deletes one resource-bound token set.
    async fn delete_tokens(&self, key: &OAuthCredentialKey) -> Result<()>;

    /// Deletes one locally cached dynamic registration.
    async fn delete_registration(&self, key: &OAuthRegistrationKey) -> Result<()>;
}
