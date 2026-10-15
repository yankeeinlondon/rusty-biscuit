//! The [`Preparer`] seam's fake honors the contract later worker tests rely on,
//! and the entry point every preparation site calls.

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


/// The entry point every preparation site calls, and the deferred worker it
/// starts: one worker, one de-duplication, for the whole invocation.
mod entry {
    use std::cell::Cell;
    use std::path::Path;

    use darkmatter::markdown::compose::ComposeContext;
    use serde_json::{Value, json};

    use super::super::{PrewarmInputs, PrewarmWorker, collect_and_submit};
    use super::*;
    use crate::composition::lifecycle::{LifecycleConfig, parse_lifecycle_config};
    use crate::events::GlobalSettings;

    fn lifecycle(frontmatter: &Value) -> LifecycleConfig {
        parse_lifecycle_config(frontmatter, Path::new("test.md")).expect("lifecycle parses")
    }

    fn inputs(frontmatter: &Value) -> PrewarmInputs {
        PrewarmInputs::captured(frontmatter, ComposeContext::capture_minimal())
    }

    /// Submit one prepared document's snapshot, as a preparation site does.
    fn prepare(handle: &super::super::PrewarmHandle, frontmatter: &Value) -> usize {
        collect_and_submit(handle, &GlobalSettings::default(), &lifecycle(frontmatter), || {
            inputs(frontmatter)
        })
    }

