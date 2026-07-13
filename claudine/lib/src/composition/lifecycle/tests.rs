//! Tests for lifecycle configuration parsing and validation.

use super::*;
use serde_json::json;

fn dummy_path() -> &'static Path {
    Path::new("test.md")
}

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
fn parses_valid_lifecycle_config() {
    let frontmatter = json!({
        "start": {
            "message": "Starting composition..."
        },
        "success": {
            "say": "All done!",
            "effect": "confirmation"
        }
    });

    let config = parse_lifecycle_config(&frontmatter, dummy_path()).unwrap();
    assert!(config.start.is_some());
    assert_eq!(
        config.start.as_ref().unwrap().message.as_deref(),
        Some("Starting composition...")
    );

    assert!(config.success.is_some());
    let success = config.success.as_ref().unwrap();
    assert_eq!(success.say.as_deref(), Some("All done!"));
    assert_eq!(success.effect.as_deref(), Some("confirmation"));
}

#[test]
fn scan_rejects_pre_checks_removed_key() {
    let frontmatter = json!({
        "pre_checks": [{"command": "test"}],
        "start": { "message": "ok" }
    });
    let (key, replacement) = scan_removed_validation_keys(&frontmatter).unwrap();
    assert_eq!(key, "pre_checks");
    assert!(replacement.contains("initialize"), "replacement: {replacement}");
}

#[test]
fn scan_rejects_post_checks_removed_key() {
    let frontmatter = json!({
        "post_checks": [{"command": "test"}],
        "success": { "message": "ok" }
    });
    let (key, replacement) = scan_removed_validation_keys(&frontmatter).unwrap();
    assert_eq!(key, "post_checks");
    assert!(replacement.contains("success"), "replacement: {replacement}");
}

#[test]
fn scan_rejects_handle_removed_key() {
    let frontmatter = json!({
        "handle": "shell('fix')",
        "start": { "message": "ok" }
    });
    let (key, replacement) = scan_removed_validation_keys(&frontmatter).unwrap();
    assert_eq!(key, "handle");
    assert!(replacement.contains("shell"), "replacement: {replacement}");
}

#[test]
fn scan_rejects_deviate_removed_key() {
    let frontmatter = json!({
        "deviate": "shell('fix')",
        "start": { "message": "ok" }
    });
    let (key, replacement) = scan_removed_validation_keys(&frontmatter).unwrap();
    assert_eq!(key, "deviate");
    assert!(replacement.contains("retry"), "replacement: {replacement}");
}

#[test]
fn scan_rejects_handle_timeout_removed_key() {
    let frontmatter = json!({
        "handle_timeout": [{"action": "retry"}],
        "failure": { "message": "ok" }
    });
    let (key, replacement) = scan_removed_validation_keys(&frontmatter).unwrap();
    assert_eq!(key, "handle_timeout");
    assert!(replacement.contains("blocked"), "replacement: {replacement}");
}

#[test]
fn scan_rejects_handle_inline_body_unchanged_removed_key() {
    let frontmatter = json!({
        "handle_inline_body_unchanged": [{"action": "retry"}],
        "failure": { "message": "ok" }
    });
    let (key, replacement) = scan_removed_validation_keys(&frontmatter).unwrap();
    assert_eq!(key, "handle_inline_body_unchanged");
    assert!(replacement.contains("failure"), "replacement: {replacement}");
}

#[test]
fn scan_allows_handle_underscore_without_suffix() {
    // `handle_` with no suffix is not one of the removed keys; only exact
    // `handle` and `handle_<non-empty>` are rejected.
    let frontmatter = json!({
        "handle_": { "message": "ok" }
    });
    assert!(scan_removed_validation_keys(&frontmatter).is_none());
}

#[test]
fn scan_returns_none_for_clean_frontmatter() {
    let frontmatter = json!({
        "start": { "message": "ok" }
    });
    assert!(scan_removed_validation_keys(&frontmatter).is_none());
}

#[test]
fn rejects_both_say_and_say_first() {
    let frontmatter = json!({
        "start": {
            "say": "Starting",
            "say_first": "Also starting"
        }
    });

    let result = parse_lifecycle_config(&frontmatter, dummy_path());
    assert!(matches!(
        result,
        Err(CompositionError::LifecycleSayConflict(_))
    ));
}

#[test]
fn trims_empty_strings_to_none() {
    let frontmatter = json!({
        "start": {
            "message": "   ",
            "say": ""
        }
    });

    let config = parse_lifecycle_config(&frontmatter, dummy_path()).unwrap();
    let start = config.start.as_ref().unwrap();
    assert!(start.message.is_none());
    assert!(start.say.is_none());
}

#[test]
fn rejects_unknown_keys() {
    let frontmatter = json!({
        "start": {
            "message": "Starting",
            "unknown_field": "value"
        }
    });

    let result = parse_lifecycle_config(&frontmatter, dummy_path());
    assert!(result.is_err());
}

#[test]
fn rejects_unknown_effect_name() {
    let frontmatter = json!({
        "start": {
            "effect": "nonexistent-effect"
        }
    });

    let result = parse_lifecycle_config(&frontmatter, dummy_path());
    assert!(matches!(
        result,
        Err(CompositionError::LifecycleUnknownEffect(_, _))
    ));
}

#[test]
fn say_plus_effect_is_valid() {
    let frontmatter = json!({
        "success": {
            "say": "Done!",
            "effect": "confirmation"
        }
    });

    let config = parse_lifecycle_config(&frontmatter, dummy_path()).unwrap();
    let success = config.success.as_ref().unwrap();
    assert_eq!(success.say.as_deref(), Some("Done!"));
    assert_eq!(success.effect.as_deref(), Some("confirmation"));
}

#[test]
fn say_first_plus_effect_is_valid() {
    let frontmatter = json!({
        "success": {
            "say_first": "Starting now",
            "effect": "confirmation"
        }
    });

    let config = parse_lifecycle_config(&frontmatter, dummy_path()).unwrap();
    let success = config.success.as_ref().unwrap();
    assert_eq!(success.say_first.as_deref(), Some("Starting now"));
    assert_eq!(success.effect.as_deref(), Some("confirmation"));
}

#[test]
fn empty_frontmatter_returns_default() {
    let frontmatter = json!({});
    let config = parse_lifecycle_config(&frontmatter, dummy_path()).unwrap();
    assert!(config.is_empty());
}

#[test]
fn non_object_frontmatter_returns_default() {
    let frontmatter = json!("not an object");
    let config = parse_lifecycle_config(&frontmatter, dummy_path()).unwrap();
    assert!(config.is_empty());
}

#[test]
fn null_lifecycle_property_is_skipped() {
    let frontmatter = json!({
        "start": null,
        "success": {
            "message": "Done"
        }
    });

    let config = parse_lifecycle_config(&frontmatter, dummy_path()).unwrap();
    assert!(config.start.is_none());
    assert!(config.success.is_some());
}

#[test]
fn frontmatter_with_non_lifecycle_keys_is_fine() {
    let frontmatter = json!({
        "title": "My Composition",
        "agent": "claude",
        "start": {
            "message": "Starting"
        }
    });

    let config = parse_lifecycle_config(&frontmatter, dummy_path()).unwrap();
    assert!(config.start.is_some());
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
fn status_state_mapping() {
    assert_eq!(LifecycleSignal::Start.status_state(), StatusState::Info);
    assert_eq!(
        LifecycleSignal::Success.status_state(),
        StatusState::Success
    );
    assert_eq!(LifecycleSignal::Blocked.status_state(), StatusState::Error);
    assert_eq!(LifecycleSignal::Failure.status_state(), StatusState::Error);
}

#[test]
fn property_names() {
    assert_eq!(LifecycleSignal::Start.property_name(), "start");
    assert_eq!(LifecycleSignal::Success.property_name(), "success");
    assert_eq!(LifecycleSignal::Blocked.property_name(), "blocked");
    assert_eq!(LifecycleSignal::Failure.property_name(), "failure");
}

#[test]
fn lifecycle_config_get() {
    let fm = json!({
        "start": { "stderr": "Starting" },
        "failure": { "stderr": "Failed" }
    });
    let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
    assert!(config.get(LifecycleSignal::Start).is_some());
    assert!(config.get(LifecycleSignal::Success).is_none());
    assert!(config.get(LifecycleSignal::Blocked).is_none());
    assert!(config.get(LifecycleSignal::Failure).is_some());
}

#[test]
fn lifecycle_config_is_empty() {
    let empty = LifecycleConfig::default();
    assert!(empty.is_empty());

    let fm = json!({ "start": { "stderr": "Go" } });
    let non_empty = parse_lifecycle_config(&fm, dummy_path()).unwrap();
    assert!(!non_empty.is_empty());
}

#[test]
#[allow(deprecated)]
fn lifecycle_runtime_state_defaults() {
    let state = LifecycleRuntimeState::default();
    assert!(!state.start_emitted);
    assert!(!state.provider_launch_started);
}

// -- RecordingEmitter + LifecycleRunGuard tests -------------------------

use std::sync::Mutex;

#[derive(Debug, Clone, PartialEq)]
enum EmittedAction {
    Stderr {
        signal: LifecycleSignal,
        text: String,
    },
    Message {
        text: String,
    },
    Notification {
        title: String,
    },
    Speech {
        text: String,
    },
    Effect {
        name: String,
    },
}

struct RecordingEmitter {
    actions: Mutex<Vec<EmittedAction>>,
}

impl RecordingEmitter {
    fn new() -> Self {
        Self {
            actions: Mutex::new(Vec::new()),
        }
    }

    fn actions(&self) -> Vec<EmittedAction> {
        self.actions.lock().unwrap().clone()
    }

    fn signals(&self) -> Vec<LifecycleSignal> {
        self.actions
            .lock()
            .unwrap()
            .iter()
            .filter_map(|a| match a {
                EmittedAction::Stderr { signal, .. } => Some(*signal),
                _ => None,
            })
            .collect()
    }
}

impl LifecycleEmitter for RecordingEmitter {
    fn emit_stderr(&self, signal: LifecycleSignal, text: &str, _term: &Terminal) {
        self.actions.lock().unwrap().push(EmittedAction::Stderr {
            signal,
            text: text.to_string(),
        });
    }

    fn emit_message(
        &self,
        text: &str,
        _source_path: &Path,
        _repo_root: Option<&Path>,
        _messaging: &RuntimeMessagingSettings,
    ) {
        self.actions.lock().unwrap().push(EmittedAction::Message {
            text: text.to_string(),
        });
    }

    fn emit_speech(&self, text: &str, _tts_config: TtsConfig) {
        self.actions.lock().unwrap().push(EmittedAction::Speech {
            text: text.to_string(),
        });
    }

    fn emit_effect(&self, name: &str) {
        self.actions.lock().unwrap().push(EmittedAction::Effect {
            name: name.to_string(),
        });
    }

    fn emit_notification(&self, title: &str) {
        self.actions
            .lock()
            .unwrap()
            .push(EmittedAction::Notification {
                title: title.to_string(),
            });
    }
}

fn test_config() -> LifecycleConfig {
    parse_lifecycle_config(
        &json!({
            "start":   { "stderr": "starting" },
            "success": { "stderr": "done" },
            "blocked": { "stderr": "blocked" },
            "failure": { "stderr": "failed" },
        }),
        dummy_path(),
    )
    .unwrap()
}

fn test_ctx() -> (GlobalSettings, RuntimeMessagingSettings, Terminal) {
    (
        GlobalSettings::default(),
        RuntimeMessagingSettings {
            user: None,
            repo: None,
        },
        Terminal::default(),
    )
}

fn make_guard<'a>(
    config: &'a LifecycleConfig,
    ctx: &'a LifecycleRuntimeContext<'a>,
    emitter: &'a RecordingEmitter,
) -> LifecycleRunGuard<'a> {
    LifecycleRunGuard::new(config, ctx, emitter)
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
fn validation_is_the_dispatch_gate_for_leaked_lifecycle() {
    // The `LifecycleRunGuard` does not re-validate; it dispatches whatever
    // string the config holds. The contract "no side effect dispatches a
    // leaked expression" is upheld by `validate_no_interpolation_leaks`
    // running in the prepare layer, *before* a guard is ever built. This
    // test proves both halves of that boundary against the fake emitter.
    let leaked = parse_lifecycle_config(
        &json!({ "start": { "message": "{{ broken( }}" } }),
        dummy_path(),
    )
    .unwrap();

    // 1. Validation rejects the leaked config — the production choke point.
    let err = validate_no_interpolation_leaks(&leaked, dummy_path(), &[]).unwrap_err();
    assert!(matches!(
        err,
        CompositionError::LifecycleInterpolationLeak { .. }
    ));

    // 2. A guard built from that same config WOULD dispatch the raw span
    //    (the message reaches the emitter verbatim), confirming the guard
    //    itself is not the gate — only the prepare-layer validation is.
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
        let mut guard = make_guard(&leaked, &ctx, &emitter);
        guard.emit_start_once();
        guard.defuse();
    }
    assert!(
        emitter.actions().iter().any(|a| matches!(
            a,
            EmittedAction::Message { text } if text.contains("{{ broken(")
        )),
        "guard does not self-gate; validation must run before a guard exists"
    );
}

