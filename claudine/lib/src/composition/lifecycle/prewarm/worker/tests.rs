//! The worker: gates, non-blocking submission, serial preparation,
//! de-duplication, the failure policy, and cancellation on drop.
//!
//! Every test drives a [`FakePreparer`]; nothing reaches a speech provider.

use std::collections::BTreeSet;
use std::path::Path;
use std::sync::Arc;

use biscuit_speaks::TtsConfig;
use darkmatter::markdown::compose::ComposeContext;
use serde_json::{Map, Value, json};

use super::*;
use crate::composition::lifecycle::prewarm::fakes::FakePreparer;
use crate::composition::lifecycle::prewarm::{DocumentContext, lifecycle_phrases};
use crate::composition::lifecycle::{LifecycleConfig, parse_lifecycle_config};
use crate::events::TtsSettings;

fn lifecycle(frontmatter: &Value) -> LifecycleConfig {
    parse_lifecycle_config(frontmatter, Path::new("test.md")).expect("lifecycle parses")
}

/// Frontmatter-only inputs; `ctx`/`env` are never read by these phrases.
fn inputs(frontmatter: &Value) -> PrewarmInputs {
    let Value::Object(map) = frontmatter.clone() else {
        panic!("frontmatter is a mapping");
    };
    PrewarmInputs {
        frontmatter: map.into(),
        context: DocumentContext::Pending(ComposeContext::capture_minimal()),
        sequence_injected_keys: BTreeSet::new(),
    }
}

/// Submit every phrase of `frontmatter`'s lifecycle, returning how many were
/// newly queued.
fn submit(handle: &PrewarmHandle, frontmatter: &Value) -> usize {
    let config = lifecycle(frontmatter);
    handle.submit_phrases(&TtsConfig::new(), lifecycle_phrases(&config), &inputs(frontmatter))
}

fn texts(values: &[&str]) -> Vec<String> {
    values.iter().map(ToString::to_string).collect()
}

#[test]
fn every_gate_and_every_accepted_dry_run_spelling_disables_prewarm() {
    let cases: [(bool, bool, Option<&str>, Option<Disabled>); 14] = [
        (false, true, None, None),
        (true, true, None, Some(Disabled::DryRun)),
        (false, false, None, Some(Disabled::Setting)),
        (false, true, Some("1"), Some(Disabled::PlayaDryRun)),
        (false, true, Some("true"), Some(Disabled::PlayaDryRun)),
        (false, true, Some("TRUE"), Some(Disabled::PlayaDryRun)),
        (false, true, Some("yes"), Some(Disabled::PlayaDryRun)),
        // Only Playa's own spellings count; anything else leaves it on.
        (false, true, Some(""), None),
        (false, true, Some("0"), None),
        (false, true, Some("false"), None),
        (false, true, Some("True"), None),
        (false, true, Some("YES"), None),
        // `--dry-run` is reported first when several gates hold.
        (true, false, Some("1"), Some(Disabled::DryRun)),
        (false, false, Some("1"), Some(Disabled::Setting)),
    ];
    for (dry_run, setting, playa, expected) in cases {
        assert_eq!(
            disabled_by(dry_run, setting, playa),
            expected,
            "dry_run={dry_run} prewarm={setting} PLAYA_DRY_RUN={playa:?}"
        );
    }
}

#[tokio::test]
#[serial_test::serial]
async fn dry_run_and_prewarm_false_start_nothing_and_resolve_nothing() {
    let _live = test_toolkit::EnvGuard::remove_safe("PLAYA_DRY_RUN");
    let phrase = json!({ "title": "Widget", "success": [{ "say": "{{ title }} is done" }] });
    let off = GlobalSettings {
        tts_prewarm: false,
        ..GlobalSettings::default()
    };

    for (settings, dry_run) in [(GlobalSettings::default(), true), (off, false)] {
        let fake = FakePreparer::recording();
        let worker = PrewarmWorker::start_gated(&settings, dry_run, Arc::new(fake.clone()));
        let handle = worker.handle();

        assert!(!handle.is_enabled(), "dry_run={dry_run}");
        assert_eq!(submit(&handle, &phrase), 0, "dry_run={dry_run}");
        drop(worker);
        assert!(fake.calls().is_empty(), "dry_run={dry_run}: {:?}", fake.calls());
    }
}

