//! Ordered fallback with an explicit kind-based policy.

mod model;
mod policy;
mod stream;

pub use model::FallbackModel;
pub use policy::{AnotherModelCanSucceed, AnyError, FallbackPolicy};

#[cfg(any(test, feature = "test-support"))]
pub use policy::FallbackPolicyMock;
