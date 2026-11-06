//! Caller-selected rules for moving to another model.

use ai_interface::ErrorKind;

#[cfg_attr(any(test, feature = "test-support"), unimock::unimock(api = FallbackPolicyMock))]
/// Decides whether a failed kind allows another model attempt.
pub trait FallbackPolicy: Send + Sync {
    /// Returns whether the chain can move on after this kind.
    fn should_fallback(&self, kind: ErrorKind) -> bool;
}

#[derive(Clone, Copy, Debug, Default)]
/// Moves on after every failure, including interrupted generation.
pub struct AnyError;

impl FallbackPolicy for AnyError {
    fn should_fallback(&self, _kind: ErrorKind) -> bool {
        true
    }
}

#[derive(Clone, Copy, Debug, Default)]
/// Moves on when the shared kind says another model can succeed.
pub struct AnotherModelCanSucceed;

impl FallbackPolicy for AnotherModelCanSucceed {
    fn should_fallback(&self, kind: ErrorKind) -> bool {
        kind.can_succeed_with_another_model()
    }
}