fn fm_from_json(value: serde_json::Value) -> darkmatter::markdown::Frontmatter {
    let mut fm = darkmatter::markdown::Frontmatter::new();
    if let serde_json::Value::Object(map) = value {
        for (key, val) in map {
            fm.insert(&key, val).unwrap();
        }
    }
    fm
}

#[test]
fn undefined_bare_variable_flags_missing_root() {
    let effective = json!({ "area": "claudine" });
    let defined = effective.as_object();
    assert_eq!(undefined_bare_variable("missing", defined), Some("missing"));
    assert_eq!(undefined_bare_variable("area", defined), None);
    // Nested miss under a defined root is treated as defined.
    assert_eq!(undefined_bare_variable("area.sub", defined), None);
    // Runtime namespaces resolve outside the frontmatter.
    assert_eq!(undefined_bare_variable("ctx.area", defined), None);
    assert_eq!(undefined_bare_variable("env.HOME", defined), None);
    assert_eq!(undefined_bare_variable("doc", defined), None);
    assert_eq!(undefined_bare_variable("doc.area", defined), None);
}

#[test]
fn undefined_lifecycle_variable_is_rejected() {
    let raw = fm_from_json(json!({
        "start": { "message": "before {{ missing_lifecycle_var }} after" }
    }));
    let effective = json!({ "start": { "message": "before  after" } });

    let err =
        validate_no_undefined_lifecycle_variables(&raw, &effective, &LifecycleConfig::default(), dummy_path()).unwrap_err();
    match err {
        CompositionError::LifecycleUndefinedVariable {
            property, variable, ..
        } => {
            assert_eq!(property, "start.message");
            assert_eq!(variable, "missing_lifecycle_var");
        }
        other => panic!("expected LifecycleUndefinedVariable, got: {other:?}"),
    }
}

#[test]
fn defined_and_namespaced_lifecycle_variables_pass() {
    let raw = fm_from_json(json!({
        "start": { "message": "{{ area }} on {{ ctx.today }}" },
        "success": { "say": "{{ missing || 'fallback' }}" },
    }));
    let effective = json!({ "area": "claudine" });

    assert!(validate_no_undefined_lifecycle_variables(&raw, &effective, &LifecycleConfig::default(), dummy_path()).is_ok());
}

#[test]
fn undefined_variable_inside_function_call_is_rejected() {
    // The original broken prompt used `parent_dir(review)`: a bare undefined
    // variable as a function argument must fail preparation, not collapse to
    // an empty string the way the whole-span-only guard let it.
    let raw = fm_from_json(json!({
        "start": { "message": "before {{ parent_dir(missing_review) }} after" }
    }));
    let effective = json!({ "area": "claudine" });

    let err =
        validate_no_undefined_lifecycle_variables(&raw, &effective, &LifecycleConfig::default(), dummy_path()).unwrap_err();
    match err {
        CompositionError::LifecycleUndefinedVariable {
            property, variable, ..
        } => {
            assert_eq!(property, "start.message");
            assert_eq!(variable, "missing_review");
        }
        other => panic!("expected LifecycleUndefinedVariable, got: {other:?}"),
    }
}

#[test]
fn undefined_variable_inside_fallback_argument_passes() {
    // Fallback semantics tolerate the undefined operand even when it is
    // wrapped in a function call, so the whole subtree is skipped.
    let raw = fm_from_json(json!({
        "start": { "message": "{{ parent_dir(missing) || 'home' }}" }
    }));
    let effective = json!({ "area": "claudine" });

    assert!(validate_no_undefined_lifecycle_variables(&raw, &effective, &LifecycleConfig::default(), dummy_path()).is_ok());
}

#[test]
fn undefined_variable_in_ternary_condition_is_rejected() {
    let raw = fm_from_json(json!({
        "start": { "message": "{{ missing == 'x' ? 'a' : 'b' }}" }
    }));
    let effective = json!({});

    let err =
        validate_no_undefined_lifecycle_variables(&raw, &effective, &LifecycleConfig::default(), dummy_path()).unwrap_err();
    match err {
        CompositionError::LifecycleUndefinedVariable {
            property, variable, ..
        } => {
            assert_eq!(property, "start.message");
            assert_eq!(variable, "missing");
        }
        other => panic!("expected LifecycleUndefinedVariable, got: {other:?}"),
    }
}

#[test]
fn undefined_variable_in_ternary_truthy_condition_is_rejected() {
    let raw = fm_from_json(json!({
        "start": { "message": "{{ missing ? 'a' : 'b' }}" }
    }));
    let effective = json!({});

    let err =
        validate_no_undefined_lifecycle_variables(&raw, &effective, &LifecycleConfig::default(), dummy_path()).unwrap_err();
    match err {
        CompositionError::LifecycleUndefinedVariable {
            property, variable, ..
        } => {
            assert_eq!(property, "start.message");
            assert_eq!(variable, "missing");
        }
        other => panic!("expected LifecycleUndefinedVariable, got: {other:?}"),
    }
}

#[test]
fn defined_condition_with_undefined_branch_operands_passes() {
    // Ternary branches intentionally tolerate undefined operands; only the
    // condition is checked.
    let raw = fm_from_json(json!({
        "start": { "message": "{{ defined ? missing : also_missing }}" }
    }));
    let effective = json!({ "defined": true });

    assert!(validate_no_undefined_lifecycle_variables(&raw, &effective, &LifecycleConfig::default(), dummy_path()).is_ok());
}

#[test]
fn undefined_variable_in_index_is_rejected() {
    let raw = fm_from_json(json!({
        "start": { "message": "{{ missing[0] }}" }
    }));
    let effective = json!({});

    let err =
        validate_no_undefined_lifecycle_variables(&raw, &effective, &LifecycleConfig::default(), dummy_path()).unwrap_err();
    match err {
        CompositionError::LifecycleUndefinedVariable { variable, .. } => {
            assert_eq!(variable, "missing");
        }
        other => panic!("expected LifecycleUndefinedVariable, got: {other:?}"),
    }
}

#[test]
fn undefined_variable_in_member_access_is_rejected() {
    let raw = fm_from_json(json!({
        "start": { "message": "{{ missing.foo }}" }
    }));
    let effective = json!({});

    let err =
        validate_no_undefined_lifecycle_variables(&raw, &effective, &LifecycleConfig::default(), dummy_path()).unwrap_err();
    match err {
        CompositionError::LifecycleUndefinedVariable { variable, .. } => {
            assert_eq!(variable, "missing");
        }
        other => panic!("expected LifecycleUndefinedVariable, got: {other:?}"),
    }
}

