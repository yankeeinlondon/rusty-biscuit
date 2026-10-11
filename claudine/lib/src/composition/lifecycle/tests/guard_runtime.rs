//! guard runtime lifecycle tests.

use super::*;

#[test]
#[allow(deprecated)]
fn lifecycle_runtime_state_defaults() {
    let state = LifecycleRuntimeState::default();
    assert!(!state.start_emitted);
    assert!(!state.provider_launch_started);
}

#[test]
fn guard_emits_start_once() {
    let config = test_config();
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
    guard.emit_start_once(); // second call is idempotent
    guard.defuse();

    assert_eq!(emitter.signals(), vec![LifecycleSignal::Start]);
}

#[test]
fn guard_drop_emits_blocked_before_launch() {
    let config = test_config();
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

    {
        let mut guard = make_guard(&config, &ctx, &emitter);
        guard.emit_start_once();
        // drop without terminal signal, not launched
    }

    assert_eq!(
        emitter.signals(),
        vec![LifecycleSignal::Start, LifecycleSignal::Blocked]
    );
}

#[test]
fn guard_drop_emits_failure_after_launch() {
    let config = test_config();
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

    {
        let mut guard = make_guard(&config, &ctx, &emitter);
        guard.emit_start_once();
        guard.mark_provider_launched();
        // drop without terminal signal, but launched
    }

    assert_eq!(
        emitter.signals(),
        vec![LifecycleSignal::Start, LifecycleSignal::Failure]
    );
}

#[test]
fn guard_drop_silent_without_start() {
    let config = test_config();
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

    {
        let _guard = make_guard(&config, &ctx, &emitter);
        // drop without ever emitting start
    }

    assert!(emitter.signals().is_empty());
}

#[test]
fn guard_emit_terminal_prevents_drop_emission() {
    let config = test_config();
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

    {
        let mut guard = make_guard(&config, &ctx, &emitter);
        guard.emit_start_once();
        guard.mark_provider_launched();
        guard.emit_terminal(LifecycleSignal::Success);
        // drop after explicit terminal — no double emission
    }

    assert_eq!(
        emitter.signals(),
        vec![LifecycleSignal::Start, LifecycleSignal::Success]
    );
}

#[test]
fn guard_defuse_prevents_drop_emission() {
    let config = test_config();
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

    {
        let mut guard = make_guard(&config, &ctx, &emitter);
        guard.emit_start_once();
        guard.defuse();
    }

    // Only start, no terminal from Drop
    assert_eq!(emitter.signals(), vec![LifecycleSignal::Start]);
}

#[test]
fn guard_emit_blocked_or_failure_pre_launch() {
    let config = test_config();
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

    {
        let mut guard = make_guard(&config, &ctx, &emitter);
        guard.emit_start_once();
        guard.emit_blocked_or_failure(); // pre-launch → Blocked
    }

    assert_eq!(
        emitter.signals(),
        vec![LifecycleSignal::Start, LifecycleSignal::Blocked]
    );
}

#[test]
fn guard_emit_blocked_or_failure_post_launch() {
    let config = test_config();
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

    {
        let mut guard = make_guard(&config, &ctx, &emitter);
        guard.emit_start_once();
        guard.mark_provider_launched();
        guard.emit_blocked_or_failure(); // post-launch → Failure
    }

    assert_eq!(
        emitter.signals(),
        vec![LifecycleSignal::Start, LifecycleSignal::Failure]
    );
}

#[test]
fn guard_state_accessors() {
    let config = test_config();
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

    assert!(!guard.start_emitted());
    assert!(!guard.provider_launched());

    guard.emit_start_once();
    assert!(guard.start_emitted());
    assert!(!guard.provider_launched());

    guard.mark_provider_launched();
    assert!(guard.provider_launched());

    guard.defuse();
}

#[test]
fn record_event_emission_tracks_state_and_prevents_double_emission() {
    let config = test_config();
    let (settings, messaging, term) = test_ctx();
    let emitter = RecordingEmitter::new();
    let ctx = LifecycleRuntimeContext {
        settings: &settings,
        messaging: &messaging,
        term: &term,
        source_path: dummy_path(),
        repo_root: None,
        launch_area: None,
        context: None,
    };
    let mut guard = LifecycleRunGuard::new(&config,
        &ctx,
        &emitter,
    );

    assert!(guard.record_event_emission(LifecycleSignal::Initialize));
    assert!(!guard.record_event_emission(LifecycleSignal::Initialize));

    assert!(guard.record_event_emission(LifecycleSignal::Start));
    assert!(!guard.record_event_emission(LifecycleSignal::Start));

    assert!(guard.record_event_emission(LifecycleSignal::Success));
    assert!(!guard.record_event_emission(LifecycleSignal::Success));
    assert!(!guard.record_event_emission(LifecycleSignal::Blocked));
    assert!(!guard.record_event_emission(LifecycleSignal::Failure));

    assert!(guard.record_event_emission(LifecycleSignal::Finalize));
    assert!(!guard.record_event_emission(LifecycleSignal::Finalize));
}

