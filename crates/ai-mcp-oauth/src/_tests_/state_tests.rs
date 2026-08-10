//! Authorization state expiry, matching, and one-time use tests.

use secrecy::SecretString;

use crate::{Error, state::AuthorizationStateTracker};

#[tokio::test]
async fn accepts_matching_state_once() {
    let tracker = AuthorizationStateTracker::new();
    let expected = SecretString::from("expected".to_owned());
    let handle = tracker.begin(&expected, 100, 600).await.unwrap();

    tracker
        .consume(&handle, &expected, Some(&expected), 101)
        .await
        .unwrap();
    assert!(matches!(
        tracker
            .consume(&handle, &expected, Some(&expected), 102)
            .await,
        Err(Error::StateReused)
    ));
}

#[tokio::test]
async fn distinguishes_missing_mismatched_and_expired_state() {
    let tracker = AuthorizationStateTracker::new();
    let expected = SecretString::from("expected".to_owned());
    let wrong = SecretString::from("wrong".to_owned());

    let missing = tracker.begin(&expected, 100, 10).await.unwrap();
    assert!(matches!(
        tracker.consume(&missing, &expected, None, 101).await,
        Err(Error::StateMissing)
    ));

    let mismatch = tracker.begin(&expected, 200, 10).await.unwrap();
    assert!(matches!(
        tracker
            .consume(&mismatch, &expected, Some(&wrong), 201)
            .await,
        Err(Error::StateMismatch)
    ));

    let expired = tracker.begin(&expected, 300, 10).await.unwrap();
    assert!(matches!(
        tracker
            .consume(&expired, &expected, Some(&expected), 311)
            .await,
        Err(Error::StateExpired)
    ));
}