#[test]
fn defined_variable_inside_function_call_passes() {
    let raw = fm_from_json(json!({
        "start": { "message": "{{ parent_dir(area) }}" }
    }));
    let effective = json!({ "area": "/repo/claudine" });

    assert!(validate_no_undefined_lifecycle_variables(&raw, &effective, &LifecycleConfig::default(), dummy_path()).is_ok());
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

#[test]
fn lifecycle_invalid_error_renders_as_block_error() {
    use biscuit_terminal::errors::BlockError;
    use biscuit_terminal::utils::escape_codes::strip_escape_codes;

    let frontmatter = json!({
        "success": {
            "speak": "hello"
        }
    });

    let err =
        parse_lifecycle_config(&frontmatter, Path::new("prompts/sentrux.md")).unwrap_err();
    let CompositionError::LifecycleInvalid {
        property,
        unknown_field,
        expected_fields,
        source_file,
        ..
    } = &err
    else {
        panic!("expected LifecycleInvalid, got {err:?}");
    };

    assert_eq!(property, "success");
    assert_eq!(unknown_field.as_deref(), Some("speak"));
    assert_eq!(source_file, Path::new("prompts/sentrux.md"));
    assert!(expected_fields.contains(&"say".to_string()));
    assert!(expected_fields.contains(&"say_first".to_string()));
    assert!(expected_fields.contains(&"effect".to_string()));

    let rendered = strip_escape_codes(err.report_block_error_optimistic(Some(80)));
    assert!(
        rendered.contains("success.speak"),
        "dotted property should appear: {rendered}"
    );
    assert!(
        rendered.contains("sentrux.md"),
        "file name should appear: {rendered}"
    );
    assert!(
        rendered.contains("say"),
        "expected fields should list 'say': {rendered}"
    );
}

#[test]
fn parse_serde_unknown_field_extracts_field_and_expected() {
    let frontmatter = json!({
        "failure": {
            "bogus_field": true
        }
    });

    let err = parse_lifecycle_config(&frontmatter, dummy_path()).unwrap_err();
    let CompositionError::LifecycleInvalid {
        property,
        unknown_field,
        expected_fields,
        ..
    } = &err
    else {
        panic!("expected LifecycleInvalid, got {err:?}");
    };

    assert_eq!(property, "failure");
    assert_eq!(unknown_field.as_deref(), Some("bogus_field"));
    assert!(!expected_fields.is_empty());
    assert!(expected_fields.contains(&"say".to_string()));
}

#[test]
fn stack_as_map_reports_sequence_mismatch_not_unknown_property() {
    use biscuit_terminal::errors::BlockError;
    use biscuit_terminal::utils::escape_codes::strip_escape_codes;

    // `stack:` authored as a map (its items missing the leading `-`)
    // rather than a YAML list. This is a type mismatch, NOT an
    // unknown-field error, so no field name / "Expected one of" catalog
    // must be fabricated.
    let frontmatter = json!({
        "initialize": {
            "stack": {
                "when": "phase >= total_phases",
                "action": [{ "warn": "too big" }]
            }
        }
    });

    let err = parse_lifecycle_config(&frontmatter, dummy_path()).unwrap_err();
    let CompositionError::LifecycleInvalid {
        property,
        message,
        unknown_field,
        expected_fields,
        ..
    } = &err
    else {
        panic!("expected LifecycleInvalid, got {err:?}");
    };

    assert_eq!(property, "initialize");
    assert!(unknown_field.is_none());
    assert!(expected_fields.is_empty());
    assert!(
        message.contains("expected a sequence"),
        "raw serde message should be preserved: {message}"
    );

    let rendered = strip_escape_codes(err.report_block_error_optimistic(Some(80)));
    assert!(
        !rendered.contains("Unknown property"),
        "must not fabricate an unknown-property diagnostic: {rendered}"
    );
    assert!(
        !rendered.contains("Expected one of"),
        "must not fabricate a field catalog: {rendered}"
    );
    assert!(
        rendered.contains("stack"),
        "hint should point at the `stack` list shape: {rendered}"
    );
}

// =====================================================================
// Phase 2: extended event inventory, lifecycle concerns, stacks
// =====================================================================

#[test]
fn all_seven_signals_have_canonical_property_names() {
    assert_eq!(LifecycleSignal::Initialize.property_name(), "initialize");
    assert_eq!(LifecycleSignal::Start.property_name(), "start");
    assert_eq!(LifecycleSignal::Success.property_name(), "success");
    assert_eq!(LifecycleSignal::Blocked.property_name(), "blocked");
    assert_eq!(LifecycleSignal::Failure.property_name(), "failure");
    assert_eq!(LifecycleSignal::Finalize.property_name(), "finalize");
    assert_eq!(LifecycleSignal::Loop.property_name(), "loop");
}

#[test]
fn signal_all_iterates_in_canonical_order() {
    let names: Vec<&'static str> =
        LifecycleSignal::ALL.iter().map(|s| s.property_name()).collect();
    assert_eq!(
        names,
        vec![
            "initialize",
            "start",
            "success",
            "blocked",
            "failure",
            "finalize",
            "loop",
        ]
    );
}

#[test]
fn signal_can_carry_error_matrix() {
    // No-error events.
    for event in [
        LifecycleSignal::Initialize,
        LifecycleSignal::Start,
        LifecycleSignal::Success,
        LifecycleSignal::Loop,
    ] {
        assert!(
            !event.can_carry_error(),
            "{event:?} should not be able to carry an error"
        );
    }
    // Err-capable events.
    for event in [
        LifecycleSignal::Blocked,
        LifecycleSignal::Failure,
        LifecycleSignal::Finalize,
    ] {
        assert!(
            event.can_carry_error(),
            "{event:?} should be able to carry an error"
        );
    }
}

#[test]
fn parses_initialize_finalize_top_level_events() {
    let fm = json!({
        "initialize": { "stderr": "composing" },
        "finalize": { "stderr": "cleanup" }
    });
    let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
    assert_eq!(
        config.initialize.as_ref().unwrap().stderr.as_deref(),
        Some("composing")
    );
    assert_eq!(
        config.finalize.as_ref().unwrap().stderr.as_deref(),
        Some("cleanup")
    );
    assert_eq!(
        config
            .get(LifecycleSignal::Initialize)
            .unwrap()
            .stderr
            .as_deref(),
        Some("composing")
    );
    assert_eq!(
        config
            .get(LifecycleSignal::Finalize)
            .unwrap()
            .stderr
            .as_deref(),
        Some("cleanup")
    );
}

#[test]
fn parses_info_warn_and_success_top_level_fields() {
    let fm = json!({
        "start": { "info": "composing" },
        "failure": { "warn": "watch out" },
        "success": { "success": "all done" }
    });
    let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
    assert_eq!(
        config.start.as_ref().unwrap().info.as_deref(),
        Some("composing")
    );
    assert_eq!(
        config.failure.as_ref().unwrap().warn.as_deref(),
        Some("watch out")
    );
    assert_eq!(
        config.success.as_ref().unwrap().success.as_deref(),
        Some("all done")
    );
}

#[test]
fn extracts_loop_lifecycle_concerns() {
    let fm = json!({
        "loop": {
            "while": "phase < total",
            "action": "increment(phase)",
            "say": "iterate",
            "stderr": "looping"
        }
    });
    let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
    let concerns = config.loop_concerns.as_ref().expect("loop concerns");
    assert_eq!(concerns.say.as_deref(), Some("iterate"));
    assert_eq!(concerns.stderr.as_deref(), Some("looping"));
    // `while` and `action` are iteration controls, not lifecycle
    // concerns, so they do not appear on the notification.
    assert_eq!(
        config.get(LifecycleSignal::Loop).unwrap().say.as_deref(),
        Some("iterate")
    );
}

#[test]
fn empty_stack_is_normalized_to_none() {
    let fm = json!({
        "start": { "stack": [] }
    });
    let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
    assert!(config.stacks.start.is_none());
    assert!(config.stack(LifecycleSignal::Start).is_none());
}

#[test]
fn rejects_short_form_say_action() {
    let fm = json!({
        "start": {
            "stack": [
                {"action": "say('hello world')"}
            ]
        }
    });
    let err = parse_lifecycle_config(&fm, dummy_path()).unwrap_err();
    match err {
        CompositionError::LifecycleShortFormRemoved { raw, rewrite, .. } => {
            assert_eq!(raw, "say('hello world')");
            assert_eq!(rewrite, "say: \"hello world\"");
        }
        other => panic!("expected LifecycleShortFormRemoved, got: {other:?}"),
    }
}

#[test]
fn positional_scalar_value_is_taken_literally() {
    // A positional scalar value is literal text by default — `ctx.repo` is
    // the text, not the context expression. Use a whole-value `{{ … }}`
    // span (resolved at event time) to interpolate a value.
    let fm = json!({
        "start": {
            "stack": [{"action": {"say": "ctx.repo"}}]
        }
    });
    let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
    let stack = config.stack(LifecycleSignal::Start).expect("start stack");
    let LifecycleActionKind::Communication(comm) = &stack[0].actions[0].kind else {
        panic!("expected communication action");
    };
    assert_eq!(comm.message, Expr::StringLiteral("ctx.repo".to_string()));
}

#[test]
fn parses_when_condition_with_stack() {
    let fm = json!({
        "start": {
            "stack": [
                {
                    "when": "env.AGENT == 'claude'",
                    "action": {"say": "using claude"}
                }
            ]
        }
    });
    let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
    let stack = config.stack(LifecycleSignal::Start).expect("start stack");
    assert!(stack[0].when.is_some());
}

#[test]
fn parses_multiple_actions_per_stack_item() {
    let fm = json!({
        "start": {
            "stack": [
                {
                    "action": [
                        {"say": "first"},
                        {"info": "second"}
                    ]
                }
            ]
        }
    });
    let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
    let stack = config.stack(LifecycleSignal::Start).expect("start stack");
    assert_eq!(stack[0].actions.len(), 2);
}

#[test]
fn parses_stop_short_form() {
    let fm = json!({
        "initialize": {
            "stack": [{"action": "stop"}]
        }
    });
    let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
    let stack = config
        .stack(LifecycleSignal::Initialize)
        .expect("initialize stack");
    assert_eq!(stack[0].actions.len(), 1);
    assert!(stack[0].actions[0].is_lifecycle_control());
}

#[test]
fn parses_skip_in_initialize() {
    let fm = json!({
        "initialize": {
            "stack": [{"action": "skip"}]
        }
    });
    let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
    let stack = config
        .stack(LifecycleSignal::Initialize)
        .expect("initialize stack");
    assert!(stack[0].actions[0].is_lifecycle_control());
}

#[test]
fn parses_retry_with_count_in_blocked() {
    let fm = json!({
        "blocked": {
            "stack": [{"action": {"retry": 3}}]
        }
    });
    let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
    let stack = config
        .stack(LifecycleSignal::Blocked)
        .expect("blocked stack");
    assert!(stack[0].actions[0].is_lifecycle_control());
}

#[test]
fn parses_proxy_with_file_arg_in_initialize() {
    let fm = json!({
        "initialize": {
            "stack": [{"action": {"proxy": "@fallback.md"}}]
        }
    });
    let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
    let stack = config
        .stack(LifecycleSignal::Initialize)
        .expect("initialize stack");
    assert!(stack[0].actions[0].is_lifecycle_control());
}

#[test]
fn parses_shell_long_form_with_on_error_and_no_error() {
    // Long-form shell action: `command`, `on_error`, `no_error` live
    // inside the explicit `{ action: shell, ... }` object.
    let fm = json!({
        "start": {
            "stack": [
                {
                    "action": {
                        "action": "shell",
                        "command": "git fetch --all",
                        "on_error": "fetch failed",
                        "no_error": true
                    }
                }
            ]
        }
    });
    let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
    let stack = config.stack(LifecycleSignal::Start).expect("start stack");
    let action = &stack[0].actions[0];
    assert!(action.no_error);
}

#[test]
fn parses_side_effect_long_form() {
    // Side-effect long form: `file`, `prop`, `value` live inside the
    // explicit `{ action: set_frontmatter, ... }` object.
    let fm = json!({
        "start": {
            "stack": [
                {
                    "action": {
                        "action": "set_frontmatter",
                        "file": "@spec.md",
                        "prop": "status",
                        "value": "in-progress"
                    }
                }
            ]
        }
    });
    let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
    let _ = config.stack(LifecycleSignal::Start).expect("start stack");
}

#[test]
fn parses_side_effect_short_form() {
    let fm = json!({
        "start": {
            "stack": [{"action": {"ensure_file": "@out/log.md"}}]
        }
    });
    let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
    let _ = config.stack(LifecycleSignal::Start).expect("start stack");
}

#[test]
fn rejects_skip_outside_initialize() {
    let fm = json!({
        "start": {"stack": [{"action": "skip"}]}
    });
    let err = parse_lifecycle_config(&fm, dummy_path()).unwrap_err();
    match err {
        CompositionError::LifecycleActionPlacement {
            action, event, ..
        } => {
            assert_eq!(action, "skip");
            assert_eq!(event, "start");
        }
        other => panic!("expected LifecycleActionPlacement, got: {other:?}"),
    }
}

#[test]
fn flow_control_is_universal_across_events() {
    // Flow control reacts to state, not just errors, so `error`/`retry`/
    // `resume`/`requeue`/`proxy` parse in every event (only `skip` is
    // placement-restricted, to `initialize`). E.g. a `success` stack may
    // `resume` because an expected artifact was not produced.
    let cases: [(&str, serde_json::Value); 6] = [
        ("start", json!({"proxy": "@other.md"})),
        ("start", json!({"retry": null})),
        (
            "success",
            json!({"resume": "the file abc.md was never written; create it"}),
        ),
        ("blocked", json!({"resume": "please"})),
        ("initialize", json!({"defer": "5m"})),
        ("success", json!({"retry": 2})),
    ];
    for (event, action) in cases {
        let fm = json!({ event: {"stack": [{"action": action}]} });
        parse_lifecycle_config(&fm, dummy_path())
            .unwrap_or_else(|e| panic!("`{action}` in `{event}` should parse, got: {e:?}"));
    }
    // `loop` carries iteration controls; a `requeue` there parses too.
    let loop_fm = json!({ "loop": {"while": "true", "stack": [{"action": {"defer": "5m"}}]} });
    parse_lifecycle_config(&loop_fm, dummy_path())
        .unwrap_or_else(|e| panic!("`requeue` in `loop` should parse, got: {e:?}"));
}

#[test]
fn accepts_recovery_actions_in_finalize() {
    // `finalize` is the optional-error terminal event and a last-chance
    // recovery surface, so retry/resume/requeue/proxy all parse there
    // (parity with the `failure` event).
    for action in [
        json!({"retry": 1}),
        json!({"resume": "finish the task"}),
        json!({"defer": "5m"}),
        json!({"proxy": "@other.md"}),
    ] {
        let fm = json!({
            "finalize": {"stack": [{"when": "err", "action": action}]}
        });
        parse_lifecycle_config(&fm, dummy_path())
            .unwrap_or_else(|e| panic!("finalize `{action}` should parse, got: {e:?}"));
    }
}

#[test]
fn rejects_multiple_lifecycle_actions_in_one_item() {
    let fm = json!({
        "blocked": {
            "stack": [
                {"action": ["stop", "skip"]}
            ]
        }
    });
    let err = parse_lifecycle_config(&fm, dummy_path()).unwrap_err();
    assert!(matches!(
        err,
        CompositionError::LifecycleMultipleLifecycleActions { .. }
    ));
}

#[test]
fn rejects_lifecycle_action_not_last() {
    let fm = json!({
        "initialize": {
            "stack": [
                {"action": ["stop", {"say": "unreachable"}]}
            ]
        }
    });
    let err = parse_lifecycle_config(&fm, dummy_path()).unwrap_err();
    assert!(matches!(
        err,
        CompositionError::LifecycleActionOrder { .. }
    ));
}

#[test]
fn accepts_lifecycle_action_as_last() {
    let fm = json!({
        "initialize": {
            "stack": [
                {"action": [{"say": "one"}, "stop"]}
            ]
        }
    });
    let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
    let stack = config
        .stack(LifecycleSignal::Initialize)
        .expect("initialize stack");
    assert_eq!(stack[0].actions.len(), 2);
    assert!(!stack[0].actions[0].is_lifecycle_control());
    assert!(stack[0].actions[1].is_lifecycle_control());
}

#[test]
fn control_checks_fire_identically_for_key_value_form() {
    // The cardinality, ordering, and placement checks operate on the parsed
    // typed `LifecycleControlAction` — independent of whether the author
    // wrote the control positional (`{"action": "skip"}` / `{"stop": null}`)
    // or key/value (`{"action": {"action": "stop"}}`). The positional-form
    // tests above already pin the behavior; this pins the same diagnostics
    // for the key/value form so the two forms cannot drift.

    // Placement: a key/value `skip` outside `initialize` is the same
    // LifecycleActionPlacement error the positional `{"action": "skip"}`
    // trips in `rejects_skip_outside_initialize`.
    let fm = json!({
        "start": {"stack": [{"action": {"action": "skip"}}]}
    });
    match parse_lifecycle_config(&fm, dummy_path()).unwrap_err() {
        CompositionError::LifecycleActionPlacement { action, event, .. } => {
            assert_eq!(action, "skip");
            assert_eq!(event, "start");
        }
        other => panic!("expected LifecycleActionPlacement, got: {other:?}"),
    }

    // Cardinality: two key/value control actions in one item trip
    // LifecycleMultipleLifecycleActions (parity with the positional
    // `["stop", "skip"]` case).
    let fm = json!({
        "blocked": {"stack": [{"action": [{"action": "stop"}, {"action": "skip"}]}]}
    });
    assert!(matches!(
        parse_lifecycle_config(&fm, dummy_path()).unwrap_err(),
        CompositionError::LifecycleMultipleLifecycleActions { .. }
    ));

    // Ordering: a key/value control action before a non-control action trips
    // LifecycleActionOrder (parity with `["stop", {"say": ...}]`).
    let fm = json!({
        "initialize": {"stack": [{"action": [
            {"action": "stop"},
            {"action": "say", "message": "unreachable"}
        ]}]}
    });
    assert!(matches!(
        parse_lifecycle_config(&fm, dummy_path()).unwrap_err(),
        CompositionError::LifecycleActionOrder { .. }
    ));

    // Positive parity: a key/value control action as the LAST item is
    // accepted, exactly like the positional form.
    let fm = json!({
        "initialize": {"stack": [{"action": [
            {"action": "say", "message": "one"},
            {"action": "stop"}
        ]}]}
    });
    let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
    let stack = config
        .stack(LifecycleSignal::Initialize)
        .expect("initialize stack");
    assert_eq!(stack[0].actions.len(), 2);
    assert!(!stack[0].actions[0].is_lifecycle_control());
    assert!(stack[0].actions[1].is_lifecycle_control());
}

#[test]
fn positional_scalar_value_is_literal_text() {
    // Positional scalar values are literal text by default — `using codex`
    // is the text, not an expression. Commas and colons inside are part of
    // the message.
    let cases: [( &str, serde_json::Value, &str); 4] = [
        ("say", json!({"say": "using codex"}), "using codex"),
        (
            "warn",
            json!({"warn": "phase 6, too big"}),
            "phase 6, too big",
        ),
        (
            "error",
            json!({"error": "invalid phase: 6"}),
            "invalid phase: 6",
        ),
        (
            "effect",
            json!({"effect": "crowd-applause"}),
            "crowd-applause",
        ),
    ];
    for (verb, action, expected) in cases {
        let fm = json!({ "blocked": { "stack": [{"action": action}] } });
        let config = parse_lifecycle_config(&fm, dummy_path())
            .unwrap_or_else(|e| panic!("`{verb}` positional scalar should parse, got: {e:?}"));
        let stack = config.stack(LifecycleSignal::Blocked).expect("blocked stack");
        let message = match &stack[0].actions[0].kind {
            LifecycleActionKind::Communication(c) => &c.message,
            LifecycleActionKind::LifecycleControl(LifecycleControlAction::Error {
                reason: Some(r),
            }) => r,
            other => panic!("unexpected action kind for `{verb}`: {other:?}"),
        };
        assert_eq!(message, &Expr::StringLiteral(expected.to_string()), "{verb}");
    }
}

#[test]
fn rejects_missing_closing_paren() {
    let fm = json!({
        "start": {
            "stack": [{"action": "say('hi'"}]
        }
    });
    let err = parse_lifecycle_config(&fm, dummy_path()).unwrap_err();
    assert!(matches!(
        err,
        CompositionError::LifecycleShortFormRemoved { .. }
    ));
}

#[test]
fn rejects_retry_with_too_many_args() {
    let fm = json!({
        "blocked": {
            "stack": [{"action": {"retry": [3, 4]}}]
        }
    });
    let err = parse_lifecycle_config(&fm, dummy_path()).unwrap_err();
    assert!(matches!(
        err,
        CompositionError::LifecycleWrongArity { .. }
    ));
}

#[test]
fn rejects_proxy_missing_target() {
    // `proxy` requires a `target` parameter; a null positional value is
    // wrong arity.
    let fm = json!({
        "initialize": {
            "stack": [{"action": {"proxy": null}}]
        }
    });
    let err = parse_lifecycle_config(&fm, dummy_path()).unwrap_err();
    assert!(matches!(
        err,
        CompositionError::LifecycleWrongArity { .. }
    ));
}

#[test]
fn rejects_stack_item_missing_action_key() {
    let fm = json!({
        "start": {
            "stack": [{"when": "true"}]
        }
    });
    let err = parse_lifecycle_config(&fm, dummy_path()).unwrap_err();
    assert!(matches!(
        err,
        CompositionError::LifecycleStackInvalidShape { .. }
    ));
}

#[test]
fn rejects_unknown_stack_item_key() {
    // A scalar `action` value cannot carry sibling parameter keys; the
    // `bogus` key is rejected as an invalid stack-item shape.
    let fm = json!({
        "start": {
            "stack": [{"action": "stop", "bogus": true}]
        }
    });
    let err = parse_lifecycle_config(&fm, dummy_path()).unwrap_err();
    assert!(matches!(
        err,
        CompositionError::LifecycleStackInvalidShape { .. }
    ));
}

#[test]
fn rejects_stack_item_that_is_not_an_object() {
    let fm = json!({
        "start": {
            "stack": ["stop"]
        }
    });
    let err = parse_lifecycle_config(&fm, dummy_path()).unwrap_err();
    assert!(matches!(
        err,
        CompositionError::LifecycleStackInvalidShape { .. }
    ));
}

// -- positional action parser (Phase 4) ----------------------------------

#[test]
fn parses_positional_communication_scalar() {
    let fm = json!({
        "success": {
            "stack": [
                {"action": {"message": "hello"}},
                {"action": {"effect": "crowd-applause"}},
                {"action": {"stderr": "an error"}},
                {"action": {"success": "it worked"}}
            ]
        }
    });
    let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
    let stack = config.stack(LifecycleSignal::Success).expect("success stack");
    assert_eq!(stack.len(), 4);
    for item in stack {
        assert!(matches!(item.actions[0].kind, LifecycleActionKind::Communication(_)));
    }
}

#[test]
fn parses_positional_shell_scalar() {
    let fm = json!({
        "start": {
            "stack": [{"action": {"shell": "git status"}}]
        }
    });
    let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
    let stack = config.stack(LifecycleSignal::Start).expect("start stack");
    assert!(matches!(stack[0].actions[0].kind, LifecycleActionKind::Shell(_)));
}

#[test]
fn parses_positional_side_effect_array() {
    let fm = json!({
        "start": {
            "stack": [{"action": {"set_frontmatter": ["s.md", "status", "ready"]}}]
        }
    });
    let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
    let stack = config.stack(LifecycleSignal::Start).expect("start stack");
    let action = &stack[0].actions[0];
    let LifecycleActionKind::SideEffect(se) = &action.kind else {
        panic!("expected side-effect action, got {action:?}");
    };
    assert_eq!(se.verb, "set_frontmatter");
    assert_eq!(se.args.len(), 3);
}

#[test]
fn parses_positional_optional_tail_side_effect() {
    let fm = json!({
        "start": {
            "stack": [
                {"action": {"ensure_file": ["out/log.md"]}},
                {"action": {"ensure_file": ["out/log.md", "# log"]}}
            ]
        }
    });
    let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
    let stack = config.stack(LifecycleSignal::Start).expect("start stack");
    assert_eq!(stack.len(), 2);
    for item in stack {
        let LifecycleActionKind::SideEffect(se) = &item.actions[0].kind else {
            panic!("expected side-effect action");
        };
        assert_eq!(se.verb, "ensure_file");
    }
}

#[test]
fn parses_positional_control_verbs() {
    let fm = json!({
        "initialize": {
            "stack": [
                {"action": {"stop": null}},
                {"action": {"stop": []}},
                {"action": {"error": "reason"}},
                {"action": {"retry": 3}},
                {"action": {"proxy": "@other.md"}}
            ]
        }
    });
    let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
    let stack = config.stack(LifecycleSignal::Initialize).expect("init stack");
    assert_eq!(stack.len(), 5);
    for item in stack {
        assert!(item.actions[0].is_lifecycle_control());
    }
}

#[test]
fn parses_positional_expression_function_variadic() {
    let fm = json!({
        "start": {
            "stack": [
                {"action": {"and": ["true", "true", "false"]}},
                {"action": {"or": ["a", "b"]}}
            ]
        }
    });
    let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
    let stack = config.stack(LifecycleSignal::Start).expect("start stack");
    assert_eq!(stack.len(), 2);
    for item in stack {
        assert!(matches!(
            item.actions[0].kind,
            LifecycleActionKind::ExpressionFunction(_)
        ));
    }
}

#[test]
fn parses_positional_expression_function_concrete() {
    let fm = json!({
        "start": {
            "stack": [{"action": {"length": "{{ items }}"}}]
        }
    });
    let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
    let stack = config.stack(LifecycleSignal::Start).expect("start stack");
    let LifecycleActionKind::ExpressionFunction(ef) = &stack[0].actions[0].kind else {
        panic!("expected expression-function action");
    };
    assert_eq!(ef.function, "length");
    assert_eq!(ef.args.len(), 1);
}

#[test]
fn parses_positional_expression_function_bracket_optional() {
    // `number(x, [default])` — the bracketed param is optional, so the
    // one-argument form is valid arity.
    let fm = json!({
        "start": {
            "stack": [{"action": {"number": "{{ value }}"}}]
        }
    });
    let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
    let stack = config.stack(LifecycleSignal::Start).expect("start stack");
    let LifecycleActionKind::ExpressionFunction(ef) = &stack[0].actions[0].kind else {
        panic!("expected expression-function action");
    };
    assert_eq!(ef.function, "number");
    assert_eq!(ef.args.len(), 1);
}

#[test]
fn parses_positional_expression_function_overload_one_arg() {
    // Overloaded functions accept their shortest (one-argument) form: the
    // longer overload's extra parameters are optional.
    let fm = json!({
        "start": {
            "stack": [
                {"action": {"frontmatter": "state.md"}},
                {"action": {"link": "state.md"}}
            ]
        }
    });
    let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
    let stack = config.stack(LifecycleSignal::Start).expect("start stack");
    assert_eq!(stack.len(), 2);

    let LifecycleActionKind::ExpressionFunction(frontmatter) = &stack[0].actions[0].kind else {
        panic!("expected frontmatter expression-function action");
    };
    assert_eq!(frontmatter.function, "frontmatter");
    assert_eq!(frontmatter.args.len(), 1);

    let LifecycleActionKind::ExpressionFunction(link) = &stack[1].actions[0].kind else {
        panic!("expected link expression-function action");
    };
    assert_eq!(link.function, "link");
    assert_eq!(link.args.len(), 1);
}

#[test]
fn parses_positional_expression_function_happy_path() {
    // Confirm the existing fixed-arity expression functions still parse.
    let fm = json!({
        "start": {
            "stack": [
                {"action": {"length": "{{ items }}"}},
                {"action": {"contains": ["{{ haystack }}", "{{ needle }}"]}},
                {"action": {"and": ["true", "true"]}},
                {"action": {"or": ["a", "b"]}}
            ]
        }
    });
    let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
    let stack = config.stack(LifecycleSignal::Start).expect("start stack");
    assert_eq!(stack.len(), 4);
    for item in stack {
        assert!(matches!(
            item.actions[0].kind,
            LifecycleActionKind::ExpressionFunction(_)
        ));
    }
}

#[test]
fn parses_positional_typed_arguments() {
    let fm = json!({
        "start": {
            "stack": [
                {"action": {"set_frontmatter": ["s.md", "ready", "{{ true }}"]}},
                {"action": {"merge_frontmatter": ["s.md", "{{ payload }}"]}}
            ]
        }
    });
    let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
    let stack = config.stack(LifecycleSignal::Start).expect("start stack");

    let LifecycleActionKind::SideEffect(set) = &stack[0].actions[0].kind else {
        panic!("expected set_frontmatter side-effect");
    };
    assert_eq!(set.args[2], Expr::BoolLiteral(true));

    let LifecycleActionKind::SideEffect(merge) = &stack[1].actions[0].kind else {
        panic!("expected merge_frontmatter side-effect");
    };
    assert!(matches!(merge.args[1], Expr::Variable(_)));
}

#[test]
fn parses_positional_action_object_value() {
    // `action: { success: "..." }` is the single-object positional form.
    let fm = json!({
        "success": {
            "stack": [{"action": {"success": "it worked"}}]
        }
    });
    let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
    let stack = config.stack(LifecycleSignal::Success).expect("success stack");
    assert_eq!(stack[0].actions.len(), 1);
    assert!(matches!(
        stack[0].actions[0].kind,
        LifecycleActionKind::Communication(_)
    ));
}

#[test]
fn rejects_positional_wrong_arity_side_effect() {
    let fm = json!({
        "start": {
            "stack": [{"action": {"set_frontmatter": ["s.md"]}}]
        }
    });
    let err = parse_lifecycle_config(&fm, dummy_path()).unwrap_err();
    assert!(
        matches!(err, CompositionError::LifecycleWrongArity { .. }),
        "expected wrong-arity error, got: {err:?}"
    );
}

#[test]
fn rejects_positional_wrong_arity_communication() {
    let fm = json!({
        "success": {
            "stack": [{"action": {"message": ["a", "b"]}}]
        }
    });
    let err = parse_lifecycle_config(&fm, dummy_path()).unwrap_err();
    assert!(
        matches!(err, CompositionError::LifecycleWrongArity { .. }),
        "expected wrong-arity error, got: {err:?}"
    );
}

#[test]
fn rejects_positional_bare_proxy_as_wrong_arity() {
    // `proxy` requires a target; a null/empty-array value is wrong arity,
    // not a short-form issue.
    let fm = json!({
        "initialize": {
            "stack": [{"action": {"proxy": null}}]
        }
    });
    let err = parse_lifecycle_config(&fm, dummy_path()).unwrap_err();
    assert!(
        matches!(err, CompositionError::LifecycleWrongArity { .. }),
        "expected wrong-arity error, got: {err:?}"
    );
}

#[test]
fn rejects_positional_unknown_verb() {
    let fm = json!({
        "start": {
            "stack": [{"action": {"sucess": "it worked"}}]
        }
    });
    let err = parse_lifecycle_config(&fm, dummy_path()).unwrap_err();
    match err {
        CompositionError::LifecycleUnknownVerb { verb, .. } => {
            assert_eq!(verb, "sucess");
        }
        other => panic!("expected LifecycleUnknownVerb, got: {other:?}"),
    }
}

#[test]
fn rejects_positional_object_value() {
    let fm = json!({
        "start": {
            "stack": [{"action": {"merge_frontmatter": {"status": "ready"}}}]
        }
    });
    let err = parse_lifecycle_config(&fm, dummy_path()).unwrap_err();
    assert!(
        matches!(
            err,
            CompositionError::LifecycleObjectDataThroughInterpolationPositional { .. }
        ),
        "expected object-data-through-interpolation error, got: {err:?}"
    );
}

#[test]
fn rejects_ambiguous_multi_key_action_object() {
    let fm = json!({
        "start": {
            "stack": [{"action": {"message": "hi", "route": "team"}}]
        }
    });
    let err = parse_lifecycle_config(&fm, dummy_path()).unwrap_err();
    assert!(
        matches!(err, CompositionError::LifecycleStackAmbiguous { .. }),
        "expected ambiguous error, got: {err:?}"
    );
}

#[test]
fn positional_and_key_value_action_object_coexist_in_array() {
    // The motivating shape from the spec: positional and key/value actions
    // in the same stack array.
    let fm = json!({
        "success": {
            "stack": [
                {
                    "when": "true",
                    "action": [
                        {"success": "it worked"},
                        {"set_frontmatter": ["s.md", "status", "done"]},
                        {"action": "shell", "command": "git push"}
                    ]
                }
            ]
        }
    });
    let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
    let stack = config.stack(LifecycleSignal::Success).expect("success stack");
    assert_eq!(stack[0].actions.len(), 3);
    assert!(matches!(
        stack[0].actions[0].kind,
        LifecycleActionKind::Communication(_)
    ));
    assert!(matches!(
        stack[0].actions[1].kind,
        LifecycleActionKind::SideEffect(_)
    ));
    assert!(matches!(stack[0].actions[2].kind, LifecycleActionKind::Shell(_)));
}

#[test]
fn parses_stdout_field_on_event_block() {
    let fm = json!({
        "start": {"stdout": "hello"}
    });
    let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
    assert_eq!(
        config.start.as_ref().unwrap().stdout.as_deref(),
        Some("hello")
    );
}

#[test]
fn parses_stdout_short_form_action() {
    let fm = json!({
        "start": {
            "stack": [{"action": {"stdout": "hello"}}]
        }
    });
    // `stdout: ...` is a recognized positional communication action;
    // parsing succeeds and produces a single-item stack.
    let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
    assert_eq!(config.stack(LifecycleSignal::Start).unwrap().len(), 1);
}

#[test]
fn parses_stdout_field_on_loop_block() {
    // A top-level `loop.stdout` is extracted as a loop lifecycle concern,
    // alongside the iteration controls. The `while` key keeps the loop
    // block otherwise valid.
    let fm = json!({
        "loop": {"while": "true", "stdout": "hello"}
    });
    let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
    assert_eq!(
        config.loop_concerns.as_ref().unwrap().stdout.as_deref(),
        Some("hello")
    );
}

#[test]
fn legacy_top_level_only_prompts_still_parse() {
    // Legacy prompts that only configure the four original top-level
    // events (`start`, `success`, `blocked`, `failure`) continue to parse
    // and expose those events through `LifecycleConfig::get` exactly as
    // before the seven-event model was introduced.
    let fm = json!({
        "start":   { "stderr": "starting" },
        "success": { "stderr": "done" },
        "blocked": { "stderr": "blocked" },
        "failure": { "stderr": "failed" }
    });
    let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
    assert!(config.initialize.is_none());
    assert!(config.finalize.is_none());
    assert!(config.loop_concerns.is_none());
    assert!(config.stacks.start.is_none());
    assert!(!config.is_empty());
    // `get` continues to work for the four legacy signals.
    for s in [
        LifecycleSignal::Start,
        LifecycleSignal::Success,
        LifecycleSignal::Blocked,
        LifecycleSignal::Failure,
    ] {
        assert!(config.get(s).is_some(), "expected {s:?} to be configured");
    }
}

// =====================================================================
// Phase 5: positional-and-key-value action validation checkpoint
// =====================================================================

#[test]
fn short_form_rejection_rewrites_to_positional() {
    // Removed `verb(args)` short form is rejected with a did-you-mean
    // positional rewrite.
    let cases: [(&str, serde_json::Value, &str); 3] = [
        ("success", json!({"success": "x"}), "success: \"x\""),
        ("shell", json!({"shell": "git push"}), "shell: \"git push\""),
        (
            "set_frontmatter",
            json!({"set_frontmatter": ["a", "b", "c"]}),
            "set_frontmatter: [\"a\", \"b\", \"c\"]",
        ),
    ];
    for (verb, action, expected_rewrite) in cases {
        let short_form = format!("{verb}({})", match verb {
            "success" => "\"x\"".to_string(),
            "shell" => "git push".to_string(),
            "set_frontmatter" => "'a','b','c'".to_string(),
            _ => unreachable!(),
        });
        let fm = json!({
            "start": {
                "stack": [{"action": short_form.clone()}]
            }
        });
        let err = parse_lifecycle_config(&fm, dummy_path()).unwrap_err();
        match err {
            CompositionError::LifecycleShortFormRemoved { raw, rewrite, .. } => {
                assert_eq!(raw, short_form, "{verb}");
                assert_eq!(rewrite, expected_rewrite, "{verb}");
            }
            other => panic!("expected LifecycleShortFormRemoved for {verb}, got: {other:?}"),
        }

        // The positional rewrite itself parses cleanly.
        let fm = json!({
            "start": {
                "stack": [{"action": action}]
            }
        });
        assert!(
            parse_lifecycle_config(&fm, dummy_path()).is_ok(),
            "{verb} positional rewrite should parse"
        );
    }
}

#[test]
fn bare_stop_accepted_bare_proxy_rejected_wrong_arity() {
    // Zero-arg positional: bare `stop` is accepted.
    let fm = json!({
        "initialize": {
            "stack": [{"action": "stop"}]
        }
    });
    assert!(parse_lifecycle_config(&fm, dummy_path()).is_ok());

    // `proxy` requires a target; a bare verb is wrong arity.
    let fm = json!({
        "initialize": {
            "stack": [{"action": "proxy"}]
        }
    });
    let err = parse_lifecycle_config(&fm, dummy_path()).unwrap_err();
    assert!(
        matches!(err, CompositionError::LifecycleWrongArity { ref verb, .. } if verb == "proxy"),
        "expected wrong-arity for bare proxy, got: {err:?}"
    );
}

#[test]
fn key_value_literal_default_vs_whole_value_interpolation() {
    // Key/value literal default: a plain string parameter is a literal.
    let fm = json!({
        "start": {
            "stack": [{"action": {"action": "message", "message": "ctx.area"}}]
        }
    });
    let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
    let stack = config.stack(LifecycleSignal::Start).expect("start stack");
    let LifecycleActionKind::Communication(comm) = &stack[0].actions[0].kind else {
        panic!("expected communication action");
    };
    assert_eq!(comm.message, Expr::StringLiteral("ctx.area".to_string()));

    // Whole-value interpolation resolves the expression at event time.
    let fm = json!({
        "start": {
            "stack": [{"action": {"action": "message", "message": "{{ ctx.area }}"}}]
        }
    });
    let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
    let stack = config.stack(LifecycleSignal::Start).expect("start stack");
    let LifecycleActionKind::Communication(comm) = &stack[0].actions[0].kind else {
        panic!("expected communication action");
    };
    assert_eq!(comm.message, Expr::Variable("ctx.area".to_string()));
}

#[test]
fn full_disambiguation_table_for_positional_and_key_value() {
    // Same verb as positional single-key object and as explicit key/value.
    let positional = json!({"start": {"stack": [{"action": {"success": "it worked"}}]}});
    let key_value = json!({
        "start": {
            "stack": [{"action": {"action": "success", "message": "it worked"}}]
        }
    });
    for fm in [&positional, &key_value] {
        let config = parse_lifecycle_config(fm, dummy_path()).unwrap();
        let stack = config.stack(LifecycleSignal::Start).expect("start stack");
        assert!(matches!(
            stack[0].actions[0].kind,
            LifecycleActionKind::Communication(_)
        ));
    }

    // Multi-key object without an `action` key is ambiguous.
    let fm = json!({
        "start": {
            "stack": [{"action": {"message": "hi", "route": "team"}}]
        }
    });
    let err = parse_lifecycle_config(&fm, dummy_path()).unwrap_err();
    assert!(
        matches!(err, CompositionError::LifecycleStackAmbiguous { .. }),
        "expected ambiguous error, got: {err:?}"
    );
}

#[test]
fn predicate_exception_when_evaluates_expression_scalar_stays_literal() {
    // `when` is always a boolean expression.
    let fm = json!({
        "start": {
            "stack": [
                {"when": "true", "action": {"say": "true"}}
            ]
        }
    });
    let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
    let stack = config.stack(LifecycleSignal::Start).expect("start stack");
    assert!(stack[0].when.is_some());

    // The positional scalar `"true"` is literal text, not a bool.
    let LifecycleActionKind::Communication(comm) = &stack[0].actions[0].kind else {
        panic!("expected communication action");
    };
    assert_eq!(comm.message, Expr::StringLiteral("true".to_string()));
}

#[test]
fn known_verb_validation_for_typoed_positional_and_key_value() {
    // Typoed positional verb gets a did-you-mean suggestion.
    let fm = json!({
        "success": {
            "stack": [{"action": {"sucess": "it worked"}}]
        }
    });
    let err = parse_lifecycle_config(&fm, dummy_path()).unwrap_err();
    match err {
        CompositionError::LifecycleUnknownVerb { verb, rewrite, .. } => {
            assert_eq!(verb, "sucess");
            assert!(rewrite.contains("success"), "got: {rewrite}");
        }
        other => panic!("expected LifecycleUnknownVerb for positional typo, got: {other:?}"),
    }

    // Typoed key/value verb gets the same suggestion.
    let fm = json!({
        "success": {
            "stack": [{"action": {"action": "sucess", "message": "it worked"}}]
        }
    });
    let err = parse_lifecycle_config(&fm, dummy_path()).unwrap_err();
    match err {
        CompositionError::LifecycleUnknownVerb { verb, rewrite, .. } => {
            assert_eq!(verb, "sucess");
            assert!(rewrite.contains("success"), "got: {rewrite}");
        }
        other => panic!("expected LifecycleUnknownVerb for key/value typo, got: {other:?}"),
    }
}

#[test]
fn expression_function_actions_positional_key_value_and_variadic_rejection() {
    // Positional expression-function action.
    let fm = json!({
        "start": {
            "stack": [{"action": {"length": "{{ items }}"}}]
        }
    });
    let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
    let stack = config.stack(LifecycleSignal::Start).expect("start stack");
    let LifecycleActionKind::ExpressionFunction(ef) = &stack[0].actions[0].kind else {
        panic!("expected expression-function action");
    };
    assert_eq!(ef.function, "length");
    assert_eq!(ef.args.len(), 1);
    assert_eq!(ef.args[0], Expr::Variable("items".to_string()));

    // Key/value expression-function action with concrete named parameters.
    let fm = json!({
        "start": {
            "stack": [{
                "action": {
                    "action": "contains",
                    "haystack": "{{ haystack }}",
                    "needle": "needle"
                }
            }]
        }
    });
    let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
    let stack = config.stack(LifecycleSignal::Start).expect("start stack");
    let LifecycleActionKind::ExpressionFunction(ef) = &stack[0].actions[0].kind else {
        panic!("expected expression-function action");
    };
    assert_eq!(ef.function, "contains");
    assert_eq!(ef.args.len(), 2);

    // Variadic expression functions reject key/value form.
    for verb in ["and", "or"] {
        let fm = json!({
            "start": {
                "stack": [{"action": {"action": verb, "a": "true", "b": "false"}}]
            }
        });
        let err = parse_lifecycle_config(&fm, dummy_path()).unwrap_err();
        assert!(
            matches!(
                err,
                CompositionError::LifecycleExpressionFunctionKeyValueUnsupported {
                    verb: ref v, ..
                } if v == verb
            ),
            "{verb} key/value should be rejected, got: {err:?}"
        );
    }
}

#[test]
fn key_value_expression_function_rejects_missing_required_param() {
    // `contains(haystack, needle)` — both required. Supplying only
    // `haystack` must fail at parse time, naming the missing `needle`.
    let fm = json!({
        "start": {
            "stack": [{
                "action": {
                    "action": "contains",
                    "haystack": "{{ haystack }}"
                }
            }]
        }
    });
    let err = parse_lifecycle_config(&fm, dummy_path()).unwrap_err();
    match err {
        CompositionError::LifecycleActionInvalidLongForm {
            action, message, ..
        } => {
            assert_eq!(action, "contains");
            assert!(
                message.contains("needle"),
                "message should name the missing `needle` param, got: {message}"
            );
        }
        other => panic!("expected LifecycleActionInvalidLongForm, got: {other:?}"),
    }
}

#[test]
fn key_value_side_effect_rejects_missing_required_params() {
    // `set_frontmatter(file, prop, value)` — all required. Supplying only
    // `file` must fail at parse time, naming both missing params.
    let fm = json!({
        "start": {
            "stack": [{
                "action": {
                    "action": "set_frontmatter",
                    "file": "@state.md"
                }
            }]
        }
    });
    let err = parse_lifecycle_config(&fm, dummy_path()).unwrap_err();
    match err {
        CompositionError::LifecycleActionInvalidLongForm {
            action, message, ..
        } => {
            assert_eq!(action, "set_frontmatter");
            assert!(
                message.contains("prop") && message.contains("value"),
                "message should name both missing params, got: {message}"
            );
        }
        other => panic!("expected LifecycleActionInvalidLongForm, got: {other:?}"),
    }
}

#[test]
fn key_value_omitting_optional_tail_param_parses() {
    // `frontmatter(file, [prop])` (expression function) — `prop` is an
    // optional tail param, so the `file`-only key/value form is valid.
    let fm = json!({
        "start": {
            "stack": [{
                "action": {
                    "action": "frontmatter",
                    "file": "@spec.md"
                }
            }]
        }
    });
    let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
    let stack = config.stack(LifecycleSignal::Start).expect("start stack");
    let LifecycleActionKind::ExpressionFunction(ef) = &stack[0].actions[0].kind else {
        panic!("expected expression-function action");
    };
    assert_eq!(ef.function, "frontmatter");
    assert_eq!(ef.args.len(), 1);

    // `ensure_file(file, [content])` (side effect) — `content` is optional,
    // so the `file`-only key/value form is valid.
    let fm = json!({
        "start": {
            "stack": [{
                "action": {
                    "action": "ensure_file",
                    "file": "@out/log.md"
                }
            }]
        }
    });
    let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
    let stack = config.stack(LifecycleSignal::Start).expect("start stack");
    let LifecycleActionKind::SideEffect(se) = &stack[0].actions[0].kind else {
        panic!("expected side-effect action");
    };
    assert_eq!(se.verb, "ensure_file");
    assert_eq!(se.args.len(), 1);
}

#[test]
fn empty_frontmatter_yields_empty_seven_event_config() {
    let fm = json!({});
    let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
    assert!(config.is_empty());
    for s in LifecycleSignal::ALL {
        assert!(config.get(s).is_none(), "expected {s:?} to be None");
        assert!(config.stack(s).is_none(), "expected stack for {s:?} to be None");
    }
}

#[test]
fn parse_lifecycle_config_handles_non_object_frontmatter() {
    let fm = json!("scalar");
    let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
    assert!(config.is_empty());
}

#[test]
fn null_event_property_is_skipped() {
    let fm = json!({
        "initialize": null,
        "start": { "stderr": "go" }
    });
    let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
    assert!(config.initialize.is_none());
    assert!(config.start.is_some());
}

#[test]
fn loop_concerns_stack_uses_loop_signal_for_placement() {
    // `Skip` is the one placement-restricted action (`initialize` only),
    // so it is invalid in the `loop` event.
    let fm = json!({
        "loop": {
            "while": "true",
            "stack": [{"action": "skip"}]
        }
    });
    let err = parse_lifecycle_config(&fm, dummy_path()).unwrap_err();
    match err {
        CompositionError::LifecycleActionPlacement { event, action, .. } => {
            assert_eq!(event, "loop");
            assert_eq!(action, "skip");
        }
        other => panic!("expected LifecycleActionPlacement, got: {other:?}"),
    }
}

#[test]
fn stop_is_valid_in_every_event() {
    for s in LifecycleSignal::ALL {
        let fm = if s == LifecycleSignal::Loop {
            json!({
                "loop": {
                    "while": "true",
                    "stack": [{"action": "stop"}]
                }
            })
        } else {
            json!({
                s.property_name(): {"stack": [{"action": "stop"}]}
            })
        };
        let config = parse_lifecycle_config(&fm, dummy_path());
        assert!(
            config.is_ok(),
            "`stop` should be valid in {s:?}, got: {:?}",
            config.err()
        );
    }
}

#[test]
fn frontmatter_excerpt_included_for_placement_error() {
    // The `WithFrontmatter` wrapper is applied at the render boundary
    // (CLI handlers), not at the parse site. Here we only verify that the
    // underlying placement error carries the property name needed for
    // frontmatter highlighting.
    let fm = json!({
        "start": {"stack": [{"action": "skip"}]}
    });
    let err = parse_lifecycle_config(&fm, dummy_path()).unwrap_err();
    match err {
        CompositionError::LifecycleActionPlacement {
            property, event, ..
        } => {
            // The stack item is at index 0, so the annotated property
            // path is `start.stack[0]`. Frontmatter highlighting falls
            // back to the top-level `start` key when no per-stack-item
            // line is found.
            assert!(property.starts_with("start"), "got: {property}");
            assert_eq!(event, "start");
        }
        other => panic!("expected placement error, got: {other:?}"),
    }
}

// =====================================================================
// Phase 3: lifecycle context, static scans, shell-audit collection
// =====================================================================

// -- err static scan ---------------------------------------------------

#[test]
fn err_in_start_stack_when_clause_is_rejected() {
    // `err` is forbidden in `start` (a no-error event) — even inside a
    // `when:` condition.
    let fm = json!({
        "start": {
            "stack": [
                {"when": "err != null", "action": {"say": "has error"}}
            ]
        }
    });
    let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
    let err = validate_no_err_in_no_error_events(&config, dummy_path()).unwrap_err();
    match err {
        CompositionError::LifecycleErrNotAvailable { event, property, .. } => {
            assert_eq!(event, "start");
            assert!(property.contains("when"), "got: {property}");
        }
        other => panic!("expected LifecycleErrNotAvailable, got: {other:?}"),
    }
}

#[test]
fn err_member_access_in_single_text_arg_is_literal() {
    // A positional scalar value is literal text by default — `err.msg` is
    // the text, not the `err` global. There is nothing to reject. To
    // reference the error in an error-carrying event, interpolate instead:
    // `{ say: "{{err.msg}}" }`.
    let fm = json!({
        "start": {
            "stack": [{"action": {"say": "err.msg"}}]
        }
    });
    let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
    assert!(validate_no_err_in_no_error_events(&config, dummy_path()).is_ok());
}

#[test]
fn err_in_single_text_arg_is_literal_across_no_error_events() {
    // A positional scalar value is literal text in every no-error event —
    // the err-availability guard only governs expression surfaces (e.g.
    // `when:` clauses), not literal message bodies.
    for ev in ["initialize", "success"] {
        let fm = json!({
            ev: {"stack": [{"action": {"say": "err"}}]}
        });
        let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
        assert!(
            validate_no_err_in_no_error_events(&config, dummy_path()).is_ok(),
            "bare `err` in a {ev} message arg should be literal, not rejected"
        );
    }
    // Loop concerns live under `loop:`.
    let fm = json!({
        "loop": {
            "while": "true",
            "stack": [{"action": {"say": "err"}}]
        }
    });
    let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
    assert!(validate_no_err_in_no_error_events(&config, dummy_path()).is_ok());
}

#[test]
fn err_in_blocked_failure_finalize_is_allowed() {
    // `err` is permitted in error-carrying events.
    for event in ["blocked", "failure", "finalize"] {
        let fm = json!({
            event: {"stack": [{"action": {"say": "err.msg"}}]}
        });
        let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
        let result = validate_no_err_in_no_error_events(&config, dummy_path());
        assert!(
            result.is_ok(),
            "err should be allowed in {event}, got: {:?}",
            result.err()
        );
    }
}

#[test]
fn doc_err_escape_hatch_is_allowed_everywhere() {
    // `doc.err` reaches a literal frontmatter property, not the lifecycle
    // global, so it is permitted even in no-error events.
    for event in ["initialize", "start", "success", "loop"] {
        let fm = if event == "loop" {
            json!({
                "loop": {
                    "while": "true",
                    "stack": [{"action": {"say": "doc.err"}}]
                }
            })
        } else {
            json!({
                event: {"stack": [{"action": {"say": "doc.err"}}]}
            })
        };
        let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
        let result = validate_no_err_in_no_error_events(&config, dummy_path());
        assert!(
            result.is_ok(),
            "doc.err should be allowed in {event}, got: {:?}",
            result.err()
        );
    }
}

#[test]
fn err_in_control_reason_single_text_arg_is_literal() {
    // `error` with a positional scalar value takes its reason literally, so
    // `err.msg` is text, not a reference to the `err` global and is not
    // rejected.
    let fm = json!({
        "start": {
            "stack": [{"action": {"error": "err.msg"}}]
        }
    });
    let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
    assert!(validate_no_err_in_no_error_events(&config, dummy_path()).is_ok());
}

#[test]
fn err_in_shell_command_single_text_arg_is_literal() {
    // `shell` with a positional scalar value takes its command literally, so
    // `err.msg` is text, not an `err`-global reference.
    let fm = json!({
        "loop": {
            "while": "true",
            "stack": [{"action": {"shell": "err.msg"}}]
        }
    });
    let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
    assert!(validate_no_err_in_no_error_events(&config, dummy_path()).is_ok());
}

// -- err static scan over interpolation spans (C4) --------------------

#[test]
fn err_interpolation_span_in_top_level_field_rejected_in_no_error_event() {
    // Late binding (C4): a top-level field reaches `err` only through a
    // `{{ … }}` span, and `err` is still forbidden in a no-error event.
    let fm = json!({ "start": { "message": "❌️  {{err.msg}}" } });
    let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
    let err = validate_no_err_in_no_error_events(&config, dummy_path()).unwrap_err();
    match err {
        CompositionError::LifecycleErrNotAvailable { event, property, .. } => {
            assert_eq!(event, "start");
            assert_eq!(property, "start.message");
        }
        other => panic!("expected LifecycleErrNotAvailable, got: {other:?}"),
    }
}

#[test]
fn err_interpolation_span_in_stack_message_rejected_in_no_error_event() {
    // A positional scalar message body is literal text, but its `{{ … }}`
    // span still reaches the `err` global and must be rejected in `start`.
    let fm = json!({
        "start": { "stack": [{"action": {"message": "❌️  {{err.msg}}"}}] }
    });
    let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
    let err = validate_no_err_in_no_error_events(&config, dummy_path()).unwrap_err();
    match err {
        CompositionError::LifecycleErrNotAvailable { event, property, .. } => {
            assert_eq!(event, "start");
            assert!(property.starts_with("start.stack"), "got: {property}");
        }
        other => panic!("expected LifecycleErrNotAvailable, got: {other:?}"),
    }
}

#[test]
fn timing_and_current_interpolation_allowed_in_no_error_events() {
    // `timing`/`current` are allowed everywhere, including no-error events.
    let fm = json!({
        "start": { "message": "took {{timing.document_ms}}ms on {{current.ctx.agent}}" }
    });
    let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
    assert!(validate_no_err_in_no_error_events(&config, dummy_path()).is_ok());
}

#[test]
fn err_interpolation_span_allowed_in_error_carrying_event() {
    // The same `{{err.msg}}` span is fine in `failure` (an error event).
    let fm = json!({ "failure": { "message": "❌️  {{err.msg}}" } });
    let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
    assert!(validate_no_err_in_no_error_events(&config, dummy_path()).is_ok());
}

// -- deferred effect validation (C4) ----------------------------------

#[test]
fn effect_field_with_interpolation_skips_prepare_validation() {
    // An `effect: "{{name}}"` cannot be checked against the catalog at parse
    // time, so it parses cleanly and is validated at event-time instead.
    let fm = json!({ "success": { "effect": "{{effect_name}}" } });
    assert!(parse_lifecycle_config(&fm, dummy_path()).is_ok());
}

#[test]
fn effect_field_literal_unknown_name_still_rejected_at_prepare() {
    // A literal (interpolation-free) unknown effect name is still rejected
    // at parse time.
    let fm = json!({ "success": { "effect": "nonexistent-effect-xyz" } });
    let err = parse_lifecycle_config(&fm, dummy_path()).unwrap_err();
    assert!(matches!(
        err,
        CompositionError::LifecycleUnknownEffect(_, _)
    ));
}

// -- stack leak scan ---------------------------------------------------

#[test]
fn stack_string_literal_with_interpolation_span_is_leak() {
    // A string literal inside a parsed expression that contains a
    // surviving `{{ … }}` span is a leak — the literal is passed through
    // verbatim to the evaluated result.
    let fm = json!({
        "start": {
            "stack": [{"action": {"say": "leaked {{ broken( }}"}}]
        }
    });
    let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
    let err = validate_no_interpolation_leaks(&config, dummy_path(), &[]).unwrap_err();
    match err {
        CompositionError::LifecycleInterpolationLeak { property, .. } => {
            assert!(
                property.starts_with("start.stack"),
                "expected stack property, got: {property}"
            );
        }
        other => panic!("expected LifecycleInterpolationLeak, got: {other:?}"),
    }
}

#[test]
fn top_level_info_field_leak_is_caught() {
    // The `info` field is now covered by the leak scan (Phase 2 added
    // the field; Phase 3 extends the scan to cover it).
    let config = LifecycleConfig {
        start: Some(LifecycleNotification {
            info: Some("leaked {{ broken( }}".to_string()),
            ..Default::default()
        }),
        ..Default::default()
    };
    let err = validate_no_interpolation_leaks(&config, dummy_path(), &[]).unwrap_err();
    match err {
        CompositionError::LifecycleInterpolationLeak { property, .. } => {
            assert_eq!(property, "start.info");
        }
        other => panic!("expected leak, got: {other:?}"),
    }
}

#[test]
fn top_level_warn_field_leak_is_caught() {
    let config = LifecycleConfig {
        start: Some(LifecycleNotification {
            warn: Some("leaked {{ broken( }}".to_string()),
            ..Default::default()
        }),
        ..Default::default()
    };
    let err = validate_no_interpolation_leaks(&config, dummy_path(), &[]).unwrap_err();
    assert!(matches!(
        err,
        CompositionError::LifecycleInterpolationLeak { property, .. } if property == "start.warn"
    ));
}

#[test]
fn initialize_finalize_loop_top_level_leaks_are_caught() {
    // All seven events are now covered.
    for event in ["initialize", "finalize"] {
        let config = LifecycleConfig {
            initialize: if event == "initialize" {
                Some(LifecycleNotification {
                    stderr: Some("leaked {{ broken( }}".to_string()),
                    ..Default::default()
                })
            } else {
                None
            },
            finalize: if event == "finalize" {
                Some(LifecycleNotification {
                    stderr: Some("leaked {{ broken( }}".to_string()),
                    ..Default::default()
                })
            } else {
                None
            },
            ..Default::default()
        };
        let result = validate_no_interpolation_leaks(&config, dummy_path(), &[]);
        match result {
            Err(CompositionError::LifecycleInterpolationLeak { property, .. })
                if property.starts_with(event) => {}
            other => panic!("expected leak for {event}, got: {other:?}"),
        }
    }
}

// -- stack undefined-variable scan -------------------------------------

#[test]
fn stack_undefined_variable_in_when_clause_is_rejected() {
    let fm = json!({
        "start": {
            "stack": [
                {"when": "missing_var == 'x'", "action": {"say": "hi"}}
            ]
        }
    });
    let raw = fm_from_json(fm.clone());
    let effective = json!({});
    let lifecycle = parse_lifecycle_config(&fm, dummy_path()).unwrap();
    let err = validate_no_undefined_lifecycle_variables(&raw, &effective, &lifecycle, dummy_path())
        .unwrap_err();
    match err {
        CompositionError::LifecycleUndefinedVariable { property, variable, .. } => {
            assert!(property.contains("when"), "got: {property}");
            assert_eq!(variable, "missing_var");
        }
        other => panic!("expected undefined variable, got: {other:?}"),
    }
}

#[test]
fn stack_err_global_is_not_undefined_in_failure() {
    // `err` is a lifecycle global in stack expressions, so it must not
    // trip the undefined-variable scan (the err static scan handles
    // misuse).
    let fm = json!({
        "failure": {
            "stack": [{"action": {"say": "err.msg"}}]
        }
    });
    let raw = fm_from_json(fm.clone());
    let effective = json!({});
    let lifecycle = parse_lifecycle_config(&fm, dummy_path()).unwrap();
    let result = validate_no_undefined_lifecycle_variables(&raw, &effective, &lifecycle, dummy_path());
    assert!(result.is_ok(), "err should not be undefined, got: {:?}", result.err());
}

#[test]
fn stack_timing_and_current_globals_are_not_undefined() {
    let fm = json!({
        "start": {
            "stack": [
                {"action": {"say": "timing.document_ms"}},
                {"action": {"say": "current.ctx.agent"}}
            ]
        }
    });
    let raw = fm_from_json(fm.clone());
    let effective = json!({});
    let lifecycle = parse_lifecycle_config(&fm, dummy_path()).unwrap();
    let result = validate_no_undefined_lifecycle_variables(&raw, &effective, &lifecycle, dummy_path());
    assert!(result.is_ok(), "got: {:?}", result.err());
}

#[test]
fn stack_bare_token_in_action_arg_is_literal_not_undefined_variable() {
    // A positional scalar value is literal text by default, so a bare token
    // is not an undefined-variable reference. Real references go through a
    // whole-value `{{ … }}` span.
    let fm = json!({
        "start": {
            "stack": [{"action": {"say": "missing_var"}}]
        }
    });
    let raw = fm_from_json(fm.clone());
    let effective = json!({});
    let lifecycle = parse_lifecycle_config(&fm, dummy_path()).unwrap();
    assert!(
        validate_no_undefined_lifecycle_variables(&raw, &effective, &lifecycle, dummy_path())
            .is_ok(),
        "a bare token in a literal message arg is not a variable reference"
    );
}

// -- lifecycle globals vs body/frontmatter interpolation --------------

#[test]
fn late_binding_global_in_top_level_field_is_a_known_root() {
    // Late binding (C4 / 5.3): `err`/`timing`/`current` are known roots in
    // top-level communication fields just like in stack surfaces — they
    // resolve at event-time, not against frontmatter — so the
    // undefined-variable scan does not flag a bare reference. (Placement
    // misuse — `err` in a no-error event — is caught separately by
    // `validate_no_err_in_no_error_events`.)
    for global in ["err", "timing", "current"] {
        let raw = fm_from_json(json!({
            "failure": { "message": format!("x: {{{{ {global} }}}}") }
        }));
        let effective = json!({});
        let result = validate_no_undefined_lifecycle_variables(
            &raw,
            &effective,
            &LifecycleConfig::default(),
            dummy_path(),
        );
        assert!(result.is_ok(), "`{global}` is a known root; got: {result:?}");
    }
}

#[test]
fn bare_err_in_top_level_field_passes_when_frontmatter_defines_it() {
    // When frontmatter has a literal `err` property, `{{ err }}` in a
    // top-level field resolves to it — the lifecycle global does not
    // interfere.
    let raw = fm_from_json(json!({
        "start": { "message": "error: {{ err }}" }
    }));
    let effective = json!({ "err": "literal-value" });
    let result = validate_no_undefined_lifecycle_variables(
        &raw,
        &effective,
        &LifecycleConfig::default(),
        dummy_path(),
    );
    assert!(result.is_ok(), "got: {:?}", result.err());
}

// -- shell-audit collection -------------------------------------------

#[test]
fn collect_lifecycle_shell_commands_extracts_literal_commands() {
    let fm = json!({
        "start": {
            "stack": [
                {"action": {"action": "shell", "command": "git fetch --all"}},
                {"action": {"say": "not a shell command"}}
            ]
        },
        "failure": {
            "stack": [
                {"action": {"action": "shell", "command": "git reset --hard", "on_error": "cleanup failed"}}
            ]
        }
    });
    let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
    let commands = collect_lifecycle_shell_commands(&config);
    let command_strings: Vec<&str> = commands.iter().map(|(c, _)| c.as_str()).collect();
    assert!(
        command_strings.contains(&"git fetch --all"),
        "expected git fetch, got: {command_strings:?}"
    );
    assert!(
        command_strings.contains(&"git reset --hard"),
        "expected git reset, got: {command_strings:?}"
    );
    assert!(
        command_strings.contains(&"cleanup failed"),
        "expected on_error command, got: {command_strings:?}"
    );
}

#[test]
fn collect_lifecycle_shell_commands_empty_when_no_shells() {
    let fm = json!({
        "start": {
            "stack": [{"action": {"say": "hello"}}]
        }
    });
    let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
    let commands = collect_lifecycle_shell_commands(&config);
    assert!(commands.is_empty(), "got: {commands:?}");
}

#[test]
fn collect_lifecycle_shell_commands_carries_property_path() {
    let fm = json!({
        "start": {
            "stack": [
                {"action": {"action": "shell", "command": "echo hi"}}
            ]
        }
    });
    let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
    let commands = collect_lifecycle_shell_commands(&config);
    assert_eq!(commands.len(), 1);
    let (_, property) = &commands[0];
    assert!(
        property.contains("start.stack[0]") && property.contains(".command"),
        "expected property path, got: {property}"
    );
}

// -- no_error on every action category --------------------------------

#[test]
fn no_error_flag_is_accepted_on_every_action_category() {
    // The universal `no_error: true` flag must be accepted on every
    // action category: communication, shell, side-effect, and
    // expression-function.
    let fm = json!({
        "start": {
            "stack": [
                {
                    "action": [
                        {"action": "say", "message": "hi", "no_error": true},
                        {"action": "shell", "command": "echo hi", "no_error": true},
                        {"action": "set_frontmatter", "file": "@a.md", "prop": "x", "value": "y", "no_error": true},
                        {"action": "length", "x": "hello", "no_error": true}
                    ]
                }
            ]
        }
    });
    let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
    let stack = config.stack(LifecycleSignal::Start).expect("start stack");
    assert_eq!(stack[0].actions.len(), 4);
    for action in &stack[0].actions {
        assert!(action.no_error, "no_error should be true for {:?}", action.kind);
    }
}

#[test]
fn no_error_on_scalar_form_threads_to_every_category() {
    // Scalar form: `no_error` is a sibling key alongside a bare-verb
    // zero-arg `action` value.
    let fm = json!({
        "start": {
            "stack": [
                {
                    "action": "stop",
                    "no_error": true
                }
            ]
        }
    });
    let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
    let stack = config.stack(LifecycleSignal::Start).expect("start stack");
    assert!(stack[0].actions[0].no_error);
}

#[test]
fn no_error_defaults_to_false() {
    let fm = json!({
        "start": {
            "stack": [{"action": {"say": "hi"}}]
        }
    });
    let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
    let stack = config.stack(LifecycleSignal::Start).expect("start stack");
    assert!(!stack[0].actions[0].no_error);
}

// =====================================================================
// Phase 5: runtime state machine
// =====================================================================

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
        frontmatter: &serde_json::Map::new(),
        live_frontmatter: None,
        err: None,
        timing: None,
        current: None,
        base_dir: None,
        ctx_base_dir: None,
        prepared_context: None,
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
        frontmatter: &serde_json::Map::new(),
        live_frontmatter: None,
        err: None,
        timing: None,
        current: None,
        base_dir: None,
        ctx_base_dir: None,
        prepared_context: None,
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
        frontmatter: &serde_json::Map::new(),
        live_frontmatter: None,
        err: None,
        timing: None,
        current: None,
        base_dir: None,
        ctx_base_dir: None,
        prepared_context: None,
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

// -- action_value_to_expr -------------------------------------------------

use darkmatter::markdown::compose::expression::{evaluate, EvaluationLookup};
use serde_json::Value;
use std::collections::HashMap;

struct MapLookup(HashMap<String, Value>);

impl EvaluationLookup for MapLookup {
    fn get(&self, path: &str) -> Option<Value> {
        self.0.get(path).cloned()
    }
}

struct EmptyLookup;

impl EvaluationLookup for EmptyLookup {
    fn get(&self, _path: &str) -> Option<Value> {
        None
    }
}

#[test]
fn action_value_to_expr_plain_literal() {
    let expr = action_value_to_expr(&json!("hello world")).unwrap();
    assert_eq!(expr, Expr::StringLiteral("hello world".into()));
}

#[test]
fn action_value_to_expr_multi_span_interpolation_stays_literal() {
    let expr = action_value_to_expr(&json!("before {{ x }} after")).unwrap();
    assert_eq!(expr, Expr::StringLiteral("before {{ x }} after".into()));
}

#[test]
fn action_value_to_expr_whole_value_bool() {
    let expr = action_value_to_expr(&json!("{{ true }}")).unwrap();
    assert_eq!(expr, Expr::BoolLiteral(true));
}

#[test]
fn action_value_to_expr_whole_value_number() {
    let expr = action_value_to_expr(&json!("{{ 3 }}")).unwrap();
    assert_eq!(expr, Expr::NumberLiteral(3.0));
}

#[test]
fn action_value_to_expr_whole_value_with_surrounding_whitespace() {
    let expr = action_value_to_expr(&json!("  {{ true }}  ")).unwrap();
    assert_eq!(expr, Expr::BoolLiteral(true));
}

#[test]
fn action_value_to_expr_whole_value_null() {
    let expr = action_value_to_expr(&json!("{{ null }}")).unwrap();
    assert_eq!(evaluate(&expr, &EmptyLookup).unwrap(), Value::Null);
}

#[test]
fn action_value_to_expr_whole_value_object_passthrough() {
    let payload = json!({ "status": "ready", "count": 7 });
    let lookup = MapLookup([("payload".to_string(), payload.clone())].into());
    let expr = action_value_to_expr(&json!("{{ payload }}")).unwrap();
    assert_eq!(evaluate(&expr, &lookup).unwrap(), payload);
}

#[test]
fn action_value_to_expr_yaml_scalar_typing() {
    assert_eq!(
        action_value_to_expr(&json!(42)).unwrap(),
        Expr::NumberLiteral(42.0)
    );
    assert_eq!(
        action_value_to_expr(&json!(true)).unwrap(),
        Expr::BoolLiteral(true)
    );
}

#[test]
fn action_value_to_expr_rejects_direct_object() {
    let err = action_value_to_expr(&json!({ "a": 1 })).unwrap_err();
    assert!(
        err.contains("object values are not supported"),
        "unexpected error: {err}"
    );
    assert!(
        err.contains("{{"),
        "error should mention whole-value interpolation: {err}"
    );
}

#[test]
fn action_value_to_expr_rejects_direct_array() {
    let err = action_value_to_expr(&json!([1, 2, 3])).unwrap_err();
    assert!(
        err.contains("array values are not supported"),
        "unexpected error: {err}"
    );
    assert!(
        err.contains("{{"),
        "error should mention whole-value interpolation: {err}"
    );
}
