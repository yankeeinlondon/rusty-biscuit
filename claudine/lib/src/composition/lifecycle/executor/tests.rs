//! Tests for lifecycle action execution.

use super::*;
use std::sync::Mutex;

use serde_json::json;

use super::super::parse_lifecycle_config;

/// Recording emitter + shell runner test double.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Emitted {
    Stderr(String),
    Info(String),
    Warn(String),
    Success(String),
    Stdout(String),
    Message(String),
    Notify(String),
    Speech(String),
    Effect(String),
}

#[derive(Default)]
struct Recorder {
    events: Mutex<Vec<Emitted>>,
}

impl Recorder {
    fn events(&self) -> Vec<Emitted> {
        self.events.lock().unwrap().clone()
    }
    fn push(&self, e: Emitted) {
        self.events.lock().unwrap().push(e);
    }
}

impl LifecycleEmitter for Recorder {
    fn emit_stderr(&self, _signal: LifecycleSignal, text: &str, _term: &Terminal) {
        self.push(Emitted::Stderr(text.to_string()));
    }
    fn emit_message(
        &self,
        text: &str,
        _source_path: &Path,
        _repo_root: Option<&Path>,
        _messaging: &RuntimeMessagingSettings,
    ) {
        self.push(Emitted::Message(text.to_string()));
    }
    fn emit_speech(&self, text: &str, _config: biscuit_speaks::TtsConfig) {
        self.push(Emitted::Speech(text.to_string()));
    }
    fn emit_effect(&self, name: &str) {
        self.push(Emitted::Effect(name.to_string()));
    }
    fn emit_notification(&self, title: &str) {
        self.push(Emitted::Notify(title.to_string()));
    }
    fn emit_info(&self, text: &str, _term: &Terminal) {
        self.push(Emitted::Info(text.to_string()));
    }
    fn emit_warn(&self, text: &str, _term: &Terminal) {
        self.push(Emitted::Warn(text.to_string()));
    }
    fn emit_success(&self, text: &str, _term: &Terminal) {
        self.push(Emitted::Success(text.to_string()));
    }
    fn emit_stdout(&self, text: &str, _term: &Terminal) {
        self.push(Emitted::Stdout(text.to_string()));
    }
}

/// Shell runner that records commands and returns a programmed exit code.
struct MockShell {
    code: i32,
    commands: Mutex<Vec<String>>,
}

impl MockShell {
    fn new(code: i32) -> Self {
        Self {
            code,
            commands: Mutex::new(Vec::new()),
        }
    }
    fn commands(&self) -> Vec<String> {
        self.commands.lock().unwrap().clone()
    }
}

impl ShellRunner for MockShell {
    fn run(&self, command: &str) -> Result<i32, String> {
        self.commands.lock().unwrap().push(command.to_string());
        Ok(self.code)
    }
}

fn temp_engine() -> (tempfile::TempDir, EffectEngine) {
    let dir = tempfile::tempdir().unwrap();
    let engine = EffectEngine::builder()
        .mutation_root(dir.path())
        .auto_rehash(false)
        .build();
    (dir, engine)
}

struct Harness {
    settings: GlobalSettings,
    messaging: RuntimeMessagingSettings,
    term: Terminal,
}

impl Default for Harness {
    fn default() -> Self {
        Self {
            settings: GlobalSettings::default(),
            messaging: RuntimeMessagingSettings {
                user: None,
                repo: None,
            },
            term: Terminal::default(),
        }
    }
}

/// Build a context for `signal` over `frontmatter`, wired to the given
/// recorder, shell, and effect engine.
#[allow(clippy::too_many_arguments)]
fn ctx<'a>(
    signal: LifecycleSignal,
    frontmatter: &'a Map<String, Value>,
    err: Option<&'a LifecycleErrorInfo>,
    engine: &'a EffectEngine,
    shell: &'a dyn ShellRunner,
    recorder: &'a Recorder,
    harness: &'a Harness,
    source_path: &'a Path,
) -> StackExecutionContext<'a> {
    StackExecutionContext {
        signal,
        frontmatter,
        live_frontmatter: None,
        err,
        timing: None,
        current: None,
        base_dir: None,
        ctx_base_dir: None,
        prepared_context: None,
        effect_engine: engine,
        shell_runner: shell,
        emitter: recorder,
        term: &harness.term,
        source_path,
        repo_root: None,
        messaging: &harness.messaging,
        settings: &harness.settings,
    }
}

fn map(value: Value) -> Map<String, Value> {
    value.as_object().unwrap().clone()
}

/// Build a context whose reads/writes flow through a shared cross-event
/// `live` cell, modelling the harness loop's per-attempt live frontmatter.
/// `base` is the immutable composed frontmatter fallback; `live` is the
/// shared mutable state seeded from it.
#[allow(clippy::too_many_arguments)]
fn ctx_with_live<'a>(
    signal: LifecycleSignal,
    base: &'a Map<String, Value>,
    live: &'a std::cell::RefCell<Map<String, Value>>,
    engine: &'a EffectEngine,
    shell: &'a dyn ShellRunner,
    recorder: &'a Recorder,
    harness: &'a Harness,
    source_path: &'a Path,
) -> StackExecutionContext<'a> {
    StackExecutionContext {
        signal,
        frontmatter: base,
        live_frontmatter: Some(live),
        err: None,
        timing: None,
        current: None,
        base_dir: None,
        ctx_base_dir: None,
        prepared_context: None,
        effect_engine: engine,
        shell_runner: shell,
        emitter: recorder,
        term: &harness.term,
        source_path,
        repo_root: None,
        messaging: &harness.messaging,
        settings: &harness.settings,
    }
}

#[test]
fn top_level_communication_fires_before_stack() {
    let config = parse_lifecycle_config(
        &json!({
            "success": {
                "stderr": "top-level first",
                "stack": [{"action": {"info": "then the stack"}}]
            }
        }),
        Path::new("test.md"),
    )
    .unwrap();

    let fm = map(json!({}));
    let (_dir, engine) = temp_engine();
    let shell = MockShell::new(0);
    let recorder = Recorder::default();
    let harness = Harness::default();
    let context = ctx(
        LifecycleSignal::Success,
        &fm,
        None,
        &engine,
        &shell,
        &recorder,
        &harness,
        Path::new("test.md"),
    );

    let outcome = context.execute_event(&config);
    assert_eq!(outcome, LifecycleEventOutcome::default());
    assert_eq!(
        recorder.events(),
        vec![
            Emitted::Stderr("top-level first".to_string()),
            Emitted::Info("then the stack".to_string()),
        ]
    );
}

#[test]
fn success_channel_top_level_and_stack_route_to_emit_success() {
    let config = parse_lifecycle_config(
        &json!({
            "success": {
                "success": "top-level success",
                "stack": [{"action": {"success": "stack success"}}]
            }
        }),
        Path::new("test.md"),
    )
    .unwrap();

    let fm = map(json!({}));
    let (_dir, engine) = temp_engine();
    let shell = MockShell::new(0);
    let recorder = Recorder::default();
    let harness = Harness::default();
    let context = ctx(
        LifecycleSignal::Success,
        &fm,
        None,
        &engine,
        &shell,
        &recorder,
        &harness,
        Path::new("test.md"),
    );

    let outcome = context.execute_event(&config);
    assert_eq!(outcome, LifecycleEventOutcome::default());
    assert_eq!(
        recorder.events(),
        vec![
            Emitted::Success("top-level success".to_string()),
            Emitted::Success("stack success".to_string()),
        ]
    );
}

#[test]
fn top_level_message_fallback_resolves_to_default_for_unknown_optional() {
    // A top-level communication field whose value uses the documented
    // `{{ missing || 'default' }}` migration path must resolve to the
    // fallback at event-time through the real executor (resolve_emit ->
    // resolve_string_value), never erroring on the unknown optional root.
    let config = parse_lifecycle_config(
        &json!({
            "failure": {
                "message": "{{ missing_optional || 'default' }}"
            }
        }),
        Path::new("test.md"),
    )
    .unwrap();

    let fm = map(json!({}));
    let (_dir, engine) = temp_engine();
    let shell = MockShell::new(0);
    let recorder = Recorder::default();
    let harness = Harness::default();
    let context = ctx(
        LifecycleSignal::Failure,
        &fm,
        None,
        &engine,
        &shell,
        &recorder,
        &harness,
        Path::new("test.md"),
    );

    let outcome = context.execute_event(&config);
    assert_eq!(outcome, LifecycleEventOutcome::default());
    assert_eq!(
        recorder.events(),
        vec![Emitted::Message("default".to_string())]
    );
}