#[tokio::test]
#[serial_test::serial]
async fn every_accepted_playa_dry_run_spelling_starts_nothing() {
    let phrase = json!({ "success": [{ "say": "done" }] });
    for value in ["1", "true", "TRUE", "yes"] {
        let _dry_run = test_toolkit::EnvGuard::set_safe("PLAYA_DRY_RUN", value);
        let fake = FakePreparer::recording();
        let worker =
            PrewarmWorker::start_gated(&GlobalSettings::default(), false, Arc::new(fake.clone()));

        assert!(!worker.handle().is_enabled(), "PLAYA_DRY_RUN={value}");
        assert_eq!(submit(&worker.handle(), &phrase), 0, "PLAYA_DRY_RUN={value}");
        drop(worker);
        assert!(fake.calls().is_empty(), "PLAYA_DRY_RUN={value}");
    }

    // The control: an unaccepted spelling leaves the worker running.
    let _dry_run = test_toolkit::EnvGuard::set_safe("PLAYA_DRY_RUN", "0");
    let fake = FakePreparer::recording();
    let worker =
        PrewarmWorker::start_gated(&GlobalSettings::default(), false, Arc::new(fake.clone()));
    assert_eq!(submit(&worker.handle(), &phrase), 1);
    fake.wait_for_calls(1).await;
    assert_eq!(fake.texts(), texts(&["done"]));
}

/// Explicit provider and voice, as `{ "tts": { "provider": "kokoro", "voice":
/// "af_heart" } }` bridges.
fn kokoro_settings() -> GlobalSettings {
    GlobalSettings {
        tts: Some(TtsSettings {
            provider: Some("kokoro".into()),
            voice: Some("af_heart".into()),
            rate: None,
        }),
        ..GlobalSettings::default()
    }
}

/// Submit `frontmatter`'s lifecycle as a preparation site does: gated by
/// `user`, prepared for the path that plays with `playback`.
fn submit_for(
    handle: &PrewarmHandle,
    user: &GlobalSettings,
    playback: &GlobalSettings,
    frontmatter: &Value,
) -> usize {
    let config = lifecycle(frontmatter);
    handle.submit(user, playback, lifecycle_phrases(&config), &inputs(frontmatter))
}

#[tokio::test]
#[serial_test::serial]
async fn candidates_use_the_lifecycle_playback_configuration() {
    let _live = test_toolkit::EnvGuard::remove_safe("PLAYA_DRY_RUN");
    let settings = GlobalSettings {
        tts: Some(TtsSettings {
            provider: Some("say".into()),
            voice: Some("Samantha".into()),
            rate: Some(1.25),
        }),
        ..GlobalSettings::default()
    };
    let fake = FakePreparer::recording();
    let worker = PrewarmWorker::deferred_with(false, Arc::new(fake.clone()));

    submit_for(&worker.handle(), &settings, &settings, &json!({ "success": [{ "say": "done" }] }));
    fake.wait_for_calls(1).await;

    assert_eq!(
        fake.calls()[0].config,
        tts_config_from_settings(settings.tts.as_ref()),
        "the config lifecycle `say` plays with"
    );
}

