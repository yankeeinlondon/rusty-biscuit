//! audio emission lifecycle tests.

use super::*;

/// A blocking lifecycle side effect that wedges (never returns) must not
/// be able to freeze the composition thread: `run_blocking_with_timeout`
/// has to return after roughly its budget, not after the work finishes.
/// This is the core of fix #1 — a hung TTS / sound provider between loop
/// iterations used to lock the run with no way for Ctrl+C to break in.
#[test]
fn run_blocking_with_timeout_returns_when_work_hangs() {
    use std::time::{Duration, Instant};

    let start = Instant::now();
    run_blocking_with_timeout("test-hang", Duration::from_millis(100), || {
        // Simulate a wedged audio device / network voice.
        std::thread::sleep(Duration::from_secs(30));
    });
    let elapsed = start.elapsed();

    assert!(
        elapsed < Duration::from_secs(5),
        "must abandon the wedged side effect near the 100ms budget, \
         not wait out the 30s sleep; took {elapsed:?}"
    );
}

/// The happy path must still run the work to completion and return its
/// result — bounding the wait must not turn into fire-and-forget for work
/// that finishes within budget.
#[test]
fn run_blocking_with_timeout_runs_work_to_completion_within_budget() {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::time::Duration;

    let done = Arc::new(AtomicBool::new(false));
    let done_clone = Arc::clone(&done);
    run_blocking_with_timeout("test-quick", Duration::from_secs(5), move || {
        std::thread::sleep(Duration::from_millis(20));
        done_clone.store(true, Ordering::SeqCst);
    });

    assert!(
        done.load(Ordering::SeqCst),
        "work that finishes within budget must complete before the call returns"
    );
}

#[test]
fn audio_order_say_plus_effect() {
    let n = LifecycleNotification {
        say: Some("Hello".into()),
        effect: Some("doorbell".into()),
        ..Default::default()
    };
    let phases = audio_phases(&n);
    assert_eq!(phases.len(), 2);
    assert!(matches!(phases[0], AudioPhase::Effect(_)));
    assert!(matches!(phases[1], AudioPhase::Speak(_)));
}

#[test]
fn audio_order_say_first_plus_effect() {
    let n = LifecycleNotification {
        say_first: Some("Hello".into()),
        effect: Some("doorbell".into()),
        ..Default::default()
    };
    let phases = audio_phases(&n);
    assert_eq!(phases.len(), 2);
    assert!(matches!(phases[0], AudioPhase::Speak(_)));
    assert!(matches!(phases[1], AudioPhase::Effect(_)));
}

#[test]
fn audio_order_speech_only() {
    let n = LifecycleNotification {
        say: Some("Hello".into()),
        ..Default::default()
    };
    let phases = audio_phases(&n);
    assert_eq!(phases.len(), 1);
    assert!(matches!(phases[0], AudioPhase::Speak(_)));
}

#[test]
fn audio_order_effect_only() {
    let n = LifecycleNotification {
        effect: Some("doorbell".into()),
        ..Default::default()
    };
    let phases = audio_phases(&n);
    assert_eq!(phases.len(), 1);
    assert!(matches!(phases[0], AudioPhase::Effect(_)));
}

#[test]
fn audio_order_no_audio() {
    let n = LifecycleNotification {
        stderr: Some("Status only".into()),
        ..Default::default()
    };
    let phases = audio_phases(&n);
    assert!(phases.is_empty());
}

#[test]
fn guard_non_audio_before_audio() {
    let config = parse_lifecycle_config(
        &json!({
            "start": {
                "stderr": "starting",
                "message": "msg",
                "notify": "notify-msg",
                "say": "hello",
                "effect": "confirmation",
            }
        }),
        dummy_path(),
    )
    .unwrap();
    let (settings, messaging, term) = test_ctx();
    let ctx = LifecycleRuntimeContext {
        settings: &settings,
        messaging: &messaging,
        term: &term,
        source_path: Path::new("/tmp/test.md"),
        repo_root: None,
        launch_area: None,
        context: None,
    };
    let emitter = RecordingEmitter::new();
    let mut guard = make_guard(&config, &ctx, &emitter);
    guard.emit_start_once();
    guard.defuse();

    let actions = emitter.actions();
    assert_eq!(actions.len(), 5);
    // Non-audio first
    assert!(matches!(actions[0], EmittedAction::Stderr { .. }));
    assert!(matches!(actions[1], EmittedAction::Message { .. }));
    assert!(matches!(actions[2], EmittedAction::Notification { .. }));
    // Audio: effect before say (default order)
    assert!(matches!(actions[3], EmittedAction::Effect { .. }));
    assert!(matches!(actions[4], EmittedAction::Speech { .. }));
}