#[test]
fn stdout_channel_top_level_and_stack_route_to_emit_stdout() {
    let config = parse_lifecycle_config(
        &json!({
            "success": {
                "stdout": "top-level stdout",
                "stack": [{"action": {"stdout": "stack stdout"}}]
            }
        }),
        Path::new("test.md"),
    )
    .unwrap();

    let fm = map(json!({}));
    let (_dir, engine) = temp_engine();
    let shell = MockShell::new(0);
    let recorder = Recorder::default();
    let harness = Harness::default();
    let context = ctx(
        LifecycleSignal::Success,
        &fm,
        None,
        &engine,
        &shell,
        &recorder,
        &harness,
        Path::new("test.md"),
    );

    let outcome = context.execute_event(&config);
    assert_eq!(outcome, LifecycleEventOutcome::default());
    assert_eq!(
        recorder.events(),
        vec![
            Emitted::Stdout("top-level stdout".to_string()),
            Emitted::Stdout("stack stdout".to_string()),
        ]
    );
}

#[test]
fn when_false_skips_item_when_true_runs() {
    let config = parse_lifecycle_config(
        &json!({
            "success": {
                "stack": [
                    {"when": "flag == 'yes'", "action": {"say": "matched"}},
                    {"when": "flag == 'no'", "action": {"say": "never"}}
                ]
            }
        }),
        Path::new("t.md"),
    )
    .unwrap();

    let fm = map(json!({"flag": "yes"}));
    let (_dir, engine) = temp_engine();
    let shell = MockShell::new(0);
    let recorder = Recorder::default();
    let harness = Harness::default();
    let context = ctx(
        LifecycleSignal::Success,
        &fm,
        None,
        &engine,
        &shell,
        &recorder,
        &harness,
        Path::new("t.md"),
    );

    context.execute_event(&config);
    assert_eq!(recorder.events(), vec![Emitted::Speech("matched".to_string())]);
}

#[test]
fn omitted_when_always_runs() {
    let config = parse_lifecycle_config(
        &json!({"success": {"stack": [{"action": {"warn": "always"}}]}}),
        Path::new("t.md"),
    )
    .unwrap();
    let fm = map(json!({}));
    let (_dir, engine) = temp_engine();
    let shell = MockShell::new(0);
    let recorder = Recorder::default();
    let harness = Harness::default();
    let context = ctx(
        LifecycleSignal::Success,
        &fm,
        None,
        &engine,
        &shell,
        &recorder,
        &harness,
        Path::new("t.md"),
    );
    context.execute_event(&config);
    assert_eq!(recorder.events(), vec![Emitted::Warn("always".to_string())]);
}

/// A `when:` guard referencing an unknown root (a typo) fails the event
/// closed: the outcome carries an action error and the guarded action
/// dispatches nothing. Without the fail-closed guard the null-resolving
/// typo would silently skip the item (Finding 2).
#[test]
fn when_unknown_root_typo_fails_closed() {
    let config = parse_lifecycle_config(
        &json!({
            "success": {
                "stack": [{"when": "spec_fil", "action": {"message": "guarded"}}]
            }
        }),
        Path::new("t.md"),
    )
    .unwrap();
    // `spec_file` is present; the guard's `spec_fil` typo is not.
    let fm = map(json!({"spec_file": "x"}));
    let (_dir, engine) = temp_engine();
    let shell = MockShell::new(0);
    let recorder = Recorder::default();
    let harness = Harness::default();
    let context = ctx(
        LifecycleSignal::Success,
        &fm,
        None,
        &engine,
        &shell,
        &recorder,
        &harness,
        Path::new("t.md"),
    );
    let outcome = context.execute_event(&config);
    assert!(
        outcome.evaluation_error.is_some(),
        "unknown `when:` root must fail closed through the evaluation channel"
    );
    assert!(
        outcome.action_error.is_none(),
        "a guard raise is an evaluation error, not a dispatch failure"
    );
    assert!(
        recorder.events().is_empty(),
        "no side effect dispatches when the guard fails closed"
    );
}

/// A `when:` guard whose unknown name is wrapped in an `|| false` fallback is
/// tolerated (not a typo to fail on): the fallback yields false, so the item
/// is skipped cleanly with no action error and no side effect.
#[test]
fn when_guarded_fallback_false_skips_cleanly() {
    let config = parse_lifecycle_config(
        &json!({
            "success": {
                "stack": [{"when": "maybe_missing || false", "action": {"message": "guarded"}}]
            }
        }),
        Path::new("t.md"),
    )
    .unwrap();
    let fm = map(json!({}));
    let (_dir, engine) = temp_engine();
    let shell = MockShell::new(0);
    let recorder = Recorder::default();
    let harness = Harness::default();
    let context = ctx(
        LifecycleSignal::Success,
        &fm,
        None,
        &engine,
        &shell,
        &recorder,
        &harness,
        Path::new("t.md"),
    );
    let outcome = context.execute_event(&config);
    assert_eq!(outcome, LifecycleEventOutcome::default());
    assert!(recorder.events().is_empty());
}

/// The same guarded-fallback form, but the fallback yields true, so the
/// item's action runs. Confirms the tolerance does not disable a legitimate
/// guard.
#[test]
fn when_guarded_fallback_true_runs_action() {
    let config = parse_lifecycle_config(
        &json!({
            "success": {
                "stack": [{"when": "maybe_missing || true", "action": {"message": "guarded"}}]
            }
        }),
        Path::new("t.md"),
    )
    .unwrap();
    let fm = map(json!({}));
    let (_dir, engine) = temp_engine();
    let shell = MockShell::new(0);
    let recorder = Recorder::default();
    let harness = Harness::default();
    let context = ctx(
        LifecycleSignal::Success,
        &fm,
        None,
        &engine,
        &shell,
        &recorder,
        &harness,
        Path::new("t.md"),
    );
    let outcome = context.execute_event(&config);
    assert_eq!(outcome, LifecycleEventOutcome::default());
    assert_eq!(
        recorder.events(),
        vec![Emitted::Message("guarded".to_string())]
    );
}

/// Regression: a `when:` referencing a known frontmatter key runs the action
/// when it resolves truthy and skips it (no error) when it resolves falsy.
#[test]
fn when_known_key_runs_when_truthy_skips_when_falsy() {
    let config = parse_lifecycle_config(
        &json!({
            "success": {
                "stack": [
                    {"when": "ready", "action": {"message": "ran"}},
                    {"when": "blocked", "action": {"message": "never"}}
                ]
            }
        }),
        Path::new("t.md"),
    )
    .unwrap();
    let fm = map(json!({"ready": true, "blocked": false}));
    let (_dir, engine) = temp_engine();
    let shell = MockShell::new(0);
    let recorder = Recorder::default();
    let harness = Harness::default();
    let context = ctx(
        LifecycleSignal::Success,
        &fm,
        None,
        &engine,
        &shell,
        &recorder,
        &harness,
        Path::new("t.md"),
    );
    let outcome = context.execute_event(&config);
    assert_eq!(outcome, LifecycleEventOutcome::default());
    assert_eq!(
        recorder.events(),
        vec![Emitted::Message("ran".to_string())]
    );
}

#[test]
fn array_actions_run_in_order_then_stop_at_control() {
    let config = parse_lifecycle_config(
        &json!({
            "failure": {
                "stack": [{
                    "action": [{"say": "one"}, {"message": "two"}, "stop"]
                }]
            }
        }),
        Path::new("t.md"),
    )
    .unwrap();
    let fm = map(json!({}));
    let (_dir, engine) = temp_engine();
    let shell = MockShell::new(0);
    let recorder = Recorder::default();
    let harness = Harness::default();
    let context = ctx(
        LifecycleSignal::Failure,
        &fm,
        None,
        &engine,
        &shell,
        &recorder,
        &harness,
        Path::new("t.md"),
    );
    let outcome = context.execute_event(&config);
    assert_eq!(outcome.control, Some(StackControl::Stop));
    assert_eq!(
        recorder.events(),
        vec![
            Emitted::Speech("one".to_string()),
            Emitted::Message("two".to_string()),
        ]
    );
}

#[test]
fn control_action_terminates_remaining_items() {
    let config = parse_lifecycle_config(
        &json!({
            "failure": {
                "stack": [
                    {"action": "stop"},
                    {"action": {"say": "unreached"}}
                ]
            }
        }),
        Path::new("t.md"),
    )
    .unwrap();
    let fm = map(json!({}));
    let (_dir, engine) = temp_engine();
    let shell = MockShell::new(0);
    let recorder = Recorder::default();
    let harness = Harness::default();
    let context = ctx(
        LifecycleSignal::Failure,
        &fm,
        None,
        &engine,
        &shell,
        &recorder,
        &harness,
        Path::new("t.md"),
    );
    let outcome = context.execute_event(&config);
    assert_eq!(outcome.control, Some(StackControl::Stop));
    assert!(recorder.events().is_empty());
}

#[test]
fn shell_action_runs_command() {
    let config = parse_lifecycle_config(
        &json!({"start": {"stack": [{"action": {"shell": "git status --short"}}]}}),
        Path::new("t.md"),
    )
    .unwrap();
    let fm = map(json!({}));
    let (_dir, engine) = temp_engine();
    let shell = MockShell::new(0);
    let recorder = Recorder::default();
    let harness = Harness::default();
    let context = ctx(
        LifecycleSignal::Start,
        &fm,
        None,
        &engine,
        &shell,
        &recorder,
        &harness,
        Path::new("t.md"),
    );
    let outcome = context.execute_event(&config);
    assert_eq!(outcome, LifecycleEventOutcome::default());
    assert_eq!(shell.commands(), vec!["git status --short".to_string()]);
}

