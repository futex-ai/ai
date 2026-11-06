//! Provider-neutral retry, fallback, concurrency, and usage-pricing policies.

#![warn(unreachable_pub)]

mod concurrency;
mod fallback;
mod pricing;
mod retry_config;
mod retry_random;
mod retrying;
mod sleeper;

pub use concurrency::ConcurrencyLimitedModel;
pub use fallback::{AnotherModelCanSucceed, AnyError, FallbackModel, FallbackPolicy};
pub use pricing::{ModelPricing, UsagePricingModel, price_usage};
pub use retry_config::{RetryConfig, STANDARD_TRANSIENT_RETRY_DELAYS};
pub use retry_random::{DynRetryRandom, RetryRandom, SystemRetryRandom};
pub use retrying::RetryingModel;
pub use sleeper::{DynSleeper, Sleeper, TokioSleeper};

#[cfg(any(test, feature = "test-support"))]
pub use fallback::FallbackPolicyMock;
#[cfg(any(test, feature = "test-support"))]
pub use retry_random::RetryRandomMock;
#[cfg(any(test, feature = "test-support"))]
pub use sleeper::SleeperMock;

#[cfg(test)]
#[path = "_tests_/mod.rs"]
mod tests;