/// Two speech paths of one invocation play with different settings: each
/// candidate is prepared with its own path's configuration, the same text for
/// two configurations is two candidates, and the same text for one
/// configuration is one.
#[tokio::test]
#[serial_test::serial]
async fn each_candidate_is_prepared_with_its_own_playback_configuration() {
    let _live = test_toolkit::EnvGuard::remove_safe("PLAYA_DRY_RUN");
    let user = kokoro_settings();
    let task = GlobalSettings::default();
    let fake = FakePreparer::recording();
    let worker = PrewarmWorker::deferred_with(false, Arc::new(fake.clone()));
    let handle = worker.handle();
    let speaking = json!({ "success": [{ "say": "done" }] });

    assert_eq!(submit_for(&handle, &user, &task, &speaking), 1, "a task path first");
    assert_eq!(submit_for(&handle, &user, &user, &speaking), 1, "same text, another config");
    assert_eq!(submit_for(&handle, &user, &task, &speaking), 0, "same text and config");
    assert_eq!(submit_for(&handle.clone(), &user, &user, &speaking), 0, "same text and config");
    fake.wait_for_calls(2).await;
    drop(worker);

    let calls = fake.calls();
    assert_eq!(calls.len(), 2, "{calls:?}");
    assert_eq!(fake.texts(), texts(&["done", "done"]));
    assert_eq!(calls[0].config, tts_config_from_settings(None), "the task path's config");
    assert_eq!(
        calls[1].config,
        tts_config_from_settings(user.tts.as_ref()),
        "the user path's config"
    );
    assert_ne!(calls[0].config, calls[1].config);
}

/// The gates come from the user's settings, never from the settings a path
/// plays with: a first submission for a path that plays with defaults does
/// not turn on what `prewarm: false` turned off.
#[tokio::test]
#[serial_test::serial]
async fn prewarm_false_holds_when_the_first_submission_plays_with_default_settings() {
    let _live = test_toolkit::EnvGuard::remove_safe("PLAYA_DRY_RUN");
    let user = GlobalSettings {
        tts_prewarm: false,
        ..kokoro_settings()
    };
    let task = GlobalSettings::default();
    assert!(task.tts_prewarm, "the control: the task defaults would enable pre-warm");
    let fake = FakePreparer::recording();
    let worker = PrewarmWorker::deferred_with(false, Arc::new(fake.clone()));
    let handle = worker.handle();
    let speaking = json!({ "success": [{ "say": "done" }] });

    assert_eq!(submit_for(&handle, &user, &task, &speaking), 0);
    assert!(!handle.is_enabled());
    assert_eq!(submit_for(&handle, &user, &user, &speaking), 0);
    drop(worker);
    assert!(fake.calls().is_empty(), "{:?}", fake.calls());
}

#[tokio::test]
async fn submission_returns_while_preparation_is_blocked_and_runs_serially() {
    let fake = FakePreparer::blocking();
    let worker = PrewarmWorker::start_with(Arc::new(fake.clone()));
    let frontmatter = json!({
        "title": "Widget",
        "success": [{ "say": "{{ title }} is done" }],
        "failure": [{ "say": "{{ title }} failed: {{ err.msg }}" }],
        "finalize": [{ "say": "bye" }],
    });

    // Returns although the first preparation never finishes on its own.
    assert_eq!(submit(&worker.handle(), &frontmatter), 2, "the `err` phrase is late");
    fake.wait_for_calls(1).await;
    tokio::task::yield_now().await;
    assert_eq!(fake.texts(), texts(&["Widget is done"]), "one preparation at a time");
    assert!(fake.completed().is_empty());

    fake.release(1);
    fake.wait_for_calls(2).await;
    assert_eq!(fake.texts(), texts(&["Widget is done", "bye"]));
    fake.release(1);
}

#[tokio::test]
async fn a_candidate_is_queued_once_per_invocation() {
    let fake = FakePreparer::blocking();
    let worker = PrewarmWorker::start_with(Arc::new(fake.clone()));
    let handle = worker.handle();
    let frontmatter = json!({
        "title": "Widget",
        "success": [{ "say": "{{ title }} is done" }],
        // A different authored phrase that resolves to the same text.
        "finalize": [{ "say": "{{ fm.title }} is done" }],
    });

    assert_eq!(submit(&handle, &frontmatter), 1, "same resolved text, one candidate");
    fake.wait_for_calls(1).await;
    // Rediscovered while in progress, through another clone of the handle.
    assert_eq!(submit(&handle.clone(), &frontmatter), 0);

    fake.release(1);
    // A new resolved text after the frontmatter changed is a new candidate;
    // the completed one is still not repeated.
    let changed = json!({
        "title": "Gadget",
        "success": [{ "say": "{{ title }} is done" }],
        "finalize": [{ "say": "Widget is done" }],
    });
    assert_eq!(submit(&handle, &changed), 1);
    fake.wait_for_calls(2).await;
    fake.release(1);
    assert_eq!(fake.texts(), texts(&["Widget is done", "Gadget is done"]));
}