#[test]
fn shell_nonzero_at_setup_routes_to_failure() {
    let config = parse_lifecycle_config(
        &json!({
            "start": {
                "stack": [{
                    "action": {"action": "shell", "command": "false", "on_error": "build failed"}
                }]
            }
        }),
        Path::new("t.md"),
    )
    .unwrap();
    let fm = map(json!({}));
    let (_dir, engine) = temp_engine();
    let shell = MockShell::new(1);
    let recorder = Recorder::default();
    let harness = Harness::default();
    let context = ctx(
        LifecycleSignal::Start,
        &fm,
        None,
        &engine,
        &shell,
        &recorder,
        &harness,
        Path::new("t.md"),
    );
    let outcome = context.execute_event(&config);
    assert!(outcome.action_error.is_some());
    assert!(outcome.routes_to_failure(LifecycleSignal::Start));
    // on_error was surfaced as a warning.
    assert_eq!(recorder.events(), vec![Emitted::Warn("build failed".to_string())]);
}

#[test]
fn shell_nonzero_at_terminal_does_not_route_to_failure() {
    let config = parse_lifecycle_config(
        &json!({"success": {"stack": [{"action": {"shell": "false"}}]}}),
        Path::new("t.md"),
    )
    .unwrap();
    let fm = map(json!({}));
    let (_dir, engine) = temp_engine();
    let shell = MockShell::new(2);
    let recorder = Recorder::default();
    let harness = Harness::default();
    let context = ctx(
        LifecycleSignal::Success,
        &fm,
        None,
        &engine,
        &shell,
        &recorder,
        &harness,
        Path::new("t.md"),
    );
    let outcome = context.execute_event(&config);
    assert!(outcome.action_error.is_some());
    assert!(!outcome.routes_to_failure(LifecycleSignal::Success));
}

#[test]
fn no_error_suppresses_propagation_and_continues() {
    let config = parse_lifecycle_config(
        &json!({
            "start": {
                "stack": [
                    {"action": {"action": "shell", "command": "false", "no_error": true}},
                    {"action": {"info": "reached"}}
                ]
            }
        }),
        Path::new("t.md"),
    )
    .unwrap();
    let fm = map(json!({}));
    let (_dir, engine) = temp_engine();
    let shell = MockShell::new(1);
    let recorder = Recorder::default();
    let harness = Harness::default();
    let context = ctx(
        LifecycleSignal::Start,
        &fm,
        None,
        &engine,
        &shell,
        &recorder,
        &harness,
        Path::new("t.md"),
    );
    let outcome = context.execute_event(&config);
    assert_eq!(outcome, LifecycleEventOutcome::default());
    assert_eq!(recorder.events(), vec![Emitted::Info("reached".to_string())]);
}

/// `no_error` is scoped to side-effect dispatch failures: an action whose
/// message interpolation *raises* (an unknown root) must still surface as an
/// evaluation error and halt the stack even when `no_error: true` is set.
#[test]
fn no_error_does_not_suppress_evaluation_raise() {
    let config = parse_lifecycle_config(
        &json!({
            "start": {
                "stack": [
                    {"action": {"action": "message", "message": "{{spec_fil}}", "no_error": true}},
                    {"action": {"info": "unreached"}}
                ]
            }
        }),
        Path::new("t.md"),
    )
    .unwrap();
    let fm = map(json!({"spec_file": "x"}));
    let (_dir, engine) = temp_engine();
    let shell = MockShell::new(0);
    let recorder = Recorder::default();
    let harness = Harness::default();
    let context = ctx(
        LifecycleSignal::Start,
        &fm,
        None,
        &engine,
        &shell,
        &recorder,
        &harness,
        Path::new("t.md"),
    );
    let outcome = context.execute_event(&config);
    assert!(
        outcome.evaluation_error.is_some(),
        "an evaluation raise halts despite no_error"
    );
    assert!(outcome.action_error.is_none());
    assert!(recorder.events().is_empty(), "the stack stopped at the raise");
}

#[test]
fn errored_action_stops_remaining_actions() {
    let config = parse_lifecycle_config(
        &json!({
            "start": {
                "stack": [{
                    "action": [{"shell": "false"}, {"info": "unreached"}]
                }]
            }
        }),
        Path::new("t.md"),
    )
    .unwrap();
    let fm = map(json!({}));
    let (_dir, engine) = temp_engine();
    let shell = MockShell::new(1);
    let recorder = Recorder::default();
    let harness = Harness::default();
    let context = ctx(
        LifecycleSignal::Start,
        &fm,
        None,
        &engine,
        &shell,
        &recorder,
        &harness,
        Path::new("t.md"),
    );
    let outcome = context.execute_event(&config);
    assert!(outcome.action_error.is_some());
    assert!(recorder.events().is_empty());
}

#[test]
fn explicit_error_control_surfaces_with_reason() {
    let config = parse_lifecycle_config(
        &json!({"success": {"stack": [{"action": {"error": "manual failure"}}]}}),
        Path::new("t.md"),
    )
    .unwrap();
    let fm = map(json!({}));
    let (_dir, engine) = temp_engine();
    let shell = MockShell::new(0);
    let recorder = Recorder::default();
    let harness = Harness::default();
    let context = ctx(
        LifecycleSignal::Success,
        &fm,
        None,
        &engine,
        &shell,
        &recorder,
        &harness,
        Path::new("t.md"),
    );
    let outcome = context.execute_event(&config);
    assert_eq!(
        outcome.control,
        Some(StackControl::Error {
            reason: Some("manual failure".to_string())
        })
    );
    assert!(outcome.action_error.is_none());
}

#[test]
fn retry_count_shorthand_resolves_max_attempts() {
    let config = parse_lifecycle_config(
        &json!({"failure": {"stack": [{"action": {"retry": 3}}]}}),
        Path::new("t.md"),
    )
    .unwrap();
    let fm = map(json!({}));
    let (_dir, engine) = temp_engine();
    let shell = MockShell::new(0);
    let recorder = Recorder::default();
    let harness = Harness::default();
    let context = ctx(
        LifecycleSignal::Failure,
        &fm,
        None,
        &engine,
        &shell,
        &recorder,
        &harness,
        Path::new("t.md"),
    );
    let outcome = context.execute_event(&config);
    assert_eq!(
        outcome.control,
        Some(StackControl::Retry {
            max_attempts: 3,
            backoff: RetryBackoff::Fixed,
            delay: "0s".to_string(),
        })
    );
}

#[test]
fn side_effect_short_form_quoted_args_dispatch_to_engine() {
    // Positional form with an array of quoted string args is the
    // unambiguous path: each arg parses as a string literal, so
    // `prop`/`value` reach the engine verbatim.
    let config = parse_lifecycle_config(
        &json!({
            "start": {
                "stack": [{"action": {"set_frontmatter": ["state.md", "status", "in-progress"]}}]
            }
        }),
        Path::new("t.md"),
    )
    .unwrap();
    let fm = map(json!({}));
    let (dir, engine) = temp_engine();
    std::fs::write(dir.path().join("state.md"), "---\n---\nbody\n").unwrap();
    let shell = MockShell::new(0);
    let recorder = Recorder::default();
    let harness = Harness::default();
    let context = ctx(
        LifecycleSignal::Start,
        &fm,
        None,
        &engine,
        &shell,
        &recorder,
        &harness,
        Path::new("t.md"),
    );
    let outcome = context.execute_event(&config);
    assert_eq!(outcome, LifecycleEventOutcome::default());
    let written = std::fs::read_to_string(dir.path().join("state.md")).unwrap();
    assert!(written.contains("status"), "frontmatter updated: {written}");
    assert!(written.contains("in-progress"));
}

#[test]
fn side_effect_long_form_reorders_named_params_positionally() {
    // The parser reorders long-form named params into the verb's
    // positional signature (`http_post` → `[url, body]`), not alphabetical
    // order (which would yield `[body, url]`). The executor then dispatches
    // positionally.
    let config = parse_lifecycle_config(
        &json!({
            "start": {
                "stack": [{
                    "action": {"action": "http_post", "url": "https://example.com/hook", "body": "hello"}
                }]
            }
        }),
        Path::new("t.md"),
    )
    .unwrap();
    let actions = &config.stack(LifecycleSignal::Start).unwrap()[0].actions;
    match &actions[0].kind {
        LifecycleActionKind::SideEffect(effect) => {
            assert_eq!(effect.verb, "http_post");
            assert_eq!(effect.args.len(), 2);
            assert_eq!(
                effect.args[0],
                Expr::StringLiteral("https://example.com/hook".to_string()),
                "url must be the first positional arg"
            );
        }
        other => panic!("expected SideEffect, got {other:?}"),
    }
}

