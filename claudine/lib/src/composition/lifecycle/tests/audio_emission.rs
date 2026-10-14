//! audio emission lifecycle tests.

use super::*;

use test_toolkit::LockedAudioSpool;

/// Place an empty executable named `<name>` in `<temp>/bin` so a `PATH` lookup
/// finds it on every OS. The eSpeak readiness these tests exercise is a
/// presence check, and neither test lets playback reach the program, so the
/// file is never run.
fn install_fixture_program(temp: &Path, name: &str) -> std::path::PathBuf {
    let bin = temp.join("bin");
    std::fs::create_dir(&bin).unwrap();
    let program = bin.join(format!("{name}{}", std::env::consts::EXE_SUFFIX));
    std::fs::write(&program, b"").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    bin
}

#[tokio::test]
#[serial_test::serial]
async fn default_emitter_publishes_audio_in_stack_order_without_waiting_for_playback() {
    use std::time::{Duration, Instant};

    let temp = tempfile::tempdir().unwrap();
    let spool = temp.path().join("spool");
    let bin = install_fixture_program(temp.path(), "espeak");

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


    let _audio_spool = LockedAudioSpool::new(&spool);

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

    // A dictionary plays its effect before speech; a list keeps its order.
    for frontmatter in [
        json!({"start": {"say": "This is a test message.", "effect": "doorbell-2"}}),
        json!({"start": [{"say": "This is a test message."}, {"effect": "doorbell-2"}]}),
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

#[tokio::test]
#[serial_test::serial]
#[tracing_test::traced_test]
async fn default_emitter_warns_once_when_speech_handoff_fails() {
    use std::fs;

    let temp = tempfile::tempdir().unwrap();
    let bin = install_fixture_program(temp.path(), "espeak");
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
        "This is a test message.",
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

/// Run `signal`'s stack through the executor with a recording emitter.
fn execute(
    frontmatter: serde_json::Value,
    signal: LifecycleSignal,
) -> (Vec<EmittedAction>, executor::LifecycleEventOutcome) {
    let emitter = RecordingEmitter::new();
    let outcome = execute_with(frontmatter, signal, &emitter, &GlobalSettings::default());
    (emitter.actions(), outcome)
}

/// Run `signal`'s stack through the executor with `emitter` and `settings`.
fn execute_with(
    frontmatter: serde_json::Value,
    signal: LifecycleSignal,
    emitter: &dyn LifecycleEmitter,
    settings: &GlobalSettings,
) -> executor::LifecycleEventOutcome {
    let config = parse_lifecycle_config(&frontmatter, dummy_path()).unwrap();
    let (_, messaging, term) = test_ctx();
    let engine_root = tempfile::tempdir().unwrap();
    let engine = darkmatter::effects::EffectEngine::builder()
        .mutation_root(engine_root.path())
        .auto_rehash(false)
        .build();
    let fm = serde_json::Map::new();
    let context = executor::StackExecutionContext {
        signal,
        scope: None,
        frontmatter: &fm,
        live_frontmatter: None,
        runtime_state: None,
        err: None,
        timing: None,
        current: None,
        group: None,
        loop_values: None,
        base_dir: None,
        ctx_base_dir: None,
        prepared_context: None,
        file_resolution_context: crate::test_support::process_context(),
        effect_engine: &engine,
        shell_runner: &executor::DisabledShellRunner,
        emitter,
        term: &term,
        source_path: dummy_path(),
        repo_root: None,
        messaging: &messaging,
        settings,
    };
    context.execute_event(&config)
}

/// A failure stack whose durable records spell its path: an effect, a guarded
/// branch whose `else` (selected) speaks and whose `then` would play two
/// effects, a trailing effect, then a `retry` transition.
fn branching_audio_stack() -> serde_json::Value {
    json!({ "failure": [
        { "effect": "doorbell-2" },
        {
            "when": "false",
            "then": [{ "effect": "confirmation" }, { "effect": "confirmation" }],
            "else": [{ "say": "This is a test message." }],
        },
        { "effect": "confirmation" },
        "retry",
    ]})
}

/// Durable publication follows stack order across a guarded branch, only the
/// selected branch publishes, and records published before a `retry`
/// transition stay queued after it. The spool lock keeps playback from
/// running, and the eSpeak stub is never executed.
#[tokio::test]
#[serial_test::serial]
async fn executor_durably_queues_the_selected_branch_in_stack_order_across_a_transition() {
    let temp = tempfile::tempdir().unwrap();
    let spool = temp.path().join("spool");
    let bin = install_fixture_program(temp.path(), "espeak");
    let path = std::env::join_paths(
        std::iter::once(bin).chain(std::env::split_paths(
            &std::env::var_os("PATH").unwrap_or_default(),
        )),
    )
    .unwrap();
    let _path = test_toolkit::EnvGuard::set_safe("PATH", path);
    let _spool = test_toolkit::EnvGuard::set_safe("PLAYA_SPOOL_DIR", &spool);
    let _dry_run = test_toolkit::EnvGuard::remove_safe("PLAYA_DRY_RUN");
    assert_eq!(biscuit_speaks::run_if_worker().await, None);
    let _audio_spool = LockedAudioSpool::new(&spool);
    let settings = GlobalSettings {
        tts: Some(TtsSettings {
            provider: Some("espeak".to_string()),
            voice: None,
            rate: None,
        }),
        ..GlobalSettings::default()
    };

    let outcome = execute_with(
        branching_audio_stack(),
        LifecycleSignal::Failure,
        &DefaultLifecycleEmitter,
        &settings,
    );

    assert!(
        matches!(outcome.control, Some(executor::StackControl::Retry { .. })),
        "{outcome:?}"
    );
    let pending = playa::detached::snapshot().unwrap().pending;
    use playa::detached::JournalSourceKind::{Command, File};
    assert_eq!(
        pending.iter().map(|job| job.source_kind).collect::<Vec<_>>(),
        vec![File, Command, File],
        "effect, the selected `else` speech, effect: {pending:?}"
    );
    assert_eq!(
        pending.iter().map(|job| job.sequence).collect::<Vec<_>>(),
        vec![1, 2, 3]
    );
}

/// Under `PLAYA_DRY_RUN` the same stack publishes nothing: no spool is
/// created and no TTS preparation runs.
#[tokio::test]
#[serial_test::serial]
async fn executor_publishes_no_audio_under_dry_run() {
    let temp = tempfile::tempdir().unwrap();
    let spool = temp.path().join("spool");
    let _spool = test_toolkit::EnvGuard::set_safe("PLAYA_SPOOL_DIR", &spool);
    let _dry_run = test_toolkit::EnvGuard::set_safe("PLAYA_DRY_RUN", "1");
    assert_eq!(biscuit_speaks::run_if_worker().await, None);

    let outcome = execute_with(
        branching_audio_stack(),
        LifecycleSignal::Failure,
        &DefaultLifecycleEmitter,
        &GlobalSettings::default(),
    );

    assert!(
        matches!(outcome.control, Some(executor::StackControl::Retry { .. })),
        "{outcome:?}"
    );
    assert!(!spool.exists(), "dry run must not create or fill the spool");
}

fn effect(name: &str) -> EmittedAction {
    EmittedAction::Effect {
        name: name.to_string(),
    }
}

fn speech(text: &str) -> EmittedAction {
    EmittedAction::Speech {
        text: text.to_string(),
    }
}

#[test]
fn executor_publishes_an_effect_then_speech_in_stack_order() {
    let (actions, outcome) = execute(
        json!({ "success": [{ "effect": "confirmation" }, { "say": "done" }] }),
        LifecycleSignal::Success,
    );
    assert_eq!(actions, [effect("confirmation"), speech("done")]);
    assert_eq!(outcome, executor::LifecycleEventOutcome::default());
}

#[test]
fn executor_publishes_speech_then_an_effect_in_stack_order() {
    let (actions, outcome) = execute(
        json!({ "success": [{ "say": "done" }, { "effect": "confirmation" }] }),
        LifecycleSignal::Success,
    );
    assert_eq!(actions, [speech("done"), effect("confirmation")]);
    assert_eq!(outcome, executor::LifecycleEventOutcome::default());
}

#[test]
fn executor_interleaves_audio_with_other_actions_and_branches_in_order() {
    let (actions, _) = execute(
        json!({ "failure": [
            { "say": "first" },
            { "stderr": "between" },
            { "when": "true", "then": [{ "effect": "crowd-applause" }, { "speak": "inside" }] },
            { "effect": "confirmation" },
        ]}),
        LifecycleSignal::Failure,
    );
    assert_eq!(
        actions,
        [
            speech("first"),
            EmittedAction::Stderr {
                signal: LifecycleSignal::Failure,
                text: "between".to_string(),
            },
            effect("crowd-applause"),
            speech("inside"),
            effect("confirmation"),
        ]
    );
}

#[test]
fn audio_before_a_directive_is_published_before_the_transition() {
    let (actions, outcome) = execute(
        json!({ "failure": [{ "say": "retrying" }, { "effect": "confirmation" }, "retry"] }),
        LifecycleSignal::Failure,
    );
    assert_eq!(actions, [speech("retrying"), effect("confirmation")]);
    assert!(
        matches!(outcome.control, Some(executor::StackControl::Retry { .. })),
        "{outcome:?}"
    );
}

#[test]
fn guard_emits_a_dictionary_in_canonical_order() {
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
    // A dictionary expands in canonical order: communication lines, then
    // the effect before speech.
    assert!(matches!(actions[0], EmittedAction::Stderr { .. }));
    assert!(matches!(actions[1], EmittedAction::Message { .. }));
    assert!(matches!(actions[2], EmittedAction::Notification { .. }));
    assert!(matches!(actions[3], EmittedAction::Effect { .. }));
    assert!(matches!(actions[4], EmittedAction::Speech { .. }));
}

#[test]
fn guard_emits_a_list_in_authored_order() {
    let config = parse_lifecycle_config(
        &json!({ "start": [{ "say": "hello" }, { "effect": "confirmation" }] }),
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

    for (signal, title) in [
        (LifecycleSignal::Start, "Starting"),
        (LifecycleSignal::Success, "Done"),
        (LifecycleSignal::Blocked, "Blocked"),
        (LifecycleSignal::Failure, "Failed"),
    ] {
        assert_eq!(literal_communications(&config, signal), [("notify", title.to_string())]);
    }
}

/// The `(verb, literal text)` of every root-level communication action.
fn literal_communications(config: &LifecycleConfig, signal: LifecycleSignal) -> Vec<(&'static str, String)> {
    config
        .stack(signal)
        .iter()
        .filter_map(|node| match node {
            actions::LifecycleNode::Action(LifecycleAction {
                kind: LifecycleActionKind::Communication(comm),
                ..
            }) => match &comm.text {
                Expr::StringLiteral(text) => Some((comm.channel.verb(), text.clone())),
                _ => None,
            },
            _ => None,
        })
        .collect()
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
    assert_eq!(
        literal_communications(&config, LifecycleSignal::Start),
        [
            ("message", "Remote message".to_string()),
            ("notify", "Local notification".to_string()),
        ]
    );
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
fn notify_emits_before_audio_in_a_dictionary() {
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