    fn texts(values: &[&str]) -> Vec<String> {
        values.iter().map(ToString::to_string).collect()
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn a_deferred_worker_starts_with_the_first_submission_that_speaks() {
        let _dry_run = test_toolkit::EnvGuard::set_safe("PLAYA_DRY_RUN", "0");
        let fake = FakePreparer::recording();
        let worker = PrewarmWorker::deferred_with(false, Arc::new(fake.clone()));
        let handle = worker.handle();

        // A silent document resolves nothing and decides nothing.
        let built = Cell::new(false);
        let queued = collect_and_submit(
            &handle,
            &GlobalSettings { tts_prewarm: false, ..GlobalSettings::default() },
            &lifecycle(&json!({ "success": [{ "shell": "true" }] })),
            || {
                built.set(true);
                inputs(&json!({}))
            },
        );
        assert_eq!(queued, 0);
        assert!(!built.get(), "inputs are only built for a document that speaks");
        assert!(handle.is_enabled(), "a silent document does not decide the gates");

        assert_eq!(prepare(&handle, &json!({ "success": [{ "say": "done" }] })), 1);
        fake.wait_for_calls(1).await;
        assert_eq!(fake.texts(), texts(&["done"]));
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn the_first_speaking_submission_decides_the_gates() {
        let _dry_run = test_toolkit::EnvGuard::set_safe("PLAYA_DRY_RUN", "0");
        let fake = FakePreparer::recording();
        let worker = PrewarmWorker::deferred_with(false, Arc::new(fake.clone()));
        let handle = worker.handle();
        let speaking = json!({ "success": [{ "say": "done" }] });

        let off = GlobalSettings { tts_prewarm: false, ..GlobalSettings::default() };
        let queued = collect_and_submit(&handle, &off, &lifecycle(&speaking), || inputs(&speaking));
        assert_eq!(queued, 0, "`prewarm: false` prepares nothing");
        assert!(!handle.is_enabled());

        // A later document cannot turn an invocation's pre-warm back on.
        assert_eq!(prepare(&handle, &speaking), 0);
        drop(worker);
        assert!(fake.calls().is_empty());
    }

    #[test]
    fn dry_run_turns_a_deferred_worker_off_before_any_submission() {
        let fake = FakePreparer::recording();
        let worker = PrewarmWorker::deferred_with(true, Arc::new(fake.clone()));
        let handle = worker.handle();
        assert!(!handle.is_enabled());

        let speaking = json!({ "success": [{ "say": "done" }] });
        let queued = collect_and_submit(
            &handle,
            &GlobalSettings::default(),
            &lifecycle(&speaking),
            || panic!("`--dry-run` resolves nothing"),
        );
        assert_eq!(queued, 0);
        drop(worker);
        assert!(fake.calls().is_empty());
    }

    /// Initialization rewrote the title between the bootstrap read and the
    /// stabilized reread: the new text is found, the unchanged phrase is not
    /// queued again. Retry, resume, loop refresh, and a `proxy` target submit
    /// through the same handle, so an identical snapshot queues nothing.
    #[tokio::test]
    #[serial_test::serial]
    async fn bootstrap_and_reread_share_one_worker_without_repeating_a_candidate() {
        let _dry_run = test_toolkit::EnvGuard::set_safe("PLAYA_DRY_RUN", "0");
        let fake = FakePreparer::recording();
        let worker = PrewarmWorker::deferred_with(false, Arc::new(fake.clone()));
        let handle = worker.handle();
        let snapshot = |title: &str| {
            json!({
                "title": title,
                "success": [{ "say": "{{ title }} is done" }],
                "finalize": [{ "say": "bye" }],
            })
        };

        assert_eq!(prepare(&handle, &snapshot("Draft")), 2, "bootstrap");
        assert_eq!(prepare(&handle, &snapshot("Final")), 1, "stabilized reread");
        for reentry in ["retry", "loop refresh", "proxy target"] {
            assert_eq!(prepare(&handle.clone(), &snapshot("Final")), 0, "{reentry}");
        }
        fake.wait_for_calls(3).await;
        assert_eq!(fake.texts(), texts(&["Draft is done", "bye", "Final is done"]));
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn sequence_injected_keys_are_late_in_every_document_their_handle_submits() {
        let _dry_run = test_toolkit::EnvGuard::set_safe("PLAYA_DRY_RUN", "0");
        let fake = FakePreparer::recording();
        let worker = PrewarmWorker::deferred_with(false, Arc::new(fake.clone()));
        // As a sequence step scopes it: the per-step overlay plus its inputs.
        let step = worker.handle().with_injected_keys(
            crate::composition::sequence::reserved::ROOT_OVERLAY_KEYS
                .iter()
                .map(|key| (*key).to_string())
                .chain(["topic".to_string()]),
        );
        let document = json!({
            "title": "Report",
            "topic": "per step",
            "key_name": "title",
            "success": [
                { "say": "{{ title }} is done" },
                { "say": "bare {{ topic }}" },
                { "say": "dotted {{ fm.topic }}" },
                { "say": "bracket {{ fm['topic'] }}" },
                { "say": "computed {{ fm[key_name] }}" },
                { "say": "output {{ seq.outputs.summary }}" },
                { "say": "iteration {{ loop.count }}" },
                { "say": "error {{ err.msg }}" },
            ],
        });

        assert_eq!(prepare(&step, &document), 1);
        fake.wait_for_calls(1).await;
        assert_eq!(fake.texts(), texts(&["Report is done"]));

        // The control: through a handle the sequence did not scope, the same
        // reads are early, so the scoping is what made them late.
        let unscoped = worker.handle();
        assert_eq!(prepare(&unscoped, &document), 4);
        fake.wait_for_calls(5).await;
        assert_eq!(
            fake.texts(),
            texts(&[
                "Report is done",
                "bare per step",
                "dotted per step",
                "bracket per step",
                "computed Report",
            ])
        );
    }

    /// Parallel sequence tasks scope their own handles, yet share the one
    /// invocation-wide de-duplication.
    #[tokio::test]
    #[serial_test::serial]
    async fn differently_scoped_tasks_share_one_de_duplication() {
        let _dry_run = test_toolkit::EnvGuard::set_safe("PLAYA_DRY_RUN", "0");
        let fake = FakePreparer::recording();
        let worker = PrewarmWorker::deferred_with(false, Arc::new(fake.clone()));
        let first = worker.handle().with_injected_keys(["alpha".to_string()]);
        let second = worker.handle().with_injected_keys(["beta".to_string()]);
        let document = json!({ "failure": [{ "say": "task failed" }] });

        assert_eq!(prepare(&first, &document), 1);
        assert_eq!(prepare(&second, &document), 0);
        fake.wait_for_calls(1).await;
        assert_eq!(fake.texts(), texts(&["task failed"]));
    }
}