#[test]
fn side_effect_short_form_routes_through_expression_path() {
    let config = parse_lifecycle_config(
        &json!({"start": {"stack": [{"action": {"ensure_file": "out/log.md"}}]}}),
        Path::new("t.md"),
    )
    .unwrap();
    let fm = map(json!({}));
    let (dir, engine) = temp_engine();
    let shell = MockShell::new(0);
    let recorder = Recorder::default();
    let harness = Harness::default();
    let context = ctx(
        LifecycleSignal::Start,
        &fm,
        None,
        &engine,
        &shell,
        &recorder,
        &harness,
        Path::new("t.md"),
    );
    let outcome = context.execute_event(&config);
    assert_eq!(outcome, LifecycleEventOutcome::default());
    assert!(dir.path().join("out/log.md").exists());
}

#[test]
fn err_global_visible_in_failure_stack_when() {
    let config = parse_lifecycle_config(
        &json!({
            "failure": {
                "stack": [{
                    "when": "err.variant == 'Io'",
                    "action": {"stderr": "saw io error"}
                }]
            }
        }),
        Path::new("t.md"),
    )
    .unwrap();
    let fm = map(json!({}));
    let err = LifecycleErrorInfo {
        kind: "ClaudineError",
        variant: "Io".to_string(),
        msg: "disk full".to_string(),
        facets: None,
    };
    let (_dir, engine) = temp_engine();
    let shell = MockShell::new(0);
    let recorder = Recorder::default();
    let harness = Harness::default();
    let context = ctx(
        LifecycleSignal::Failure,
        &fm,
        Some(&err),
        &engine,
        &shell,
        &recorder,
        &harness,
        Path::new("t.md"),
    );
    context.execute_event(&config);
    assert_eq!(
        recorder.events(),
        vec![Emitted::Stderr("saw io error".to_string())]
    );
}

#[test]
fn message_interpolates_frontmatter_in_literal() {
    let config = parse_lifecycle_config(
        &json!({"success": {"stack": [{"action": {"action": "info", "message": "done {{ name }}"}}]}}),
        Path::new("t.md"),
    )
    .unwrap();
    let fm = map(json!({"name": "alpha"}));
    let (_dir, engine) = temp_engine();
    let shell = MockShell::new(0);
    let recorder = Recorder::default();
    let harness = Harness::default();
    let context = ctx(
        LifecycleSignal::Success,
        &fm,
        None,
        &engine,
        &shell,
        &recorder,
        &harness,
        Path::new("t.md"),
    );
    context.execute_event(&config);
    assert_eq!(recorder.events(), vec![Emitted::Info("done alpha".to_string())]);
}

#[test]
fn emit_top_level_for_signal_fires_comm_without_running_stack() {
    // The stack carries a side-effect action; `emit_top_level_for_signal`
    // must emit only the top-level communication and never touch it.
    let config = parse_lifecycle_config(
        &json!({
            "success": {
                "stderr": "top-level only",
                "stack": [{"action": {"info": "must not run"}}]
            }
        }),
        Path::new("t.md"),
    )
    .unwrap();
    let fm = map(json!({}));
    let (_dir, engine) = temp_engine();
    let shell = MockShell::new(0);
    let recorder = Recorder::default();
    let harness = Harness::default();
    let context = ctx(
        LifecycleSignal::Success,
        &fm,
        None,
        &engine,
        &shell,
        &recorder,
        &harness,
        Path::new("t.md"),
    );

    context.emit_top_level_for_signal(&config);

    assert_eq!(
        recorder.events(),
        vec![Emitted::Stderr("top-level only".to_string())],
        "only the top-level stderr fires; the stack's info action does not"
    );
}

#[test]
fn empty_event_yields_default_outcome() {
    let config = LifecycleConfig::default();
    let fm = map(json!({}));
    let (_dir, engine) = temp_engine();
    let shell = MockShell::new(0);
    let recorder = Recorder::default();
    let harness = Harness::default();
    let context = ctx(
        LifecycleSignal::Start,
        &fm,
        None,
        &engine,
        &shell,
        &recorder,
        &harness,
        Path::new("t.md"),
    );
    let outcome = context.execute_event(&config);
    assert_eq!(outcome, LifecycleEventOutcome::default());
    assert!(recorder.events().is_empty());
}

/// Legacy top-level-only lifecycle prompts (no `stack:`, no `initialize`/
/// `finalize`/`loop`) must emit the same channels and order as before the
/// seven-event model was introduced.
#[test]
fn legacy_top_level_only_prompts_emit_same_channels() {
    let config = parse_lifecycle_config(
        &json!({
            "start": {
                "stderr": "starting",
                "say": "go",
                "effect": "confirmation"
            },
            "success": {
                "stderr": "done",
                "say": "finished",
                "effect": "crowd-applause"
            },
            "blocked": { "stderr": "blocked" },
            "failure": { "stderr": "failed" }
        }),
        Path::new("t.md"),
    )
    .unwrap();

    let fm = map(json!({}));
    let (_dir, engine) = temp_engine();
    let shell = MockShell::new(0);
    let recorder = Recorder::default();
    let harness = Harness::default();

    let cases: &[(LifecycleSignal, &[Emitted])] = &[
        (
            LifecycleSignal::Start,
            &[
                Emitted::Stderr("starting".to_string()),
                Emitted::Effect("confirmation".to_string()),
                Emitted::Speech("go".to_string()),
            ],
        ),
        (
            LifecycleSignal::Success,
            &[
                Emitted::Stderr("done".to_string()),
                Emitted::Effect("crowd-applause".to_string()),
                Emitted::Speech("finished".to_string()),
            ],
        ),
        (
            LifecycleSignal::Blocked,
            &[Emitted::Stderr("blocked".to_string())],
        ),
        (
            LifecycleSignal::Failure,
            &[Emitted::Stderr("failed".to_string())],
        ),
    ];

    for (signal, expected) in cases {
        let context = ctx(
            *signal,
            &fm,
            None,
            &engine,
            &shell,
            &recorder,
            &harness,
            Path::new("t.md"),
        );
        let outcome = context.execute_event(&config);
        assert_eq!(outcome, LifecycleEventOutcome::default(), "{signal:?}");
        assert_eq!(recorder.events(), *expected, "{signal:?}");
        recorder.events.lock().unwrap().clear();
    }
}

// ── Phase 4 (C2): event-time interpolation via DM2 ──────────────────

fn io_err(msg: &str) -> LifecycleErrorInfo {
    LifecycleErrorInfo {
        kind: "ClaudineError",
        variant: "Io".to_string(),
        msg: msg.to_string(),
        facets: None,
    }
}

/// Top-level `failure.message: "{{err.msg}}"` is a deferred (raw) key that
/// must interpolate the real error at event-time — the original bug.
#[test]
fn top_level_message_interpolates_err_at_event_time() {
    let config = parse_lifecycle_config(
        &json!({"failure": {"message": "❌️ {{err.msg}}"}}),
        Path::new("t.md"),
    )
    .unwrap();
    let fm = map(json!({}));
    let err = io_err("disk full");
    let (_dir, engine) = temp_engine();
    let shell = MockShell::new(0);
    let recorder = Recorder::default();
    let harness = Harness::default();
    let context = ctx(
        LifecycleSignal::Failure,
        &fm,
        Some(&err),
        &engine,
        &shell,
        &recorder,
        &harness,
        Path::new("t.md"),
    );
    context.execute_event(&config);
    assert_eq!(
        recorder.events(),
        vec![Emitted::Message("❌️ disk full".to_string())]
    );
}

/// A `failure` stack `message(❌️ {{err.msg}})` renders the real error
/// end-to-end through composition (parse → executor → DM2).
#[test]
fn stack_message_interpolates_err_at_event_time() {
    let config = parse_lifecycle_config(
        &json!({"failure": {"stack": [{"action": {"message": "❌️ {{err.msg}}"}}]}}),
        Path::new("t.md"),
    )
    .unwrap();
    let fm = map(json!({}));
    let err = io_err("disk full");
    let (_dir, engine) = temp_engine();
    let shell = MockShell::new(0);
    let recorder = Recorder::default();
    let harness = Harness::default();
    let context = ctx(
        LifecycleSignal::Failure,
        &fm,
        Some(&err),
        &engine,
        &shell,
        &recorder,
        &harness,
        Path::new("t.md"),
    );
    context.execute_event(&config);
    assert_eq!(
        recorder.events(),
        vec![Emitted::Message("❌️ disk full".to_string())]
    );
}

/// A mixed body resolves both an early-binding frontmatter span (`phase`)
/// and a late-binding global span (`err.msg`) at event-time.
#[test]
fn mixed_body_resolves_both_spans_at_event_time() {
    let config = parse_lifecycle_config(
        &json!({"failure": {"stack": [
            {"action": {"message": "phase {{phase}} failed: {{err.msg}}"}}
        ]}}),
        Path::new("t.md"),
    )
    .unwrap();
    let fm = map(json!({"phase": 6}));
    let err = io_err("disk full");
    let (_dir, engine) = temp_engine();
    let shell = MockShell::new(0);
    let recorder = Recorder::default();
    let harness = Harness::default();
    let context = ctx(
        LifecycleSignal::Failure,
        &fm,
        Some(&err),
        &engine,
        &shell,
        &recorder,
        &harness,
        Path::new("t.md"),
    );
    context.execute_event(&config);
    assert_eq!(
        recorder.events(),
        vec![Emitted::Message("phase 6 failed: disk full".to_string())]
    );
}