#[test]
fn guard_say_first_ordering() {
    let config = parse_lifecycle_config(
        &json!({
            "start": {
                "say_first": "hello",
                "effect": "confirmation",
            }
        }),
        dummy_path(),
    )
    .unwrap();
    let (settings, messaging, term) = test_ctx();
    let ctx = LifecycleRuntimeContext {
        settings: &settings,
        messaging: &messaging,
        term: &term,
        source_path: Path::new("/tmp/test.md"),
        repo_root: None,
        launch_area: None,
        context: None,
    };
    let emitter = RecordingEmitter::new();
    let mut guard = make_guard(&config, &ctx, &emitter);
    guard.emit_start_once();
    guard.defuse();

    let actions = emitter.actions();
    assert_eq!(actions.len(), 2);
    // say_first → speech before effect
    assert!(matches!(actions[0], EmittedAction::Speech { .. }));
    assert!(matches!(actions[1], EmittedAction::Effect { .. }));
}

#[test]
#[serial_test::serial]
fn emit_signal_skips_blocking_side_effects_when_interrupted() {
    // Bug fix (2026-05-09): a Ctrl+C during a long compose run must
    // skip messenger sends, desktop notifications, TTS, and sound
    // effects so the process exits promptly. Only the cheap stderr
    // line is allowed to render so the user sees the terminal status.
    let config = parse_lifecycle_config(
        &json!({
            "failure": {
                "stderr": "failed",
                "message": "Compose run failed",
                "notify": "Compose failed",
                "say": "compose failed",
                "effect": "confirmation",
            }
        }),
        dummy_path(),
    )
    .unwrap();
    let (settings, messaging, term) = test_ctx();
    let ctx = LifecycleRuntimeContext {
        settings: &settings,
        messaging: &messaging,
        term: &term,
        source_path: Path::new("/tmp/test.md"),
        repo_root: None,
        launch_area: None,
        context: None,
    };
    let emitter = RecordingEmitter::new();
    let mut guard = make_guard(&config, &ctx, &emitter);

    crate::interrupt::clear_for_tests();
    crate::interrupt::mark_interrupted();
    guard.emit_terminal(LifecycleSignal::Failure);
    crate::interrupt::clear_for_tests();

    let actions = emitter.actions();
    assert_eq!(
        actions.len(),
        1,
        "interrupt must drop messenger/notification/TTS/effect; got: {actions:?}"
    );
    assert!(
        matches!(actions[0], EmittedAction::Stderr { .. }),
        "stderr line must still render so the user sees the terminal status"
    );
}

#[test]
#[serial_test::serial]
fn emit_signal_runs_all_side_effects_when_not_interrupted() {
    // Companion to the interrupt test: when no interrupt is observed,
    // every configured side effect still fires.
    let config = parse_lifecycle_config(
        &json!({
            "failure": {
                "stderr": "failed",
                "message": "Compose run failed",
                "notify": "Compose failed",
            }
        }),
        dummy_path(),
    )
    .unwrap();
    let (settings, messaging, term) = test_ctx();
    let ctx = LifecycleRuntimeContext {
        settings: &settings,
        messaging: &messaging,
        term: &term,
        source_path: Path::new("/tmp/test.md"),
        repo_root: None,
        launch_area: None,
        context: None,
    };
    let emitter = RecordingEmitter::new();
    let mut guard = make_guard(&config, &ctx, &emitter);

    crate::interrupt::clear_for_tests();
    guard.emit_terminal(LifecycleSignal::Failure);

    let actions = emitter.actions();
    assert!(
        actions
            .iter()
            .any(|a| matches!(a, EmittedAction::Stderr { .. }))
    );
    assert!(
        actions
            .iter()
            .any(|a| matches!(a, EmittedAction::Message { .. }))
    );
    assert!(
        actions
            .iter()
            .any(|a| matches!(a, EmittedAction::Notification { .. }))
    );
}

// =====================================================================
// notify parsing and emission (Phase 3)
// =====================================================================