#[test]
fn finalize_cannot_emit_without_terminal() {
    let config = test_config();
    let (settings, messaging, term) = test_ctx();
    let emitter = RecordingEmitter::new();
    let ctx = LifecycleRuntimeContext {
        settings: &settings,
        messaging: &messaging,
        term: &term,
        source_path: dummy_path(),
        repo_root: None,
        launch_area: None,
        context: None,
    };
    let mut guard = LifecycleRunGuard::new(&config,
        &ctx,
        &emitter,
    );
    assert!(!guard.record_event_emission(LifecycleSignal::Finalize));
}

/// Regression for the setup-stack failure path: `run_event_stack` records
/// nothing, so running the `Failure` stack alone leaves `terminal_emitted`
/// false and `Finalize` stays a no-op. Only `record_event_emission(Failure)`
/// flips the flag so the subsequent `Finalize` fires. This is the
/// bookkeeping invariant the `routes_to_failure` fix depends on.
#[test]
fn finalize_requires_recorded_terminal_not_just_stack_run() {
    let config = test_config();
    let (settings, messaging, term) = test_ctx();
    let emitter = RecordingEmitter::new();
    let ctx = LifecycleRuntimeContext {
        settings: &settings,
        messaging: &messaging,
        term: &term,
        source_path: dummy_path(),
        repo_root: None,
        launch_area: None,
        context: None,
    };

    // Running the failure stack via a context (without record) does not
    // touch the guard's terminal flag, so Finalize is still skipped.
    let mut guard = LifecycleRunGuard::new(&config, &ctx, &emitter);
    let stack_ctx = crate::composition::lifecycle_executor::StackExecutionContext {
        signal: LifecycleSignal::Failure,
        scope: None,
        frontmatter: &serde_json::Map::new(),
        live_frontmatter: None,
        runtime_state: None,
        err: None,
        timing: None,
        current: None,
        group: None,
        base_dir: None,
        ctx_base_dir: None,
        prepared_context: None,
        file_resolution_context: crate::test_support::process_context(),
        effect_engine: &darkmatter::effects::EffectEngine::builder()
            .mutation_root(std::env::current_dir().unwrap())
            .auto_rehash(false)
            .build(),
        shell_runner: &crate::composition::lifecycle_executor::SystemShellRunner,
        emitter: &emitter,
        term: &term,
        source_path: dummy_path(),
        repo_root: None,
        messaging: &messaging,
        settings: &settings,
    };
    guard.run_event_stack(LifecycleSignal::Failure, &stack_ctx);
    assert!(
        !guard.record_event_emission(LifecycleSignal::Finalize),
        "Finalize must be a no-op when no terminal signal was recorded"
    );

    // Recording Failure first flips terminal_emitted, so Finalize fires.
    let mut guard = LifecycleRunGuard::new(&config, &ctx, &emitter);
    assert!(guard.record_event_emission(LifecycleSignal::Failure));
    assert!(
        guard.record_event_emission(LifecycleSignal::Finalize),
        "Finalize must fire once the terminal Failure signal is recorded"
    );
}