/// Currentness: the same lifecycle config re-resolves `{{phase}}` against
/// each event's live frontmatter, so a loop message reflects the current
/// iteration's value (the raw deferred subtree stays the stored definition).
#[test]
fn message_reflects_current_frontmatter_per_event() {
    let config = parse_lifecycle_config(
        &json!({"success": {"stack": [{"action": {"message": "iter {{phase}}"}}]}}),
        Path::new("t.md"),
    )
    .unwrap();
    let (_dir, engine) = temp_engine();
    let shell = MockShell::new(0);
    let harness = Harness::default();
    for (phase, expected) in [(1u64, "iter 1"), (2u64, "iter 2")] {
        let fm = map(json!({ "phase": phase }));
        let recorder = Recorder::default();
        let context = ctx(
            LifecycleSignal::Success,
            &fm,
            None,
            &engine,
            &shell,
            &recorder,
            &harness,
            Path::new("t.md"),
        );
        context.execute_event(&config);
        assert_eq!(recorder.events(), vec![Emitted::Message(expected.to_string())]);
    }
}

/// Just-in-time resolution: stack action #1 runs `set_frontmatter` on the
/// document; action #2 references that key and sees the mutated value.
#[test]
fn stack_action_sees_prior_set_frontmatter() {
    let config = parse_lifecycle_config(
        &json!({"success": {"stack": [{"action": [
            {"set_frontmatter": ["t.md", "status", "done"]},
            {"message": "{{status}}"}
        ]}]}}),
        Path::new("t.md"),
    )
    .unwrap();
    let fm = map(json!({"status": "pending"}));
    let (dir, engine) = temp_engine();
    std::fs::write(
        dir.path().join("t.md"),
        "---\nstatus: pending\n---\nbody\n",
    )
    .unwrap();
    let shell = MockShell::new(0);
    let recorder = Recorder::default();
    let harness = Harness::default();
    // `source_path` is the bare file name so it resolves against the engine
    // mutation root identically to the `set_frontmatter` target.
    let context = ctx(
        LifecycleSignal::Success,
        &fm,
        None,
        &engine,
        &shell,
        &recorder,
        &harness,
        Path::new("t.md"),
    );
    let outcome = context.execute_event(&config);
    assert_eq!(outcome, LifecycleEventOutcome::default());
    assert_eq!(recorder.events(), vec![Emitted::Message("done".to_string())]);
}

/// Cross-event visibility (review-2 High finding): a `start.stack`
/// `set_frontmatter` mutation persists into the shared per-attempt live cell,
/// so a *later* event's top-level `success.message` AND `finalize.message`
/// — each built from a separate context sharing the same cell — interpolate
/// the MUTATED value, not the original composed value. This is the harness
/// orchestration contract (`start` → `success`/`finalize`) driven at the
/// `StackExecutionContext` + shared `RefCell` seam.
#[test]
fn frontmatter_mutation_in_start_is_visible_to_later_events() {
    let config = parse_lifecycle_config(
        &json!({
            "start": {"stack": [{"action": {"set_frontmatter": ["t.md", "status", "running"]}}]},
            "success": {"message": "status={{status}}"},
            "finalize": {"message": "final={{status}}"}
        }),
        Path::new("t.md"),
    )
    .unwrap();

    // Composed base frontmatter (the original value the harness would carry
    // immutably) and the shared live cell seeded from it.
    let base = map(json!({"status": "pending"}));
    let live = std::cell::RefCell::new(base.clone());

    let (dir, engine) = temp_engine();
    std::fs::write(dir.path().join("t.md"), "---\nstatus: pending\n---\nbody\n").unwrap();
    let shell = MockShell::new(0);
    let harness = Harness::default();

    // start: runs the stack that mutates the document frontmatter.
    {
        let recorder = Recorder::default();
        let context = ctx_with_live(
            LifecycleSignal::Start,
            &base,
            &live,
            &engine,
            &shell,
            &recorder,
            &harness,
            Path::new("t.md"),
        );
        let outcome = context.execute_event(&config);
        assert_eq!(outcome, LifecycleEventOutcome::default());
    }
    // The mutation persisted into the shared cell.
    assert_eq!(live.borrow().get("status"), Some(&json!("running")));

    // success: a separate context sharing the same live cell sees `running`.
    {
        let recorder = Recorder::default();
        let context = ctx_with_live(
            LifecycleSignal::Success,
            &base,
            &live,
            &engine,
            &shell,
            &recorder,
            &harness,
            Path::new("t.md"),
        );
        context.execute_event(&config);
        assert_eq!(
            recorder.events(),
            vec![Emitted::Message("status=running".to_string())]
        );
    }

    // finalize: likewise sees the mutated value, not the original `pending`.
    {
        let recorder = Recorder::default();
        let context = ctx_with_live(
            LifecycleSignal::Finalize,
            &base,
            &live,
            &engine,
            &shell,
            &recorder,
            &harness,
            Path::new("t.md"),
        );
        context.execute_event(&config);
        assert_eq!(
            recorder.events(),
            vec![Emitted::Message("final=running".to_string())]
        );
    }
}

/// Negative control: with `live_frontmatter: None` (single-event caller),
/// a frontmatter mutation in one event's context does NOT leak into a later
/// event built from its own base frontmatter — behavior is unchanged from
/// before the cross-event cell existed. The `success` context, given the
/// original base, resolves `{{status}}` against `pending`.
#[test]
fn without_live_cell_later_event_resolves_against_its_own_base() {
    let config = parse_lifecycle_config(
        &json!({
            "start": {"stack": [{"action": {"set_frontmatter": ["t.md", "status", "running"]}}]},
            "success": {"message": "status={{status}}"}
        }),
        Path::new("t.md"),
    )
    .unwrap();

    let (dir, engine) = temp_engine();
    std::fs::write(dir.path().join("t.md"), "---\nstatus: pending\n---\nbody\n").unwrap();
    let shell = MockShell::new(0);
    let harness = Harness::default();

    // start: single-event context (no shared cell). Its stack mutation is
    // visible only intra-stack and discarded when the stack returns.
    let start_fm = map(json!({"status": "pending"}));
    {
        let recorder = Recorder::default();
        let context = ctx(
            LifecycleSignal::Start,
            &start_fm,
            None,
            &engine,
            &shell,
            &recorder,
            &harness,
            Path::new("t.md"),
        );
        context.execute_event(&config);
    }

    // success: a fresh single-event context with the ORIGINAL base sees the
    // original value, proving the None path carries no cross-event state.
    let success_fm = map(json!({"status": "pending"}));
    let recorder = Recorder::default();
    let context = ctx(
        LifecycleSignal::Success,
        &success_fm,
        None,
        &engine,
        &shell,
        &recorder,
        &harness,
        Path::new("t.md"),
    );
    context.execute_event(&config);
    assert_eq!(
        recorder.events(),
        vec![Emitted::Message("status=pending".to_string())]
    );
}

/// Parity: event-time rendering of a representative string equals what
/// Darkmatter's subtree compose produces for the same string with the same
/// data — there is no second interpolation engine.
#[test]
fn event_time_rendering_matches_compose() {
    use darkmatter::markdown::compose::EffectiveStateBuilder;
    use darkmatter::markdown::compose::subtree::{SubtreeStrictness, compose_subtree};

    let template = "phase {{phase}}: {{err.msg}}";
    let err = io_err("disk full");

    // Executor path.
    let config = parse_lifecycle_config(
        &json!({"failure": {"stack": [{"action": {"message": template}}]}}),
        Path::new("t.md"),
    )
    .unwrap();
    let fm = map(json!({"phase": 6}));
    let (_dir, engine) = temp_engine();
    let shell = MockShell::new(0);
    let recorder = Recorder::default();
    let harness = Harness::default();
    let context = ctx(
        LifecycleSignal::Failure,
        &fm,
        Some(&err),
        &engine,
        &shell,
        &recorder,
        &harness,
        Path::new("t.md"),
    );
    context.execute_event(&config);
    let Emitted::Message(executor_text) = &recorder.events()[0] else {
        panic!("expected a Message emission");
    };

    // Direct DM2 subtree compose for the same string + data.
    let state = EffectiveStateBuilder::new()
        .with_frontmatter(
            [("phase".to_string(), json!(6))].into_iter().collect(),
        )
        .with_context(
            darkmatter::markdown::compose::ComposeContext::capture_for_content(
                Path::new("."),
                "",
            ),
        )
        .build()
        .unwrap();
    let compose_value = compose_subtree(
        &json!(template),
        &state,
        lifecycle_injected_globals(Some(&err), None, None),
        SubtreeStrictness::Lenient,
    )
    .unwrap();
    assert_eq!(executor_text, compose_value.as_str().unwrap());
    assert_eq!(executor_text, "phase 6: disk full");
}