#[tokio::test]
#[tracing_test::traced_test]
async fn failures_warn_once_without_the_phrase_and_later_candidates_still_run() {
    let fake = FakePreparer::recording()
        .failing_on("first secret phrase")
        .failing_on("second secret phrase");
    let worker = PrewarmWorker::start_with(Arc::new(fake.clone()));
    let frontmatter = json!({
        "start": [{ "say": "first secret phrase" }],
        "success": [{ "say": "second secret phrase" }],
        "finalize": [{ "say": "fine" }],
    });

    assert_eq!(submit(&worker.handle(), &frontmatter), 3);
    fake.wait_for_completed(3).await;
    // A failed candidate is not retried in this invocation.
    assert_eq!(submit(&worker.handle(), &frontmatter), 0);
    drop(worker);

    assert_eq!(
        fake.completed(),
        texts(&["first secret phrase", "second secret phrase", "fine"])
    );
    logs_assert(|lines: &[&str]| {
        let warnings: Vec<_> = lines
            .iter()
            .filter(|line| line.contains("TTS pre-warm could not prepare"))
            .collect();
        if warnings.len() != 1 {
            return Err(format!("expected one warning, got {warnings:?}"));
        }
        if !warnings[0].contains("WARN") || warnings[0].contains("secret") {
            return Err(format!("not a sanitized warning: {}", warnings[0]));
        }
        Ok(())
    });
}

#[tokio::test]
async fn a_successful_or_no_op_preparation_counts_as_done() {
    // The recording fake's success is exactly what a provider that produces no
    // file, or an unresolvable default, returns.
    let fake = FakePreparer::recording();
    let worker = PrewarmWorker::start_with(Arc::new(fake.clone()));
    let frontmatter = json!({ "success": [{ "say": "done" }] });

    assert_eq!(submit(&worker.handle(), &frontmatter), 1);
    fake.wait_for_completed(1).await;
    drop(worker);

    assert_eq!(fake.completed(), texts(&["done"]));
    assert_eq!(fake.calls().len(), 1, "never prepared twice");
}

#[tokio::test]
async fn dropping_the_worker_cancels_without_draining_and_keeps_completed_work() {
    let fake = FakePreparer::blocking();
    let worker = PrewarmWorker::start_with(Arc::new(fake.clone()));
    let handle = worker.handle();
    let frontmatter = json!({
        "start": [{ "say": "one" }],
        "success": [{ "say": "two" }],
        "finalize": [{ "say": "three" }],
    });

    assert_eq!(submit(&handle, &frontmatter), 3);
    fake.wait_for_calls(1).await;
    fake.release(1);
    fake.wait_for_calls(2).await;

    // "two" is in flight and "three" is queued.
    drop(worker);
    fake.release(2);

    assert_eq!(fake.completed(), texts(&["one"]), "completed work survives");
    assert_eq!(fake.texts(), texts(&["one", "two"]), "the queue is not drained");
    assert!(!handle.is_enabled());
    assert_eq!(
        submit(&handle, &json!({ "success": [{ "say": "four" }] })),
        0,
        "a cancelled worker accepts nothing"
    );
    assert_eq!(fake.texts(), texts(&["one", "two"]));
}

#[test]
fn a_disabled_worker_is_a_no_op() {
    let worker = PrewarmWorker::disabled();
    let mut frontmatter = Map::new();
    frontmatter.insert("success".into(), json!([{ "say": "done" }]));

    assert!(!worker.handle().is_enabled());
    assert_eq!(submit(&worker.handle(), &Value::Object(frontmatter)), 0);
}
