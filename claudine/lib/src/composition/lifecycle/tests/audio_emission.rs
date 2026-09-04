//! audio emission lifecycle tests.

use super::*;

#[cfg(unix)]
use fs4::fs_std::FileExt as _;

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

#[cfg(unix)]
#[tokio::test]
#[serial_test::serial]
async fn default_emitter_publishes_audio_in_phase_order_without_waiting_for_playback() {
    use std::fs::{self, OpenOptions};
    use std::os::unix::fs::PermissionsExt as _;
    use std::time::{Duration, Instant};

    let temp = tempfile::tempdir().unwrap();
    let spool = temp.path().join("spool");
    let bin = temp.path().join("bin");
    fs::create_dir(&bin).unwrap();
    let espeak = bin.join("espeak");
    fs::write(&espeak, "#!/bin/sh\n/bin/sleep 5\n").unwrap();
    let mut permissions = fs::metadata(&espeak).unwrap().permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&espeak, permissions).unwrap();

    let path = std::env::join_paths(
        std::iter::once(bin.clone()).chain(std::env::split_paths(
            &std::env::var_os("PATH").unwrap_or_default(),
        )),
    )
    .unwrap();
    let _path = test_toolkit::EnvGuard::set_safe("PATH", path);
    let _spool = test_toolkit::EnvGuard::set_safe("PLAYA_SPOOL_DIR", &spool);
    let _dry_run = test_toolkit::EnvGuard::remove_safe("PLAYA_DRY_RUN");
    assert_eq!(biscuit_speaks::run_if_worker().await, None);

    fs::create_dir(&spool).unwrap();
    fs::set_permissions(&spool, fs::Permissions::from_mode(0o700)).unwrap();
    let worker = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(spool.join("worker.lock"))
        .unwrap();
    worker.lock_exclusive().unwrap();

    let settings = GlobalSettings {
        tts: Some(TtsSettings {
            provider: Some("espeak".to_string()),
            voice: None,
            rate: None,
        }),
        ..GlobalSettings::default()
    };
    let messaging = RuntimeMessagingSettings::default();
    let term = Terminal::default();
    let ctx = LifecycleRuntimeContext {
        settings: &settings,
        messaging: &messaging,
        term: &term,
        source_path: Path::new("/tmp/test.md"),
        repo_root: None,
        launch_area: None,
        context: None,
    };
    let emitter = DefaultLifecycleEmitter;
    let start = Instant::now();

    for frontmatter in [
        json!({"start": {"say": "Phase 1 of the plan in the claudine package area, was implemented successfully", "effect": "doorbell-2"}}),
        json!({"start": {"say_first": "Phase 1 of the plan in the claudine package area, was implemented successfully", "effect": "doorbell-2"}}),
    ] {
        let config = parse_lifecycle_config(&frontmatter, dummy_path()).unwrap();
        let mut guard = LifecycleRunGuard::new(&config, &ctx, &emitter);
        guard.emit_start_once();
        guard.defuse();
    }

    assert!(
        start.elapsed() < Duration::from_secs(4),
        "lifecycle emission must return after durable publication"
    );
    let snapshot = playa::detached::snapshot().unwrap();
    assert_eq!(
        snapshot
            .pending
            .iter()
            .map(|job| job.source_kind)
            .collect::<Vec<_>>(),
        vec![
            playa::detached::JournalSourceKind::File,
            playa::detached::JournalSourceKind::Command,
            playa::detached::JournalSourceKind::Command,
            playa::detached::JournalSourceKind::File,
        ]
    );
    assert_eq!(
        snapshot
            .pending
            .iter()
            .map(|job| job.sequence)
            .collect::<Vec<_>>(),
        vec![1, 2, 3, 4]
    );
}

#[tokio::test]
#[serial_test::serial]
#[tracing_test::traced_test]
async fn default_emitter_warns_once_when_effect_handoff_fails() {
    let temp = tempfile::tempdir().unwrap();
    let not_a_directory = temp.path().join("not-a-directory");
    std::fs::write(&not_a_directory, b"file").unwrap();
    let _spool = test_toolkit::EnvGuard::set_safe("PLAYA_SPOOL_DIR", &not_a_directory);
    let _dry_run = test_toolkit::EnvGuard::remove_safe("PLAYA_DRY_RUN");
    assert_eq!(biscuit_speaks::run_if_worker().await, None);

    DefaultLifecycleEmitter.emit_effect("doorbell-2");

    logs_assert(|logs| {
        let warnings = logs
            .iter()
            .filter(|line| line.contains("Lifecycle sound effect handoff failed"))
            .count();
        assert_eq!(warnings, 1, "expected one handoff warning, got: {logs:?}");
        Ok(())
    });
}

#[cfg(unix)]
#[tokio::test]
#[serial_test::serial]
#[tracing_test::traced_test]
async fn default_emitter_warns_once_when_speech_handoff_fails() {
    use std::fs;
    use std::os::unix::fs::PermissionsExt as _;

    let temp = tempfile::tempdir().unwrap();
    let bin = temp.path().join("bin");
    fs::create_dir(&bin).unwrap();
    let espeak = bin.join("espeak");
    fs::write(&espeak, "#!/bin/sh\nexit 0\n").unwrap();
    let mut permissions = fs::metadata(&espeak).unwrap().permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&espeak, permissions).unwrap();
    let path = std::env::join_paths(
        std::iter::once(bin).chain(std::env::split_paths(
            &std::env::var_os("PATH").unwrap_or_default(),
        )),
    )
    .unwrap();
    let not_a_directory = temp.path().join("not-a-directory");
    fs::write(&not_a_directory, b"file").unwrap();
    let _path = test_toolkit::EnvGuard::set_safe("PATH", path);
    let _spool = test_toolkit::EnvGuard::set_safe("PLAYA_SPOOL_DIR", &not_a_directory);
    let _dry_run = test_toolkit::EnvGuard::remove_safe("PLAYA_DRY_RUN");
    assert_eq!(biscuit_speaks::run_if_worker().await, None);

    let config = TtsConfig::new().with_failover(TtsFailoverStrategy::SpecificProvider(
        biscuit_speaks::TtsProvider::Host(biscuit_speaks::HostTtsProvider::ESpeak),
    ));
    DefaultLifecycleEmitter.emit_speech(
        "Phase 1 of the plan in the claudine package area, was implemented successfully",
        config,
    );

    logs_assert(|logs| {
        let warnings = logs
            .iter()
            .filter(|line| line.contains("Lifecycle TTS handoff failed"))
            .count();
        assert_eq!(warnings, 1, "expected one handoff warning, got: {logs:?}");
        Ok(())
    });
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