/// Phase 7 reproduction fixture (acceptance criterion 1): a top-level
/// `failure` block shaped like `prompts/implement-plan.md` — both a `say`
/// and a `message` field mixing an early-binding frontmatter span
/// (`{{phase}}`) with the late-binding `err` global — renders the real
/// values when the failure event fires. This is the original bug: before
/// late binding, `{{err.msg}}` collapsed to empty at compose time.
#[test]
fn reproduction_failure_block_renders_real_error_at_event_time() {
    let config = parse_lifecycle_config(
        &json!({"failure": {
            "say": "Phase {{phase}} ran into problems!",
            "message": "❌️ phase {{phase}} failed: {{err.msg}}",
        }}),
        Path::new("t.md"),
    )
    .unwrap();
    let fm = map(json!({"phase": 6}));
    let err = io_err("disk full");
    let (_dir, engine) = temp_engine();
    let shell = MockShell::new(0);
    let recorder = Recorder::default();
    let harness = Harness::default();
    let context = ctx(
        LifecycleSignal::Failure,
        &fm,
        Some(&err),
        &engine,
        &shell,
        &recorder,
        &harness,
        Path::new("t.md"),
    );
    context.execute_event(&config);
    assert_eq!(
        recorder.events(),
        vec![
            Emitted::Message("❌️ phase 6 failed: disk full".to_string()),
            Emitted::Speech("Phase 6 ran into problems!".to_string()),
        ]
    );
}

// ── Phase 5 (C4): fail-closed event-time resolution ─────────────────

/// A reference whose root is a *known* frontmatter key that resolves to
/// `null`/empty renders empty and does **not** error (5.6).
#[test]
fn known_but_empty_reference_renders_empty() {
    let config = parse_lifecycle_config(
        &json!({"success": {"stack": [{"action": {"message": "spec={{spec_file}}"}}]}}),
        Path::new("t.md"),
    )
    .unwrap();
    let fm = map(json!({"spec_file": null}));
    let (_dir, engine) = temp_engine();
    let shell = MockShell::new(0);
    let recorder = Recorder::default();
    let harness = Harness::default();
    let context = ctx(
        LifecycleSignal::Success,
        &fm,
        None,
        &engine,
        &shell,
        &recorder,
        &harness,
        Path::new("t.md"),
    );
    let outcome = context.execute_event(&config);
    assert_eq!(outcome, LifecycleEventOutcome::default());
    assert_eq!(recorder.events(), vec![Emitted::Message("spec=".to_string())]);
}

/// A typo (an unknown root) fails closed: the action errors and nothing is
/// dispatched (5.6).
#[test]
fn unknown_root_typo_fails_closed() {
    let config = parse_lifecycle_config(
        &json!({"success": {"stack": [{"action": {"message": "{{spec_fil}}"}}]}}),
        Path::new("t.md"),
    )
    .unwrap();
    let fm = map(json!({"spec_file": "x"}));
    let (_dir, engine) = temp_engine();
    let shell = MockShell::new(0);
    let recorder = Recorder::default();
    let harness = Harness::default();
    let context = ctx(
        LifecycleSignal::Success,
        &fm,
        None,
        &engine,
        &shell,
        &recorder,
        &harness,
        Path::new("t.md"),
    );
    let outcome = context.execute_event(&config);
    assert!(
        outcome.evaluation_error.is_some(),
        "typo must fail closed through the evaluation channel"
    );
    assert!(outcome.action_error.is_none());
    assert!(recorder.events().is_empty(), "nothing dispatched");
}

/// A top-level field with an unknown root fails the event closed before any
/// side effect is dispatched (5.5).
#[test]
fn top_level_unknown_root_fails_event_closed() {
    let config = parse_lifecycle_config(
        &json!({"success": {"message": "{{spec_fil}}"}}),
        Path::new("t.md"),
    )
    .unwrap();
    let fm = map(json!({}));
    let (_dir, engine) = temp_engine();
    let shell = MockShell::new(0);
    let recorder = Recorder::default();
    let harness = Harness::default();
    let context = ctx(
        LifecycleSignal::Success,
        &fm,
        None,
        &engine,
        &shell,
        &recorder,
        &harness,
        Path::new("t.md"),
    );
    let outcome = context.execute_event(&config);
    assert!(
        outcome.evaluation_error.is_some(),
        "a top-level interpolation raise is an evaluation error"
    );
    assert!(outcome.action_error.is_none());
    assert!(recorder.events().is_empty());
}

/// Post-DM2 leak guard (5.4): a known reference whose resolved value is
/// itself raw template text leaves a surviving `{{ … }}` span, which fails
/// before dispatch.
#[test]
fn post_dm2_surviving_span_fails_before_dispatch() {
    let config = parse_lifecycle_config(
        &json!({"success": {"stack": [{"action": {"message": "{{tmpl}}"}}]}}),
        Path::new("t.md"),
    )
    .unwrap();
    // The frontmatter value is literal template text — resolving `{{tmpl}}`
    // yields `{{x}}`, a surviving recognized span.
    let fm = map(json!({"tmpl": "{{x}}"}));
    let (_dir, engine) = temp_engine();
    let shell = MockShell::new(0);
    let recorder = Recorder::default();
    let harness = Harness::default();
    let context = ctx(
        LifecycleSignal::Success,
        &fm,
        None,
        &engine,
        &shell,
        &recorder,
        &harness,
        Path::new("t.md"),
    );
    let outcome = context.execute_event(&config);
    assert!(
        outcome.evaluation_error.is_some(),
        "surviving span is an evaluation-layer failure"
    );
    assert!(outcome.action_error.is_none());
    assert!(recorder.events().is_empty(), "no side effect dispatched");
}

/// Deferred effect validation (5.7): an `effect({{name}})` whose resolved
/// name is not in the catalog reports `LifecycleUnknownEffect` and dispatches
/// nothing.
#[test]
fn deferred_effect_invalid_resolved_name_reports_unknown_effect() {
    let config = parse_lifecycle_config(
        &json!({"success": {"stack": [{"action": {"effect": "{{effect_name}}"}}]}}),
        Path::new("t.md"),
    )
    .unwrap();
    let fm = map(json!({"effect_name": "nonexistent-effect-xyz"}));
    let (_dir, engine) = temp_engine();
    let shell = MockShell::new(0);
    let recorder = Recorder::default();
    let harness = Harness::default();
    let context = ctx(
        LifecycleSignal::Success,
        &fm,
        None,
        &engine,
        &shell,
        &recorder,
        &harness,
        Path::new("t.md"),
    );
    let outcome = context.execute_event(&config);
    let info = outcome.action_error.expect("invalid effect fails closed");
    assert_eq!(info.variant, "LifecycleUnknownEffect");
    assert!(recorder.events().is_empty(), "no effect dispatched");
}

/// A deferred effect whose resolved name *is* in the catalog dispatches
/// normally.
#[test]
fn deferred_effect_valid_resolved_name_dispatches() {
    let config = parse_lifecycle_config(
        &json!({"success": {"stack": [{"action": {"effect": "{{effect_name}}"}}]}}),
        Path::new("t.md"),
    )
    .unwrap();
    let fm = map(json!({"effect_name": "confirmation"}));
    let (_dir, engine) = temp_engine();
    let shell = MockShell::new(0);
    let recorder = Recorder::default();
    let harness = Harness::default();
    let context = ctx(
        LifecycleSignal::Success,
        &fm,
        None,
        &engine,
        &shell,
        &recorder,
        &harness,
        Path::new("t.md"),
    );
    let outcome = context.execute_event(&config);
    assert_eq!(outcome, LifecycleEventOutcome::default());
    assert_eq!(
        recorder.events(),
        vec![Emitted::Effect("confirmation".to_string())]
    );
}

/// `ctx.*` capture must follow `ctx_base_dir` (the launch area) when set,
/// not `base_dir` (the prompt's parent). Regression for lifecycle messages
/// interpolating `{{ctx.*}}` against the prompt file's directory instead of
/// the directory the caller launched from.
///
/// Uses `ctx.repo_root` as a directory-sensitive probe: each temp dir is its
/// own git repo, so the discovery resolves to whichever directory the
/// capture is rooted at — deterministic and cross-platform (no
/// monorepo/cargo fixture needed).
#[test]
fn ctx_capture_follows_ctx_base_dir_not_base_dir() {
    let git_init = |dir: &Path| {
        let ok = std::process::Command::new("git")
            .arg("init")
            .arg("--quiet")
            .current_dir(dir)
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        assert!(ok, "git init must succeed in {}", dir.display());
    };

    let ctx_dir = tempfile::tempdir().unwrap();
    let base_dir = tempfile::tempdir().unwrap();
    git_init(ctx_dir.path());
    git_init(base_dir.path());

    // Canonicalize: macOS temp dirs are symlinks (`/var` → `/private/var`),
    // and sniff reports the canonical repo root.
    let ctx_root = std::fs::canonicalize(ctx_dir.path()).unwrap();
    let base_root = std::fs::canonicalize(base_dir.path()).unwrap();

    let (_engine_dir, engine) = temp_engine();
    let shell = MockShell::new(0);
    let recorder = Recorder::default();
    let harness = Harness::default();
    let fm = Map::new();
    let source_path = ctx_root.join("prompt.md");

    let context = StackExecutionContext {
        signal: LifecycleSignal::Start,
        frontmatter: &fm,
        live_frontmatter: None,
        err: None,
        timing: None,
        current: None,
        // `base_dir` deliberately differs from `ctx_base_dir` so a leak back
        // to `base_dir` would resolve to `base_root` and fail the assert.
        base_dir: Some(base_root.as_path()),
        ctx_base_dir: Some(ctx_root.as_path()),
        // No prepared snapshot: exercise the fallback re-capture path so the
        // assertion proves `ctx_base_dir` (not `base_dir`) roots the capture.
        prepared_context: None,
        effect_engine: &engine,
        shell_runner: &shell,
        emitter: &recorder,
        term: &harness.term,
        source_path: &source_path,
        repo_root: None,
        messaging: &harness.messaging,
        settings: &harness.settings,
    };

    let resolved = context
        .resolve_string_value("{{ctx.repo_root}}", &fm)
        .expect("ctx.repo_root resolves");
    let resolved = resolved.as_str().unwrap_or_default();
    assert_eq!(
        resolved,
        ctx_root.to_string_lossy(),
        "ctx.* must capture against ctx_base_dir (launch area), not base_dir"
    );
    assert_ne!(
        resolved,
        base_root.to_string_lossy(),
        "ctx.* must not leak to base_dir"
    );
}

