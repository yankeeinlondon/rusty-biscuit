//! The [`Preparer`] seam's fake honors the contract later worker tests rely on.

use std::sync::Arc;

use biscuit_speaks::TtsConfig;

use super::fakes::{FakePreparer, PrepareCall};
use super::{Preparer, PrewarmError};

#[tokio::test]
async fn recording_fake_records_text_and_config_and_succeeds() {
    let fake = FakePreparer::recording();
    let preparer: Arc<dyn Preparer> = Arc::new(fake.clone());
    let config = TtsConfig::new().with_voice("af_heart");

    let result = preparer.prepare("build is done".into(), config.clone()).await;

    assert_eq!(result, Ok(()));
    assert_eq!(
        fake.calls(),
        vec![PrepareCall { text: "build is done".into(), config }]
    );
    assert_eq!(fake.completed(), vec!["build is done".to_string()]);
}

#[tokio::test]
async fn blocking_fake_records_the_call_and_holds_completion_until_released() {
    let fake = FakePreparer::blocking();
    let preparer: Arc<dyn Preparer> = Arc::new(fake.clone());

    let task = tokio::spawn(preparer.prepare("held".into(), TtsConfig::new()));
    fake.wait_for_calls(1).await;
    tokio::task::yield_now().await;

    assert_eq!(fake.texts(), vec!["held".to_string()]);
    assert!(fake.completed().is_empty(), "a blocked call has not completed");

    fake.release(1);
    assert_eq!(task.await.expect("task joins"), Ok(()));
    assert_eq!(fake.completed(), vec!["held".to_string()]);
}

#[tokio::test]
async fn dropping_a_blocked_preparation_never_completes_it() {
    let fake = FakePreparer::blocking();
    let task = tokio::spawn(fake.prepare("cancelled".into(), TtsConfig::new()));
    fake.wait_for_calls(1).await;

    task.abort();
    assert!(task.await.expect_err("aborted").is_cancelled());
    fake.release(1);
    tokio::task::yield_now().await;

    assert_eq!(fake.texts(), vec!["cancelled".to_string()]);
    assert!(fake.completed().is_empty());
}

#[tokio::test]
async fn failing_fake_fails_only_the_named_text() {
    let fake = FakePreparer::recording().failing_on("bad");

    assert_eq!(
        fake.prepare("bad".into(), TtsConfig::new()).await,
        Err(PrewarmError::Prepare)
    );
    assert_eq!(fake.prepare("good".into(), TtsConfig::new()).await, Ok(()));
    assert_eq!(fake.texts(), vec!["bad".to_string(), "good".to_string()]);
    assert_eq!(
        fake.completed(),
        vec!["bad".to_string(), "good".to_string()]
    );
}