/// `redesignate_terminal_to_failure` overwrites a recorded `Success`/
/// `Blocked` terminal slot with `Failure` while keeping `terminal_emitted`
/// true — so a `success`/`blocked` stack's `error()` downgrade can run the
/// `failure` event and still reach `finalize`. The success/blocked top-level
/// emission stays fired (it happened before the stack), and re-designation
/// is a no-op for any other slot.
#[test]
fn redesignate_terminal_to_failure_overwrites_success_keeps_finalize() {
    let config = test_config();
    let (settings, messaging, term) = test_ctx();
    let emitter = RecordingEmitter::new();
    let ctx = LifecycleRuntimeContext {
        settings: &settings,
        messaging: &messaging,
        term: &term,
        source_path: dummy_path(),
        repo_root: None,
        launch_area: None,
        context: None,
    };

    // Success slot → re-designate to Failure → finalize still fires.
    let mut guard = LifecycleRunGuard::new(&config, &ctx, &emitter);
    assert!(guard.record_event_emission(LifecycleSignal::Success));
    assert_eq!(guard.terminal_signal(), Some(LifecycleSignal::Success));
    assert!(guard.redesignate_terminal_to_failure());
    assert_eq!(guard.terminal_signal(), Some(LifecycleSignal::Failure));
    assert!(
        guard.record_event_emission(LifecycleSignal::Finalize),
        "finalize must still fire after a success→failure re-designation"
    );

    // Blocked slot re-designates too.
    let mut guard = LifecycleRunGuard::new(&config, &ctx, &emitter);
    assert!(guard.record_event_emission(LifecycleSignal::Blocked));
    assert!(guard.redesignate_terminal_to_failure());
    assert_eq!(guard.terminal_signal(), Some(LifecycleSignal::Failure));

    // No-op when the recorded slot is already Failure (or unset).
    let mut guard = LifecycleRunGuard::new(&config, &ctx, &emitter);
    assert!(!guard.redesignate_terminal_to_failure());
    assert!(guard.record_event_emission(LifecycleSignal::Failure));
    assert!(!guard.redesignate_terminal_to_failure());
    assert_eq!(guard.terminal_signal(), Some(LifecycleSignal::Failure));
}