/// End-to-end of the exact layout that let the bug regress: a prompt living
/// OUTSIDE any area (`<repo>/prompts`) while the run was launched FROM a
/// different area. The single composition-start snapshot is captured against
/// the launch area and threaded as `prepared_context`; the lifecycle event
/// reuses it for `{{ctx.*}}` instead of re-capturing against the prompt's
/// parent (`base_dir`).
///
/// Probes `ctx.repo_root` (directory-sensitive, only needs `git init`).
/// The snapshot is rooted at `launch_root`; `base_dir` points at the
/// prompt's parentless-of-area `prompts/` dir inside a *different* repo, so
/// the pre-fix re-capture would have produced `base_root`, not `launch_root`.
#[test]
fn lifecycle_reuses_prepared_snapshot_for_prompt_outside_launch_area() {
    let git_init = |dir: &Path| {
        let ok = std::process::Command::new("git")
            .arg("init")
            .arg("--quiet")
            .current_dir(dir)
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        assert!(ok, "git init must succeed in {}", dir.display());
    };

    // The launch area: the package area the caller launched from.
    let launch_dir = tempfile::tempdir().unwrap();
    git_init(launch_dir.path());
    let launch_root = std::fs::canonicalize(launch_dir.path()).unwrap();

    // A separate repo whose `prompts/` subdir holds the prompt file — the
    // "prompt outside any area" shape. `base_dir` points here.
    let prompt_repo = tempfile::tempdir().unwrap();
    git_init(prompt_repo.path());
    let prompt_repo_root = std::fs::canonicalize(prompt_repo.path()).unwrap();
    let prompts_dir = prompt_repo_root.join("prompts");
    std::fs::create_dir(&prompts_dir).unwrap();
    let source_path = prompts_dir.join("implement-plan.md");

    // The single composition-start snapshot, captured ONCE against the
    // launch area (mirrors what the CLI does in `compose/prep.rs`).
    let prepared = ComposeContext::capture_for_content(
        launch_root.as_path(),
        "{{ ctx.repo_root }}",
    );

    let (_engine_dir, engine) = temp_engine();
    let shell = MockShell::new(0);
    let recorder = Recorder::default();
    let harness = Harness::default();
    let fm = Map::new();

    let context = StackExecutionContext {
        signal: LifecycleSignal::Initialize,
        frontmatter: &fm,
        live_frontmatter: None,
        err: None,
        timing: None,
        current: None,
        // The prompt's parent — inside a different repo, no area.
        base_dir: Some(prompts_dir.as_path()),
        ctx_base_dir: Some(launch_root.as_path()),
        // The reused snapshot is the source of truth.
        prepared_context: Some(&prepared),
        effect_engine: &engine,
        shell_runner: &shell,
        emitter: &recorder,
        term: &harness.term,
        source_path: &source_path,
        repo_root: None,
        messaging: &harness.messaging,
        settings: &harness.settings,
    };

    let resolved = context
        .resolve_string_value("{{ctx.repo_root}}", &fm)
        .expect("ctx.repo_root resolves");
    let resolved = resolved.as_str().unwrap_or_default();
    assert_eq!(
        resolved,
        launch_root.to_string_lossy(),
        "lifecycle must reuse the launch-area snapshot, not the prompt dir"
    );
    assert_ne!(
        resolved,
        prompt_repo_root.to_string_lossy(),
        "lifecycle ctx.* must not resolve against the prompt's own repo"
    );
}

/// A caller-supplied file reference resolves against the captured launch
/// area (`ctx_base_dir`) after the ambient process CWD has moved away from
/// it — the core post-`chdir` independence contract.
///
/// Layout: the `spec` file lives ONLY under the launch area; `base_dir`
/// (the prompt's parent) does not contain it, and the ambient CWD is an
/// unrelated directory. Pre-fix, `file_exists(spec)` returned `false`
/// because resolution fell back to the mutated ambient CWD instead of the
/// captured launch area.
#[serial_test::serial]
#[test]
fn file_exists_resolves_against_launch_area_after_chdir() {
    let launch_dir = tempfile::tempdir().unwrap();
    let prompt_dir = tempfile::tempdir().unwrap();
    let unrelated_dir = tempfile::tempdir().unwrap();

    // The caller-supplied file lives only under the launch area.
    let spec_path = launch_dir.path().join("spec.md");
    std::fs::write(&spec_path, "# spec\n").unwrap();

    // Move the ambient CWD to an unrelated dir (mirrors the wrapper's
    // `switch_process_cwd` to the repo root before lifecycle events fire).
    let original_cwd = std::env::current_dir().unwrap();
    std::env::set_current_dir(unrelated_dir.path()).unwrap();

    let (_engine_dir, engine) = temp_engine();
    let shell = MockShell::new(0);
    let recorder = Recorder::default();
    let harness = Harness::default();
    let fm = map(json!({ "spec": "spec.md" }));
    let source_path = prompt_dir.path().join("prompt.md");

    let context = StackExecutionContext {
        signal: LifecycleSignal::Initialize,
        frontmatter: &fm,
        live_frontmatter: None,
        err: None,
        timing: None,
        current: None,
        // The prompt's parent — does NOT contain spec.md.
        base_dir: Some(prompt_dir.path()),
        // The launch area — the one anchor spec.md is relative to.
        ctx_base_dir: Some(launch_dir.path()),
        prepared_context: None,
        effect_engine: &engine,
        shell_runner: &shell,
        emitter: &recorder,
        term: &harness.term,
        source_path: &source_path,
        repo_root: None,
        messaging: &harness.messaging,
        settings: &harness.settings,
    };

    let resolved = context
        .resolve_string_value("{{file_exists(spec)}}", &fm)
        .expect("file_exists(spec) must resolve");
    assert_eq!(
        resolved,
        Value::Bool(true),
        "file_exists must hit the launch-area fallback, not the ambient CWD"
    );

    // Restore the ambient CWD so other tests are unaffected.
    std::env::set_current_dir(&original_cwd).unwrap();
}

/// Prepare-time and event-time resolution agree for the same caller-supplied
/// path: both anchor on the launch area. This asserts the event-time
/// `StackExecutionContext` path (`file_exists` → `true`) matches what the
/// `ResolutionContext` builder alone produces — the two paths share one
/// explicit anchor instead of diverging on ambient-CWD timing.
#[serial_test::serial]
#[test]
fn prepare_time_and_event_time_agree_on_file_reference() {
    let launch_dir = tempfile::tempdir().unwrap();
    let prompt_dir = tempfile::tempdir().unwrap();

    let spec_path = launch_dir.path().join("plan.md");
    std::fs::write(&spec_path, "# plan\n").unwrap();

    let original_cwd = std::env::current_dir().unwrap();
    std::env::set_current_dir(prompt_dir.path()).unwrap();

    // Prepare-time resolution context (mirrors what ComposeOptions builds):
    let prepare_ctx = ResolutionContext::new(prompt_dir.path().to_path_buf())
        .with_file_ref_fallback_dir(launch_dir.path().to_path_buf());

    // Event-time resolution context (built by StackExecutionContext):
    let (_engine_dir, engine) = temp_engine();
    let shell = MockShell::new(0);
    let recorder = Recorder::default();
    let harness = Harness::default();
    let fm = map(json!({ "spec": "plan.md" }));
    let source_path = prompt_dir.path().join("prompt.md");
    let context = StackExecutionContext {
        signal: LifecycleSignal::Initialize,
        frontmatter: &fm,
        live_frontmatter: None,
        err: None,
        timing: None,
        current: None,
        base_dir: Some(prompt_dir.path()),
        ctx_base_dir: Some(launch_dir.path()),
        prepared_context: None,
        effect_engine: &engine,
        shell_runner: &shell,
        emitter: &recorder,
        term: &harness.term,
        source_path: &source_path,
        repo_root: None,
        messaging: &harness.messaging,
        settings: &harness.settings,
    };
    let event_ctx = context.resolution_context();

    // Both contexts carry the same fallback directory.
    assert_eq!(
        prepare_ctx.file_ref_fallback_dir, event_ctx.file_ref_fallback_dir,
        "prepare-time and event-time must share the launch-area fallback"
    );
    // Both base dirs point at the prompt's parent.
    assert_eq!(prepare_ctx.base_dir, event_ctx.base_dir);

    // The event-time file_exists agrees with the prepare-time anchor.
    let resolved = context
        .resolve_string_value("{{file_exists(spec)}}", &fm)
        .expect("file_exists(spec) resolves");
    assert_eq!(resolved, Value::Bool(true));

    std::env::set_current_dir(&original_cwd).unwrap();
}

