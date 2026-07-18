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
    fn run(&self, command: &str) -> Result<i32, ShellRunError> {
        self.commands.lock().unwrap().push(command.to_string());
        Ok(self.code)
    }
}

/// A [`ShellRunner`] whose command never starts, so the spawn-failure arm is
/// reachable without depending on the host shell.
struct SpawnFailShell;

impl SpawnFailShell {
    /// The `io::Error` every run reports, so a test can compare the projected
    /// prose against the exact source it was built from.
    fn io_error() -> std::io::Error {
        std::io::Error::new(std::io::ErrorKind::NotFound, "no such file or directory")
    }
}

impl ShellRunner for SpawnFailShell {
    fn run(&self, command: &str) -> Result<i32, ShellRunError> {
        Err(ShellRunError::Spawn {
            command: command.to_string(),
            source: Self::io_error(),
        })
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
        runtime_state: None,
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

/// Build a context wired to both the per-attempt `live` cell and the
/// invocation-local `runtime` cell, modelling the harness loop's full wiring.
#[allow(clippy::too_many_arguments)]
fn ctx_with_runtime<'a>(
    signal: LifecycleSignal,
    base: &'a Map<String, Value>,
    live: &'a std::cell::RefCell<Map<String, Value>>,
    runtime: &'a crate::composition::RuntimeState,
    engine: &'a EffectEngine,
    shell: &'a dyn ShellRunner,
    recorder: &'a Recorder,
    harness: &'a Harness,
    source_path: &'a Path,
) -> StackExecutionContext<'a> {
    StackExecutionContext {
        runtime_state: Some(runtime),
        ..ctx_with_live(signal, base, live, engine, shell, recorder, harness, source_path)
    }
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
        runtime_state: None,
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


mod action_dispatch;
mod conditions_control;
mod event_time_interpolation;
mod filesystem_lookup;
mod mutation_visibility;
mod runtime_set;