#[test]
fn run_event_stack_emits_top_level_and_stack() {
    let fm = json!({
        "start": {
            "stderr": "top-level",
            "stack": [{"action": {"stderr": "stack"}}]
        }
    });
    let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
    let (settings, messaging, term) = test_ctx();
    let emitter = RecordingEmitter::new();
    let ctx = LifecycleRuntimeContext {
        settings: &settings,
        messaging: &messaging,
        term: &term,
        source_path: dummy_path(),
        repo_root: None,
        launch_area: None,
        context: None,
    };
    let mut guard = LifecycleRunGuard::new(&config,
        &ctx,
        &emitter,
    );

    assert!(guard.record_event_emission(LifecycleSignal::Start));

    let stack_ctx = crate::composition::lifecycle_executor::StackExecutionContext {
        signal: LifecycleSignal::Start,
        scope: None,
        frontmatter: &serde_json::Map::new(),
        live_frontmatter: None,
        runtime_state: None,
        err: None,
        timing: None,
        current: None,
        group: None,
        base_dir: None,
        ctx_base_dir: None,
        prepared_context: None,
        file_resolution_context: crate::test_support::process_context(),
        effect_engine: &darkmatter::effects::EffectEngine::builder()
            .mutation_root(std::env::current_dir().unwrap())
            .auto_rehash(false)
            .build(),
        shell_runner: &crate::composition::lifecycle_executor::SystemShellRunner,
        emitter: &emitter,
        term: &term,
        source_path: dummy_path(),
        repo_root: None,
        messaging: &messaging,
        settings: &settings,
    };
    let outcome = guard.run_event_stack(LifecycleSignal::Start, &stack_ctx);
    assert!(outcome.control.is_none());
    assert!(outcome.action_error.is_none());

    let stderr_signals: Vec<LifecycleSignal> = emitter
        .signals()
        .into_iter()
        .collect();
    assert_eq!(stderr_signals, vec![LifecycleSignal::Start, LifecycleSignal::Start]);
    let texts: Vec<String> = emitter
        .actions
        .lock()
        .unwrap()
        .iter()
        .filter_map(|a| match a {
            EmittedAction::Stderr { text, .. } => Some(text.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(texts, vec!["top-level", "stack"]);
}

#[test]
fn execute_event_still_runs_full_event() {
    let config = test_config();
    let (settings, messaging, term) = test_ctx();
    let emitter = RecordingEmitter::new();
    let ctx = LifecycleRuntimeContext {
        settings: &settings,
        messaging: &messaging,
        term: &term,
        source_path: dummy_path(),
        repo_root: None,
        launch_area: None,
        context: None,
    };
    let mut guard = LifecycleRunGuard::new(
        &config,
        &ctx,
        &emitter,
    );

    let stack_ctx = crate::composition::lifecycle_executor::StackExecutionContext {
        signal: LifecycleSignal::Start,
        scope: None,
        frontmatter: &serde_json::Map::new(),
        live_frontmatter: None,
        runtime_state: None,
        err: None,
        timing: None,
        current: None,
        group: None,
        base_dir: None,
        ctx_base_dir: None,
        prepared_context: None,
        file_resolution_context: crate::test_support::process_context(),
        effect_engine: &darkmatter::effects::EffectEngine::builder()
            .mutation_root(std::env::current_dir().unwrap())
            .auto_rehash(false)
            .build(),
        shell_runner: &crate::composition::lifecycle_executor::SystemShellRunner,
        emitter: &emitter,
        term: &term,
        source_path: dummy_path(),
        repo_root: None,
        messaging: &messaging,
        settings: &settings,
    };
    let outcome = guard.execute_event(LifecycleSignal::Start, &stack_ctx);
    assert!(outcome.control.is_none());
    assert!(guard.start_emitted());
    assert_eq!(emitter.signals().len(), 1);
}

/// Records, at every `stderr` emission, whether a terminal lifecycle scope was
/// active — the flag the CLI's SIGINT guard reads to grant a repeat Ctrl+C the
/// grace window.
struct ScopeProbeEmitter {
    observations: Mutex<Vec<(LifecycleSignal, bool)>>,
}

impl LifecycleEmitter for ScopeProbeEmitter {
    fn emit_stderr(&self, signal: LifecycleSignal, _text: &str, _term: &Terminal) {
        self.observations
            .lock()
            .unwrap()
            .push((signal, crate::interrupt::terminal_lifecycle_active()));
    }

    fn emit_message(
        &self,
        _text: &str,
        _source_path: &Path,
        _repo_root: Option<&Path>,
        _messaging: &RuntimeMessagingSettings,
    ) {
    }

    fn emit_speech(&self, _text: &str, _tts_config: TtsConfig) {}

    fn emit_effect(&self, _name: &str) {}

    fn emit_notification(&self, _title: &str) {}
}

/// `run_event_stack` is the choke point every production lifecycle path
/// (`execute_event` included) runs a stack through, so observing the scope from
/// inside each event's top-level notification and stack action proves all four
/// terminal events enter it — and that `start` does not.
#[test]
fn run_event_stack_marks_only_terminal_events_as_terminal_lifecycle_work() {
    let event = |label: &str| json!({"stderr": label, "stack": [{"action": {"stderr": label}}]});
    let config = parse_lifecycle_config(
        &json!({
            "start": event("start"),
            "success": event("success"),
            "blocked": event("blocked"),
            "failure": event("failure"),
            "finalize": event("finalize"),
        }),
        dummy_path(),
    )
    .unwrap();
    let (settings, messaging, term) = test_ctx();
    let emitter = ScopeProbeEmitter {
        observations: Mutex::new(Vec::new()),
    };
    let ctx = LifecycleRuntimeContext {
        settings: &settings,
        messaging: &messaging,
        term: &term,
        source_path: dummy_path(),
        repo_root: None,
        launch_area: None,
        context: None,
    };
    let guard = LifecycleRunGuard::new(&config, &ctx, &emitter);
    let effect_engine = darkmatter::effects::EffectEngine::builder()
        .mutation_root(std::env::current_dir().unwrap())
        .auto_rehash(false)
        .build();
    let stack_ctx = crate::composition::lifecycle_executor::StackExecutionContext {
        signal: LifecycleSignal::Start,
        scope: None,
        frontmatter: &serde_json::Map::new(),
        live_frontmatter: None,
        runtime_state: None,
        err: None,
        timing: None,
        current: None,
        group: None,
        base_dir: None,
        ctx_base_dir: None,
        prepared_context: None,
        file_resolution_context: crate::test_support::process_context(),
        effect_engine: &effect_engine,
        shell_runner: &crate::composition::lifecycle_executor::SystemShellRunner,
        emitter: &emitter,
        term: &term,
        source_path: dummy_path(),
        repo_root: None,
        messaging: &messaging,
        settings: &settings,
    };

    let signals = [
        LifecycleSignal::Start,
        LifecycleSignal::Success,
        LifecycleSignal::Blocked,
        LifecycleSignal::Failure,
        LifecycleSignal::Finalize,
    ];
    for signal in signals {
        let outcome = guard.run_event_stack(signal, &stack_ctx);
        assert!(outcome.action_error.is_none(), "{signal:?}: {outcome:?}");
        assert!(
            !crate::interrupt::terminal_lifecycle_active(),
            "the scope must end with the {signal:?} event"
        );
    }

    let observed_for = |signal: LifecycleSignal| -> Vec<bool> {
        emitter
            .observations
            .lock()
            .unwrap()
            .iter()
            .filter(|(emitted, _)| *emitted == signal)
            .map(|(_, active)| *active)
            .collect()
    };
    assert_eq!(observed_for(LifecycleSignal::Start), vec![false, false]);
    for signal in &signals[1..] {
        assert_eq!(
            observed_for(*signal),
            vec![true, true],
            "{signal:?}'s top-level notification and stack must run inside the scope"
        );
    }
}