/// `frontmatter(spec, review_iterations)` resolves against the launch-area
/// fallback — the mechanism behind `iteration` derivation in prompts like
/// `review-feature.md`. Pre-fix this returned empty/null because the spec
/// file resolved as missing, leaving `iteration` stuck at `1`.
#[serial_test::serial]
#[test]
fn frontmatter_reads_resolve_against_launch_area_fallback() {
    let launch_dir = tempfile::tempdir().unwrap();
    let prompt_dir = tempfile::tempdir().unwrap();

    // The spec file carries a `review_iterations` frontmatter property.
    let spec_path = launch_dir.path().join("spec.md");
    std::fs::write(
        &spec_path,
        "---\nreview_iterations: 3\n---\n# spec\n",
    )
    .unwrap();

    let original_cwd = std::env::current_dir().unwrap();
    std::env::set_current_dir(prompt_dir.path()).unwrap();

    let (_engine_dir, engine) = temp_engine();
    let shell = MockShell::new(0);
    let recorder = Recorder::default();
    let harness = Harness::default();
    let fm = map(json!({ "spec": "spec.md" }));
    let source_path = prompt_dir.path().join("prompt.md");

    let context = StackExecutionContext {
        signal: LifecycleSignal::Initialize,
        frontmatter: &fm,
        live_frontmatter: None,
        err: None,
        timing: None,
        current: None,
        base_dir: Some(prompt_dir.path()),
        ctx_base_dir: Some(launch_dir.path()),
        prepared_context: None,
        effect_engine: &engine,
        shell_runner: &shell,
        emitter: &recorder,
        term: &harness.term,
        source_path: &source_path,
        repo_root: None,
        messaging: &harness.messaging,
        settings: &harness.settings,
    };

    let resolved = context
        .resolve_string_value("{{frontmatter(spec, 'review_iterations')}}", &fm)
        .expect("frontmatter(spec, 'review_iterations') resolves");
    assert_eq!(
        resolved,
        Value::Number(3.into()),
        "frontmatter() must read the spec via the launch-area fallback"
    );

    std::env::set_current_dir(&original_cwd).unwrap();
}

// ── Phase 4 regression suite ────────────────────────────────────────────
//
// These tests close the remaining verification goals from the spec with
// explicit regression coverage at the claudine lifecycle layer.

/// A caller-supplied file that exists ONLY under the launch area (not
/// under the prompt dir, not under the post-`chdir` ambient CWD which
/// represents the repo root) resolves through the fallback — proving the
/// new fallback is the source of the hit (verification goal #8).
///
/// Three distinct anchors are materialized so the test cannot pass by
/// accident: `prompt_dir` (base_dir, empty), `repo_root_dir` (the
/// post-`chdir` ambient CWD, empty), and `launch_dir` (ctx_base_dir /
/// fallback, holds the only copy).
#[serial_test::serial]
#[test]
fn regression_path_only_under_launch_area_resolves() {
    let launch_dir = tempfile::tempdir().unwrap();
    let prompt_dir = tempfile::tempdir().unwrap();
    let repo_root_dir = tempfile::tempdir().unwrap();

    // The caller-supplied file lives ONLY under the launch area.
    std::fs::write(launch_dir.path().join("unique.md"), "# unique\n").unwrap();
    // Defensive sanity: neither other anchor holds the file.
    assert!(!prompt_dir.path().join("unique.md").exists());
    assert!(!repo_root_dir.path().join("unique.md").exists());

    // The wrapper's `switch_process_cwd` repositions the process to the
    // repo root before lifecycle events fire.
    let original_cwd = std::env::current_dir().unwrap();
    std::env::set_current_dir(repo_root_dir.path()).unwrap();

    let (_engine_dir, engine) = temp_engine();
    let shell = MockShell::new(0);
    let recorder = Recorder::default();
    let harness = Harness::default();
    let fm = map(json!({ "spec": "unique.md" }));
    let source_path = prompt_dir.path().join("prompt.md");

    let context = StackExecutionContext {
        signal: LifecycleSignal::Initialize,
        frontmatter: &fm,
        live_frontmatter: None,
        err: None,
        timing: None,
        current: None,
        base_dir: Some(prompt_dir.path()),
        ctx_base_dir: Some(launch_dir.path()),
        prepared_context: None,
        effect_engine: &engine,
        shell_runner: &shell,
        emitter: &recorder,
        term: &harness.term,
        source_path: &source_path,
        repo_root: None,
        messaging: &harness.messaging,
        settings: &harness.settings,
    };

    let resolved = context
        .resolve_string_value("{{file_exists(spec)}}", &fm)
        .expect("file_exists(spec) must resolve");
    assert_eq!(
        resolved,
        Value::Bool(true),
        "the launch-area fallback is the only anchor that holds unique.md; resolution must \
         prove the fallback (not prompt dir, not repo root) was consulted",
    );

    std::env::set_current_dir(&original_cwd).unwrap();
}

/// An intentionally conflicting filename present in BOTH the prompt dir
/// and the launch area resolves to the prompt-dir copy — the
/// document-first contract holds end-to-end at the lifecycle layer
/// (verification goal #9, re-affirmed).
///
/// Each copy carries a distinct `title` frontmatter property so the
/// `frontmatter(spec, 'title')` value identifies which file won.
#[serial_test::serial]
#[test]
fn regression_conflicting_filename_prompt_dir_wins() {
    let launch_dir = tempfile::tempdir().unwrap();
    let prompt_dir = tempfile::tempdir().unwrap();
    let repo_root_dir = tempfile::tempdir().unwrap();

    // Both anchors hold a same-named spec.md with distinct titles.
    std::fs::write(
        prompt_dir.path().join("spec.md"),
        "---\ntitle: from-prompt-dir\n---\n# prompt\n",
    )
    .unwrap();
    std::fs::write(
        launch_dir.path().join("spec.md"),
        "---\ntitle: from-launch-area\n---\n# launch\n",
    )
    .unwrap();

    let original_cwd = std::env::current_dir().unwrap();
    std::env::set_current_dir(repo_root_dir.path()).unwrap();

    let (_engine_dir, engine) = temp_engine();
    let shell = MockShell::new(0);
    let recorder = Recorder::default();
    let harness = Harness::default();
    let fm = map(json!({ "spec": "spec.md" }));
    let source_path = prompt_dir.path().join("prompt.md");

    let context = StackExecutionContext {
        signal: LifecycleSignal::Initialize,
        frontmatter: &fm,
        live_frontmatter: None,
        err: None,
        timing: None,
        current: None,
        base_dir: Some(prompt_dir.path()),
        ctx_base_dir: Some(launch_dir.path()),
        prepared_context: None,
        effect_engine: &engine,
        shell_runner: &shell,
        emitter: &recorder,
        term: &harness.term,
        source_path: &source_path,
        repo_root: None,
        messaging: &harness.messaging,
        settings: &harness.settings,
    };

    let resolved = context
        .resolve_string_value("{{frontmatter(spec, 'title')}}", &fm)
        .expect("frontmatter(spec, 'title') must resolve");
    assert_eq!(
        resolved,
        Value::String("from-prompt-dir".to_string()),
        "document-first contract: the prompt-dir copy must win over the launch-area fallback",
    );

    std::env::set_current_dir(&original_cwd).unwrap();
}

/// The `ctx.*` capture hint is demand-driven, so a read buried in a container
/// literal must still be collected — otherwise the expression evaluates
/// against an uncaptured sniff group. A non-recursing arm yields an empty hint.
#[test]
fn ctx_scan_hint_descends_container_literals() {
    use darkmatter::markdown::compose::expression::parse;

    let array = parse("[ctx.area, ctx.package]").expect("array literal must parse");
    let hint = ctx_scan_hint(&array);
    assert!(hint.contains("ctx.area"), "got: {hint}");
    assert!(hint.contains("ctx.package"), "got: {hint}");

    // Object values are expressions and contribute a hint; the key `ctx` is
    // authored text, not a reference, so it adds nothing.
    let object = parse("{ ctx: ctx.agent }").expect("object literal must parse");
    assert_eq!(ctx_scan_hint(&object), "ctx.agent");
}