#[test]
fn parses_notify_for_all_signals() {
    let fm = json!({
        "start": { "notify": "Starting" },
        "success": { "notify": "Done" },
        "blocked": { "notify": "Blocked" },
        "failure": { "notify": "Failed" }
    });
    let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();

    assert_eq!(
        config.start.as_ref().unwrap().notify.as_deref(),
        Some("Starting")
    );
    assert_eq!(
        config.success.as_ref().unwrap().notify.as_deref(),
        Some("Done")
    );
    assert_eq!(
        config.blocked.as_ref().unwrap().notify.as_deref(),
        Some("Blocked")
    );
    assert_eq!(
        config.failure.as_ref().unwrap().notify.as_deref(),
        Some("Failed")
    );
}

#[test]
fn parses_message_and_notify_independently() {
    let fm = json!({
        "start": {
            "message": "Remote message",
            "notify": "Local notification"
        }
    });
    let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
    let start = config.start.as_ref().unwrap();
    assert_eq!(start.message.as_deref(), Some("Remote message"));
    assert_eq!(start.notify.as_deref(), Some("Local notification"));
}

#[test]
fn blank_notify_is_normalized_to_none() {
    let fm = json!({
        "start": { "notify": "   " },
        "success": { "notify": "" }
    });
    let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
    assert!(config.start.as_ref().unwrap().notify.is_none());
    assert!(config.success.as_ref().unwrap().notify.is_none());
}

#[test]
fn notify_emits_without_active_route() {
    let config = parse_lifecycle_config(
        &json!({
            "start": { "notify": "Hello desktop" }
        }),
        dummy_path(),
    )
    .unwrap();
    let (settings, messaging, term) = test_ctx();
    let ctx = LifecycleRuntimeContext {
        settings: &settings,
        messaging: &messaging,
        term: &term,
        source_path: Path::new("/tmp/test.md"),
        repo_root: None,
        launch_area: None,
        context: None,
    };
    let emitter = RecordingEmitter::new();
    let mut guard = make_guard(&config, &ctx, &emitter);
    guard.emit_start_once();
    guard.defuse();

    let actions = emitter.actions();
    assert_eq!(actions.len(), 1);
    assert_eq!(
        actions[0],
        EmittedAction::Notification {
            title: "Hello desktop".to_string()
        }
    );
}

#[test]
fn notify_emits_before_audio_phases() {
    let config = parse_lifecycle_config(
        &json!({
            "start": {
                "notify": "Desktop first",
                "say": "hello",
                "effect": "confirmation",
            }
        }),
        dummy_path(),
    )
    .unwrap();
    let (settings, messaging, term) = test_ctx();
    let ctx = LifecycleRuntimeContext {
        settings: &settings,
        messaging: &messaging,
        term: &term,
        source_path: Path::new("/tmp/test.md"),
        repo_root: None,
        launch_area: None,
        context: None,
    };
    let emitter = RecordingEmitter::new();
    let mut guard = make_guard(&config, &ctx, &emitter);
    guard.emit_start_once();
    guard.defuse();

    let actions = emitter.actions();
    assert_eq!(actions.len(), 3);
    assert!(matches!(actions[0], EmittedAction::Notification { .. }));
    assert!(matches!(actions[1], EmittedAction::Effect { .. }));
    assert!(matches!(actions[2], EmittedAction::Speech { .. }));
}

#[test]
fn notify_alone_no_other_outputs() {
    let config = parse_lifecycle_config(
        &json!({
            "success": { "notify": "Only notify" }
        }),
        dummy_path(),
    )
    .unwrap();
    let (settings, messaging, term) = test_ctx();
    let ctx = LifecycleRuntimeContext {
        settings: &settings,
        messaging: &messaging,
        term: &term,
        source_path: Path::new("/tmp/test.md"),
        repo_root: None,
        launch_area: None,
        context: None,
    };
    let emitter = RecordingEmitter::new();
    let mut guard = make_guard(&config, &ctx, &emitter);
    guard.emit_terminal(LifecycleSignal::Success);

    let actions = emitter.actions();
    assert_eq!(actions.len(), 1);
    assert_eq!(
        actions[0],
        EmittedAction::Notification {
            title: "Only notify".to_string()
        }
    );
}

#[tokio::test]
async fn default_lifecycle_emitter_emit_notification_does_not_panic() {
    let emitter = DefaultLifecycleEmitter;
    // Fire-and-forget through the title-only trait method.
    emitter.emit_notification("unit testing");
    // And exercise the body-bearing path directly so the rendered
    // notification has a distinct title and message line.
    crate::messaging::execute_notification(
        "unit testing",
        Some("you can dismiss this notification"),
    );
    // Give the spawned tasks a moment to start
    tokio::task::yield_now().await;
}
