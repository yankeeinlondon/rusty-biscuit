//! Phase 7 — atomic task execution and lifecycle semantics.
//!
//! Every test drives the public seam ([`TaskExecution::run`]) over a task node
//! produced by the *real* preflight walk against files on disk, because the
//! contract under test starts at the node: approved shell bytes, the authoring
//! directory a reference resolved from, and the composition mode of the
//! referenced document are all decided before execution and must survive it
//! unchanged.

use std::cell::RefCell;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::AtomicBool;

use biscuit_terminal::terminal::Terminal;
use darkmatter::effects::EffectEngine;
use darkmatter::markdown::compose::{ComposeContext, EffectiveStateBuilder};
use serde_json::{Value, json};
use tempfile::TempDir;

use super::*;
use crate::composition::error::CompositionError;
use crate::composition::lifecycle::executor::{ShellRunError, ShellRunner};
use crate::composition::lifecycle::{LifecycleEmitter, LifecycleSignal};
use crate::composition::sequence::preflight::build_preflight_graph_with_context;
use crate::composition::sequence::{build_step_overlay, resolve_sequence_plan};
use crate::events::GlobalSettings;
use crate::messaging::RuntimeMessagingSettings;

// -- fixtures ---------------------------------------------------------------

fn write_source(dir: &Path, name: &str, frontmatter: &[(&str, Value)], body: &str) -> String {
    let mut text = String::from("---\n");
    for (key, value) in frontmatter {
        text.push_str(&format!("{key}: {}\n", serde_json::to_string(value).unwrap()));
    }
    text.push_str("---\n\n");
    text.push_str(body);
    let path = dir.join(name);
    fs::write(&path, &text).unwrap();
    path.display().to_string()
}

fn write_yaml(dir: &Path, name: &str, value: &Value) {
    fs::write(dir.join(name), serde_json::to_string_pretty(value).unwrap()).unwrap();
}

/// A sequence document whose single step is the task under test.
fn one_step_source(dir: &Path, step: Value) -> String {
    write_source(
        dir,
        "seq.md",
        &[("sequence", json!([step]))],
        "Document body.\n",
    )
}

/// Everything one task execution needs, owned so the borrows taken in
/// [`Fixture::execute`] stay alive for the call.
struct Fixture {
    _dir: TempDir,
    graph: PreflightGraph,
    state: EffectiveState,
    frontmatter: Map<String, Value>,
    overlay: Value,
    engine: EffectEngine,
    _engine_dir: TempDir,
    term: Terminal,
    settings: GlobalSettings,
    messaging: RuntimeMessagingSettings,
    source_path: PathBuf,
}

impl Fixture {
    /// Build the preflight graph for a sequence document and capture the
    /// effective state its first step composes against.
    fn build(dir: TempDir, source: &str) -> Result<Self, CompositionError> {
        let resolved = crate::composition::resolve_composition_source(source)?;
        let plan = resolve_sequence_plan(&resolved)?.expect("fixture declares a sequence");
        // Demand-driven: no fixture references `ctx.*`, so the preflight walk is
        // handed a date/time-only context instead of probing git, the repo, the
        // OS, and the hardware once per test.
        let context = ComposeContext::capture_for_content(&PathBuf::from("."), "");
        let graph = build_preflight_graph_with_context(&plan, &resolved, context.clone())?;

        let overlay = build_step_overlay(&plan, 0).as_set_overrides(None);
        let mut frontmatter: Map<String, Value> = resolved
            .markdown
            .frontmatter()
            .as_map()
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        if let Value::Object(overrides) = &overlay {
            for (key, value) in overrides {
                frontmatter.insert(key.clone(), value.clone());
            }
        }
        let lookup: HashMap<String, Value> = frontmatter
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        let state = EffectiveStateBuilder::new()
            .with_frontmatter(lookup)
            // Demand-driven: no fixture references `ctx.*`, so this captures
            // date/time only and keeps the suite off git/repo/hardware probes.
            .with_context(context)
            .with_allow_ctx_override(true)
            .build()
            .unwrap();

        let engine_dir = TempDir::new().unwrap();
        let engine = EffectEngine::builder()
            .mutation_root(engine_dir.path())
            .auto_rehash(false)
            .build();

        Ok(Self {
            _dir: dir,
            graph,
            state,
            frontmatter,
            overlay,
            engine,
            _engine_dir: engine_dir,
            term: Terminal::default(),
            settings: GlobalSettings::default(),
            messaging: RuntimeMessagingSettings {
                user: None,
                repo: None,
            },
            source_path: PathBuf::from(source),
        })
    }

    fn task(&self) -> &PreflightTask {
        self.graph.steps[0]
            .task
            .as_ref()
            .expect("fixture's step declares an executable")
    }

    /// Run the fixture's task with the given doubles.
    fn execute(&self, wiring: &Wiring<'_>) -> TaskOutcome {
        let live = RefCell::new(self.frontmatter.clone());
        let stack = StackExecutionContext {
            signal: LifecycleSignal::Start,
            frontmatter: &self.frontmatter,
            live_frontmatter: Some(&live),
            runtime_state: wiring.runtime,
            err: None,
            timing: None,
            current: None,
            base_dir: None,
            ctx_base_dir: None,
            prepared_context: None,
            effect_engine: &self.engine,
            shell_runner: wiring.lifecycle_shell,
            emitter: wiring.recorder,
            term: &self.term,
            source_path: &self.source_path,
            repo_root: None,
            messaging: &self.messaging,
            settings: &self.settings,
        };
        TaskExecution {
            task: self.task(),
            graph: &self.graph,
            stack: &stack,
            state: &self.state,
            runtime: wiring.runtime,
            user_setters: wiring.user_setters,
            overlay: Some(&self.overlay),
            shell: wiring.shell,
            prompt: wiring.prompt,
            interrupt: wiring.interrupt,
        }
        .run()
    }
}

/// The injectable doubles for one execution.
struct Wiring<'a> {
    recorder: &'a Recorder,
    lifecycle_shell: &'a dyn ShellRunner,
    shell: &'a dyn TaskShellRunner,
    prompt: &'a dyn PromptTaskRunner,
    runtime: Option<&'a RuntimeState>,
    user_setters: Option<&'a Value>,
    interrupt: Option<&'a AtomicBool>,
}

impl<'a> Wiring<'a> {
    fn new(recorder: &'a Recorder, shell: &'a FakeTaskShell) -> Self {
        Self {
            recorder,
            lifecycle_shell: recorder,
            shell,
            prompt: &UnavailablePromptRunner,
            runtime: None,
            user_setters: None,
            interrupt: None,
        }
    }
}

/// Records every ordered observation a task makes on the outside world:
/// lifecycle emissions and lifecycle-stack shell commands.
#[derive(Default)]
struct Recorder {
    events: Mutex<Vec<String>>,
    /// Exit code every lifecycle-stack shell command reports.
    stack_shell_code: Mutex<i32>,
}

impl Recorder {
    fn events(&self) -> Vec<String> {
        self.events.lock().unwrap().clone()
    }
    fn push(&self, entry: String) {
        self.events.lock().unwrap().push(entry);
    }
    fn fail_stack_shell(&self) {
        *self.stack_shell_code.lock().unwrap() = 1;
    }
}

impl LifecycleEmitter for Recorder {
    fn emit_stderr(&self, _signal: LifecycleSignal, text: &str, _term: &Terminal) {
        self.push(format!("stderr:{text}"));
    }
    fn emit_message(
        &self,
        text: &str,
        _source_path: &Path,
        _repo_root: Option<&Path>,
        _messaging: &RuntimeMessagingSettings,
    ) {
        self.push(format!("message:{text}"));
    }
    fn emit_speech(&self, text: &str, _config: biscuit_speaks::TtsConfig) {
        self.push(format!("speech:{text}"));
    }
    fn emit_effect(&self, name: &str) {
        self.push(format!("effect:{name}"));
    }
    fn emit_notification(&self, title: &str) {
        self.push(format!("notify:{title}"));
    }
    fn emit_info(&self, text: &str, _term: &Terminal) {
        self.push(format!("info:{text}"));
    }
    fn emit_warn(&self, text: &str, _term: &Terminal) {
        self.push(format!("warn:{text}"));
    }
    fn emit_success(&self, text: &str, _term: &Terminal) {
        self.push(format!("success:{text}"));
    }
    fn emit_stdout(&self, text: &str, _term: &Terminal) {
        self.push(format!("stdout:{text}"));
    }
}

impl ShellRunner for Recorder {
    fn run(&self, command: &str) -> Result<i32, ShellRunError> {
        self.push(format!("stack-shell:{command}"));
        Ok(*self.stack_shell_code.lock().unwrap())
    }
}

/// A [`TaskShellRunner`] that records the exact bytes and timeout it received
/// and replays a programmed result per invocation.
#[derive(Default)]
struct FakeTaskShell {
    seen: Mutex<Vec<(String, Duration)>>,
    results: Mutex<Vec<ShellCommandOutput>>,
    spawn_failure: Mutex<bool>,
}

impl FakeTaskShell {
    fn with_stdout(lines: &[&str]) -> Self {
        let shell = Self::default();
        *shell.results.lock().unwrap() = lines
            .iter()
            .map(|line| ShellCommandOutput {
                stdout: format!("{line}\n"),
                exit_code: 0,
                timed_out: false,
            })
            .collect();
        shell
    }
    fn with_results(results: Vec<ShellCommandOutput>) -> Self {
        let shell = Self::default();
        *shell.results.lock().unwrap() = results;
        shell
    }
    fn failing_to_spawn() -> Self {
        let shell = Self::default();
        *shell.spawn_failure.lock().unwrap() = true;
        shell
    }
    fn commands(&self) -> Vec<String> {
        self.seen
            .lock()
            .unwrap()
            .iter()
            .map(|(command, _)| command.clone())
            .collect()
    }
    fn timeouts(&self) -> Vec<Duration> {
        self.seen
            .lock()
            .unwrap()
            .iter()
            .map(|(_, timeout)| *timeout)
            .collect()
    }
}

impl TaskShellRunner for FakeTaskShell {
    fn run(&self, command: &str, timeout: Duration) -> Result<ShellCommandOutput, std::io::Error> {
        self.seen
            .lock()
            .unwrap()
            .push((command.to_string(), timeout));
        if *self.spawn_failure.lock().unwrap() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "no such file or directory",
            ));
        }
        let mut results = self.results.lock().unwrap();
        if results.is_empty() {
            return Ok(ShellCommandOutput {
                stdout: String::new(),
                exit_code: 0,
                timed_out: false,
            });
        }
        Ok(results.remove(0))
    }
}

/// A [`PromptTaskRunner`] that records requests and replays one outcome.
struct FakePrompt {
    requests: Mutex<Vec<PromptTaskRequest>>,
    outcome: PromptRunOutcome,
}

impl FakePrompt {
    fn succeeding(stdout: &str) -> Self {
        Self::with(PromptRunOutcome {
            stdout: stdout.to_string(),
            exit_code: 0,
            interrupted: false,
        })
    }
    fn with(outcome: PromptRunOutcome) -> Self {
        Self {
            requests: Mutex::new(Vec::new()),
            outcome,
        }
    }
    fn last(&self) -> PromptTaskRequest {
        self.requests.lock().unwrap().last().cloned().unwrap()
    }
}

impl PromptTaskRunner for FakePrompt {
    fn run(&self, request: &PromptTaskRequest) -> Result<PromptRunOutcome, CompositionError> {
        self.requests.lock().unwrap().push(request.clone());
        Ok(self.outcome.clone())
    }
}

fn failure_message(outcome: &TaskOutcome) -> String {
    outcome
        .error
        .as_ref()
        .map(|d| d.message().to_string())
        .unwrap_or_else(|| format!("expected a failure, got {:?}", outcome.status))
}

// -- stage orchestration ----------------------------------------------------

mod stages {
    use super::*;

    #[test]
    fn setup_runs_before_the_primary_and_teardown_runs_after_it() {
        let dir = TempDir::new().unwrap();
        let source = one_step_source(
            dir.path(),
            json!({
                "name": "alpha",
                "shell": "primary-command",
                "setup": [{ "action": { "shell": "setup-command" } }],
                "teardown": [{ "action": { "shell": "teardown-command" } }],
            }),
        );
        let fixture = Fixture::build(dir, &source).unwrap();
        let recorder = Recorder::default();
        let shell = FakeTaskShell::with_stdout(&["primary out"]);

        let outcome = fixture.execute(&Wiring::new(&recorder, &shell));

        assert!(outcome.succeeded(), "{}", failure_message(&outcome));
        assert_eq!(
            recorder.events(),
            vec![
                "stack-shell:setup-command".to_string(),
                "stack-shell:teardown-command".to_string(),
            ],
        );
        assert_eq!(shell.commands(), vec!["primary-command".to_string()]);
        assert_eq!(outcome.stdout, "primary out");
    }

    #[test]
    fn a_failed_setup_skips_the_primary_but_still_runs_teardown() {
        let dir = TempDir::new().unwrap();
        let source = one_step_source(
            dir.path(),
            json!({
                "name": "alpha",
                "shell": "primary-command",
                "setup": [{ "action": { "shell": "setup-command" } }],
                "teardown": [{ "action": { "shell": "teardown-command" } }],
            }),
        );
        let fixture = Fixture::build(dir, &source).unwrap();
        let recorder = Recorder::default();
        recorder.fail_stack_shell();
        let shell = FakeTaskShell::with_stdout(&["primary out"]);

        let outcome = fixture.execute(&Wiring::new(&recorder, &shell));

        assert_eq!(outcome.status, TaskStatus::Failed);
        assert_eq!(outcome.error.as_ref().unwrap().stage, TaskStage::Setup);
        assert!(
            shell.commands().is_empty(),
            "the primary action must not run after a failed setup",
        );
        assert!(
            recorder
                .events()
                .contains(&"stack-shell:teardown-command".to_string()),
            "teardown is owed once setup has started: {:?}",
            recorder.events(),
        );
    }

    #[test]
    fn teardown_reads_the_primary_error_through_err() {
        let dir = TempDir::new().unwrap();
        let source = one_step_source(
            dir.path(),
            json!({
                "name": "alpha",
                "shell": "primary-command",
                "teardown": [{ "action": { "message": "cleanup after: {{ err.msg }}" } }],
            }),
        );
        let fixture = Fixture::build(dir, &source).unwrap();
        let recorder = Recorder::default();
        let shell = FakeTaskShell::with_results(vec![ShellCommandOutput {
            stdout: String::new(),
            exit_code: 3,
            timed_out: false,
        }]);

        let outcome = fixture.execute(&Wiring::new(&recorder, &shell));

        assert_eq!(outcome.status, TaskStatus::Failed);
        let emitted = recorder.events();
        assert_eq!(emitted.len(), 1, "{emitted:?}");
        assert!(
            emitted[0].contains("exited with code 3"),
            "teardown must see the primary error in `err`: {emitted:?}",
        );
    }

    #[test]
    fn a_failed_teardown_converts_a_successful_task_to_failure() {
        let dir = TempDir::new().unwrap();
        let source = one_step_source(
            dir.path(),
            json!({
                "name": "alpha",
                "shell": "primary-command",
                "teardown": [{ "action": { "shell": "teardown-command" } }],
            }),
        );
        let fixture = Fixture::build(dir, &source).unwrap();
        let recorder = Recorder::default();
        recorder.fail_stack_shell();
        let shell = FakeTaskShell::with_stdout(&["primary out"]);
        let runtime = RuntimeState::new();
        let mut wiring = Wiring::new(&recorder, &shell);
        wiring.runtime = Some(&runtime);

        let outcome = fixture.execute(&wiring);

        assert_eq!(outcome.status, TaskStatus::Failed);
        assert_eq!(outcome.error.as_ref().unwrap().stage, TaskStage::Teardown);
        assert!(outcome.secondary_errors.is_empty());
        assert!(
            !outcome.output_committed && runtime.output_count() == 0,
            "no later work may observe an output from a task teardown failed",
        );
    }

    #[test]
    fn a_primary_failure_stays_primary_and_a_teardown_failure_is_secondary() {
        let dir = TempDir::new().unwrap();
        let source = one_step_source(
            dir.path(),
            json!({
                "name": "alpha",
                "shell": "primary-command",
                "teardown": [{ "action": { "shell": "teardown-command" } }],
            }),
        );
        let fixture = Fixture::build(dir, &source).unwrap();
        let recorder = Recorder::default();
        recorder.fail_stack_shell();
        let shell = FakeTaskShell::with_results(vec![ShellCommandOutput {
            stdout: String::new(),
            exit_code: 7,
            timed_out: false,
        }]);

        let outcome = fixture.execute(&Wiring::new(&recorder, &shell));

        assert_eq!(outcome.error.as_ref().unwrap().stage, TaskStage::Primary);
        assert!(
            failure_message(&outcome).contains("exited with code 7"),
            "{}",
            failure_message(&outcome),
        );
        assert_eq!(outcome.secondary_errors.len(), 1);
        assert_eq!(outcome.secondary_errors[0].stage, TaskStage::Teardown);
    }

    #[test]
    fn no_error_keeps_its_dispatch_only_suppression_in_a_task_stack() {
        let dir = TempDir::new().unwrap();
        let source = one_step_source(
            dir.path(),
            json!({
                "name": "alpha",
                "shell": "primary-command",
                "teardown": [{
                    "action": { "action": "shell", "command": "teardown-command", "no_error": true },
                }],
            }),
        );
        let fixture = Fixture::build(dir, &source).unwrap();
        let recorder = Recorder::default();
        recorder.fail_stack_shell();
        let shell = FakeTaskShell::with_stdout(&["primary out"]);

        let outcome = fixture.execute(&Wiring::new(&recorder, &shell));

        assert!(outcome.succeeded(), "{}", failure_message(&outcome));
    }

    #[test]
    fn an_unparsable_stack_fails_before_setup_starts_so_teardown_never_runs() {
        let dir = TempDir::new().unwrap();
        let source = one_step_source(
            dir.path(),
            json!({
                "name": "alpha",
                "shell": "primary-command",
                "setup": [{ "when": "not a || valid ==", "action": { "message": "hi" } }],
                "teardown": [{ "action": { "shell": "teardown-command" } }],
            }),
        );
        let fixture = Fixture::build(dir, &source).unwrap();
        let recorder = Recorder::default();
        let shell = FakeTaskShell::with_stdout(&["primary out"]);

        let outcome = fixture.execute(&Wiring::new(&recorder, &shell));

        assert_eq!(outcome.status, TaskStatus::Failed);
        assert!(
            recorder.events().is_empty() && shell.commands().is_empty(),
            "nothing may run when the task's stacks do not parse: {:?}",
            recorder.events(),
        );
    }

    #[test]
    fn a_stack_that_is_not_a_list_is_rejected() {
        let dir = TempDir::new().unwrap();
        let source = one_step_source(
            dir.path(),
            json!({
                "name": "alpha",
                "shell": "primary-command",
                "setup": { "action": { "message": "hi" } },
            }),
        );
        let fixture = Fixture::build(dir, &source).unwrap();
        let recorder = Recorder::default();
        let shell = FakeTaskShell::default();

        let outcome = fixture.execute(&Wiring::new(&recorder, &shell));

        assert_eq!(outcome.status, TaskStatus::Failed);
        assert!(
            failure_message(&outcome).contains("must be a list of action-stack items"),
            "{}",
            failure_message(&outcome),
        );
    }
}

// -- shell tasks ------------------------------------------------------------

mod shell_tasks {
    use super::*;

    #[test]
    fn commands_run_in_declaration_order_and_their_stdout_concatenates() {
        let dir = TempDir::new().unwrap();
        let source = one_step_source(
            dir.path(),
            json!({ "name": "alpha", "shell": ["first", "second", "third"] }),
        );
        let fixture = Fixture::build(dir, &source).unwrap();
        let recorder = Recorder::default();
        let shell = FakeTaskShell::with_stdout(&["one", "two", "three"]);
        let runtime = RuntimeState::new();
        let mut wiring = Wiring::new(&recorder, &shell);
        wiring.runtime = Some(&runtime);

        let outcome = fixture.execute(&wiring);

        assert!(outcome.succeeded(), "{}", failure_message(&outcome));
        assert_eq!(shell.commands(), vec!["first", "second", "third"]);
        assert_eq!(outcome.stdout, "one\ntwo\nthree");
        assert_eq!(
            runtime.last_output_text().as_deref(),
            Some("one\ntwo\nthree"),
            "the concatenation is what lands in `outputs`",
        );
    }

    /// Approved == executed: preflight resolved `{{ state.name }}` once, and the
    /// executor runs those bytes without re-resolving anything.
    #[test]
    fn the_preflight_approved_bytes_are_executed_verbatim() {
        let dir = TempDir::new().unwrap();
        let source = one_step_source(
            dir.path(),
            json!({ "name": "alpha", "shell": "echo {{ state.name }}" }),
        );
        let fixture = Fixture::build(dir, &source).unwrap();
        assert!(
            matches!(&fixture.task().action, PreflightAction::Shell { commands }
                if commands == &["echo alpha".to_string()]),
            "preflight resolves the command once: {:?}",
            fixture.task().action,
        );
        let recorder = Recorder::default();
        let shell = FakeTaskShell::with_stdout(&["alpha"]);

        fixture.execute(&Wiring::new(&recorder, &shell));

        assert_eq!(shell.commands(), vec!["echo alpha".to_string()]);
    }

    /// The inverse guard: whatever bytes preflight stored are the bytes that
    /// run, proving the executor holds no interpolation step at all.
    #[test]
    fn approved_bytes_are_never_re_interpolated() {
        let dir = TempDir::new().unwrap();
        let source = one_step_source(
            dir.path(),
            json!({ "name": "alpha", "shell": "echo literal-only" }),
        );
        let fixture = Fixture::build(dir, &source).unwrap();
        let recorder = Recorder::default();
        let shell = FakeTaskShell::with_stdout(&["literal"]);

        let approved = match &fixture.task().action {
            PreflightAction::Shell { commands } => commands.clone(),
            other => panic!("expected a shell task, got {other:?}"),
        };
        fixture.execute(&Wiring::new(&recorder, &shell));

        assert_eq!(shell.commands(), approved);
    }

    #[test]
    fn the_default_per_command_budget_is_thirty_seconds() {
        let dir = TempDir::new().unwrap();
        let source = one_step_source(dir.path(), json!({ "name": "alpha", "shell": "work" }));
        let fixture = Fixture::build(dir, &source).unwrap();
        let recorder = Recorder::default();
        let shell = FakeTaskShell::with_stdout(&["done"]);

        fixture.execute(&Wiring::new(&recorder, &shell));

        assert_eq!(shell.timeouts(), vec![Duration::from_secs(30)]);
        assert_eq!(DEFAULT_COMMAND_TIMEOUT, Duration::from_secs(30));
    }

    #[test]
    fn a_typed_duration_overrides_the_default_for_every_command() {
        let dir = TempDir::new().unwrap();
        let source = one_step_source(
            dir.path(),
            json!({ "name": "alpha", "shell": ["a", "b"], "timeout": "2 min" }),
        );
        let fixture = Fixture::build(dir, &source).unwrap();
        let recorder = Recorder::default();
        let shell = FakeTaskShell::with_stdout(&["a", "b"]);

        fixture.execute(&Wiring::new(&recorder, &shell));

        assert_eq!(
            shell.timeouts(),
            vec![Duration::from_secs(120), Duration::from_secs(120)],
        );
    }

    #[test]
    fn a_zero_duration_timeout_is_rejected_rather_than_read_as_unbounded() {
        let dir = TempDir::new().unwrap();
        let source = one_step_source(
            dir.path(),
            json!({ "name": "alpha", "shell": "work", "timeout": "0s" }),
        );
        let fixture = Fixture::build(dir, &source).unwrap();
        let recorder = Recorder::default();
        let shell = FakeTaskShell::default();

        let outcome = fixture.execute(&Wiring::new(&recorder, &shell));

        assert_eq!(outcome.status, TaskStatus::Failed);
        assert!(
            failure_message(&outcome).contains("greater than zero"),
            "{}",
            failure_message(&outcome),
        );
        assert!(shell.commands().is_empty());
    }

    #[test]
    fn a_bare_integer_timeout_is_rejected() {
        let dir = TempDir::new().unwrap();
        let source = one_step_source(
            dir.path(),
            json!({ "name": "alpha", "shell": "work", "timeout": 30 }),
        );
        let fixture = Fixture::build(dir, &source).unwrap();
        let recorder = Recorder::default();
        let shell = FakeTaskShell::default();

        let outcome = fixture.execute(&Wiring::new(&recorder, &shell));

        assert_eq!(outcome.status, TaskStatus::Failed);
        assert!(
            failure_message(&outcome).contains("missing unit"),
            "{}",
            failure_message(&outcome),
        );
    }

    #[test]
    fn a_timed_out_command_fails_the_task_and_stops_the_rest() {
        let dir = TempDir::new().unwrap();
        let source = one_step_source(
            dir.path(),
            json!({ "name": "alpha", "shell": ["slow", "never"], "timeout": "5s" }),
        );
        let fixture = Fixture::build(dir, &source).unwrap();
        let recorder = Recorder::default();
        let shell = FakeTaskShell::with_results(vec![ShellCommandOutput {
            stdout: "partial\n".to_string(),
            exit_code: 143,
            timed_out: true,
        }]);

        let outcome = fixture.execute(&Wiring::new(&recorder, &shell));

        assert_eq!(outcome.status, TaskStatus::Failed);
        assert!(
            failure_message(&outcome).contains("timed out after 5s"),
            "{}",
            failure_message(&outcome),
        );
        assert_eq!(shell.commands(), vec!["slow".to_string()]);
    }

    #[test]
    fn a_nonzero_exit_stops_the_remaining_commands() {
        let dir = TempDir::new().unwrap();
        let source = one_step_source(
            dir.path(),
            json!({ "name": "alpha", "shell": ["fails", "never"] }),
        );
        let fixture = Fixture::build(dir, &source).unwrap();
        let recorder = Recorder::default();
        let shell = FakeTaskShell::with_results(vec![ShellCommandOutput {
            stdout: String::new(),
            exit_code: 2,
            timed_out: false,
        }]);

        let outcome = fixture.execute(&Wiring::new(&recorder, &shell));

        assert_eq!(outcome.status, TaskStatus::Failed);
        assert_eq!(shell.commands(), vec!["fails".to_string()]);
    }

    #[test]
    fn a_command_that_cannot_be_spawned_is_a_typed_failure() {
        let dir = TempDir::new().unwrap();
        let source = one_step_source(dir.path(), json!({ "name": "alpha", "shell": "work" }));
        let fixture = Fixture::build(dir, &source).unwrap();
        let recorder = Recorder::default();
        let shell = FakeTaskShell::failing_to_spawn();

        let outcome = fixture.execute(&Wiring::new(&recorder, &shell));

        assert_eq!(outcome.status, TaskStatus::Failed);
        assert!(
            failure_message(&outcome).contains("failed to run"),
            "{}",
            failure_message(&outcome),
        );
    }

    /// The production runner, on the platform actually running the suite:
    /// `echo` exists on macOS, Linux, and Windows `cmd`.
    #[test]
    fn the_system_shell_captures_stdout_on_this_platform() {
        let output = SystemTaskShell
            .run("echo task-shell-ok", Duration::from_secs(30))
            .unwrap();
        assert_eq!(output.exit_code, 0);
        assert!(!output.timed_out);
        assert_eq!(output.stdout.trim_end(), "task-shell-ok");
    }

    /// Killing an overrunning child is the one shell behavior with no portable
    /// command to exercise it, so the sleep is gated rather than the assertion.
    #[cfg(unix)]
    #[test]
    fn the_system_shell_kills_a_command_that_overruns_its_budget() {
        let output = SystemTaskShell
            .run("sleep 5", Duration::from_millis(150))
            .unwrap();
        assert!(output.timed_out, "the child must be killed, not awaited");
    }
}

// -- side-effect tasks ------------------------------------------------------

mod side_effect_tasks {
    use super::*;

    #[test]
    fn a_side_effect_task_captures_its_textual_return() {
        let dir = TempDir::new().unwrap();
        let source = one_step_source(
            dir.path(),
            json!({ "name": "alpha", "side_effect": { "ensure_dir": ["made-here"] } }),
        );
        let fixture = Fixture::build(dir, &source).unwrap();
        let recorder = Recorder::default();
        let shell = FakeTaskShell::default();
        let runtime = RuntimeState::new();
        let mut wiring = Wiring::new(&recorder, &shell);
        wiring.runtime = Some(&runtime);

        let outcome = fixture.execute(&wiring);

        assert!(outcome.succeeded(), "{}", failure_message(&outcome));
        assert!(
            outcome.stdout.ends_with("made-here"),
            "the effect's returned path is the task's output: {:?}",
            outcome.stdout,
        );
        assert_eq!(runtime.output_count(), 1);
    }

    /// `set` returns the value it replaced — nothing, on a first write — so the
    /// task still contributes exactly one (empty) entry.
    #[test]
    fn a_side_effect_with_no_textual_return_appends_the_empty_string() {
        let dir = TempDir::new().unwrap();
        let source = one_step_source(
            dir.path(),
            json!({ "name": "alpha", "side_effect": { "set": ["ready", "{{ true }}"] } }),
        );
        let fixture = Fixture::build(dir, &source).unwrap();
        let recorder = Recorder::default();
        let shell = FakeTaskShell::default();
        let runtime = RuntimeState::new();
        let mut wiring = Wiring::new(&recorder, &shell);
        wiring.runtime = Some(&runtime);

        let outcome = fixture.execute(&wiring);

        assert!(outcome.succeeded(), "{}", failure_message(&outcome));
        assert_eq!(outcome.stdout, "");
        assert_eq!(runtime.last_output_text().as_deref(), Some(""));
        assert_eq!(
            runtime.snapshot().mutations.get("ready"),
            Some(&Value::Bool(true)),
            "whole-value typing survives the action grammar",
        );
    }

    #[test]
    fn the_mutation_delta_reports_the_keys_the_task_wrote() {
        let dir = TempDir::new().unwrap();
        let source = one_step_source(
            dir.path(),
            json!({
                "name": "alpha",
                "side_effect": { "set": ["stage", "primary"] },
                "setup": [{ "action": { "set": ["stage", "setup"] } }],
            }),
        );
        let fixture = Fixture::build(dir, &source).unwrap();
        let recorder = Recorder::default();
        let shell = FakeTaskShell::default();
        let runtime = RuntimeState::new();
        let mut wiring = Wiring::new(&recorder, &shell);
        wiring.runtime = Some(&runtime);

        let outcome = fixture.execute(&wiring);

        assert!(outcome.succeeded(), "{}", failure_message(&outcome));
        assert_eq!(outcome.mutations.len(), 1, "{:?}", outcome.mutations);
        assert_eq!(outcome.mutations[0].key, "stage");
        assert_eq!(outcome.mutations[0].value, json!("primary"));
        assert_eq!(
            outcome.mutations[0].prior,
            Value::Null,
            "the delta is measured against the layer as it stood before the task",
        );
        assert_eq!(
            outcome.stdout, "setup",
            "the primary `set` returns what the setup stage wrote",
        );
    }

    #[test]
    fn a_non_side_effect_action_is_rejected() {
        let dir = TempDir::new().unwrap();
        let source = one_step_source(
            dir.path(),
            json!({ "name": "alpha", "side_effect": { "message": "not work" } }),
        );
        let fixture = Fixture::build(dir, &source).unwrap();
        let recorder = Recorder::default();
        let shell = FakeTaskShell::default();

        let outcome = fixture.execute(&Wiring::new(&recorder, &shell));

        assert_eq!(outcome.status, TaskStatus::Failed);
        assert!(
            failure_message(&outcome).contains("is a communication action"),
            "{}",
            failure_message(&outcome),
        );
        assert!(
            recorder.events().is_empty(),
            "a rejected side effect must not emit: {:?}",
            recorder.events(),
        );
    }

    #[test]
    fn a_side_effect_targeting_a_reserved_key_fails_the_task() {
        let dir = TempDir::new().unwrap();
        let source = one_step_source(
            dir.path(),
            json!({ "name": "alpha", "side_effect": { "set": ["outputs", "hijacked"] } }),
        );
        let fixture = Fixture::build(dir, &source).unwrap();
        let recorder = Recorder::default();
        let shell = FakeTaskShell::default();
        let runtime = RuntimeState::new();
        let mut wiring = Wiring::new(&recorder, &shell);
        wiring.runtime = Some(&runtime);

        let outcome = fixture.execute(&wiring);

        assert_eq!(outcome.status, TaskStatus::Failed);
        assert!(
            failure_message(&outcome).contains("reserved"),
            "{}",
            failure_message(&outcome),
        );
        assert_eq!(runtime.output_count(), 0);
    }
}

// -- prompt tasks -----------------------------------------------------------

mod prompt_tasks {
    use super::*;

    fn prompt_fixture(params: Value) -> (TempDir, String) {
        let dir = TempDir::new().unwrap();
        write_source(
            dir.path(),
            "review.md",
            &[("title", json!("Review"))],
            "Review.\n",
        );
        let source = one_step_source(
            dir.path(),
            json!({ "name": "alpha", "topic": "parsing", "prompt": "review.md", "params": params }),
        );
        (dir, source)
    }

    #[test]
    fn params_are_evaluated_just_in_time_against_the_effective_state() {
        let (dir, source) = prompt_fixture(json!({
            "topic": "{{ state.name }}",
            "position": "{{ state.index }}",
            "literal": "unchanged",
        }));
        let fixture = Fixture::build(dir, &source).unwrap();
        let recorder = Recorder::default();
        let shell = FakeTaskShell::default();
        let prompt = FakePrompt::succeeding("final text");
        let mut wiring = Wiring::new(&recorder, &shell);
        wiring.prompt = &prompt;

        let outcome = fixture.execute(&wiring);

        assert!(outcome.succeeded(), "{}", failure_message(&outcome));
        let request = prompt.last();
        assert_eq!(request.params.get("topic"), Some(&json!("alpha")));
        assert_eq!(
            request.params.get("position"),
            Some(&json!(1)),
            "a whole-value span keeps its type instead of rendering to text",
        );
        assert_eq!(request.params.get("literal"), Some(&json!("unchanged")));
        assert_eq!(outcome.stdout, "final text");
    }

    #[test]
    fn layered_overrides_run_params_then_setters_then_mutations_then_overlay() {
        let (dir, source) = prompt_fixture(json!({
            "only_param": "from-params",
            "shared": "from-params",
            "mutated": "from-params",
        }));
        let fixture = Fixture::build(dir, &source).unwrap();
        let recorder = Recorder::default();
        let shell = FakeTaskShell::default();
        let prompt = FakePrompt::succeeding("done");
        let runtime = RuntimeState::new();
        let engine = EffectEngine::builder().auto_rehash(false).build();
        runtime
            .set(&engine, "mutated", json!("from-mutation"), &Map::new())
            .unwrap();
        let setters = json!({ "shared": "from-setters", "mutated": "from-setters" });
        let mut wiring = Wiring::new(&recorder, &shell);
        wiring.prompt = &prompt;
        wiring.runtime = Some(&runtime);
        wiring.user_setters = Some(&setters);

        fixture.execute(&wiring);

        let overrides = prompt.last().set_overrides;
        assert_eq!(overrides["only_param"], json!("from-params"));
        assert_eq!(
            overrides["shared"],
            json!("from-setters"),
            "sequence user setters outrank task params",
        );
        assert_eq!(
            overrides["mutated"],
            json!("from-mutation"),
            "accumulated runtime mutations outrank user setters",
        );
        assert_eq!(
            overrides["state"]["name"],
            json!("alpha"),
            "the reserved overlay outranks everything",
        );
        assert_eq!(overrides["outputs"], json!([]));
    }

    #[test]
    fn a_param_targeting_a_reserved_key_is_rejected_before_the_provider_runs() {
        let (dir, source) = prompt_fixture(json!({ "state": "hijacked" }));
        let fixture = Fixture::build(dir, &source).unwrap();
        let recorder = Recorder::default();
        let shell = FakeTaskShell::default();
        let prompt = FakePrompt::succeeding("never");
        let mut wiring = Wiring::new(&recorder, &shell);
        wiring.prompt = &prompt;

        let outcome = fixture.execute(&wiring);

        assert_eq!(outcome.status, TaskStatus::Failed);
        assert!(
            failure_message(&outcome).contains("reserved key"),
            "{}",
            failure_message(&outcome),
        );
        assert!(prompt.requests.lock().unwrap().is_empty());
    }

    /// The referenced document's own frontmatter decides the mode; the
    /// referencing step never states it.
    #[test]
    fn inline_compose_mode_comes_from_the_referenced_document() {
        for (name, frontmatter, expected) in [
            ("plain.md", vec![("title", json!("Plain"))], false),
            ("inline.md", vec![("prompt", json!("Write the body."))], true),
        ] {
            let dir = TempDir::new().unwrap();
            write_source(dir.path(), name, &frontmatter, "Body.\n");
            let source = one_step_source(dir.path(), json!({ "name": "alpha", "prompt": name }));
            let fixture = Fixture::build(dir, &source).unwrap();
            let recorder = Recorder::default();
            let shell = FakeTaskShell::default();
            let prompt = FakePrompt::succeeding("out");
            let mut wiring = Wiring::new(&recorder, &shell);
            wiring.prompt = &prompt;

            fixture.execute(&wiring);

            assert_eq!(
                prompt.last().inline_compose,
                expected,
                "{name} should compose with inline_compose={expected}",
            );
        }
    }

    #[test]
    fn a_nonzero_provider_exit_fails_the_task_and_names_the_code() {
        let (dir, source) = prompt_fixture(json!({}));
        let fixture = Fixture::build(dir, &source).unwrap();
        let recorder = Recorder::default();
        let shell = FakeTaskShell::default();
        let prompt = FakePrompt::with(PromptRunOutcome {
            stdout: "partial".to_string(),
            exit_code: 4,
            interrupted: false,
        });
        let runtime = RuntimeState::new();
        let mut wiring = Wiring::new(&recorder, &shell);
        wiring.prompt = &prompt;
        wiring.runtime = Some(&runtime);

        let outcome = fixture.execute(&wiring);

        assert_eq!(outcome.status, TaskStatus::Failed);
        assert!(
            failure_message(&outcome).contains("exited with code 4"),
            "{}",
            failure_message(&outcome),
        );
        assert_eq!(runtime.output_count(), 0, "a failed task appends nothing");
    }

    #[test]
    fn an_interrupted_provider_reports_interruption_rather_than_failure() {
        let (dir, source) = prompt_fixture(json!({}));
        let fixture = Fixture::build(dir, &source).unwrap();
        let recorder = Recorder::default();
        let shell = FakeTaskShell::default();
        let prompt = FakePrompt::with(PromptRunOutcome {
            stdout: "partial".to_string(),
            exit_code: 130,
            interrupted: true,
        });
        let runtime = RuntimeState::new();
        let mut wiring = Wiring::new(&recorder, &shell);
        wiring.prompt = &prompt;
        wiring.runtime = Some(&runtime);

        let outcome = fixture.execute(&wiring);

        assert_eq!(outcome.status, TaskStatus::Interrupted);
        assert!(outcome.error.is_none());
        assert_eq!(runtime.output_count(), 0);
    }

    /// A reference authored inside a task file resolves from *that* file's
    /// directory, and the resolved path travels to the runner untouched.
    #[test]
    fn a_prompt_reference_resolves_from_its_authoring_directory() {
        let dir = TempDir::new().unwrap();
        let nested = dir.path().join("nested");
        fs::create_dir_all(&nested).unwrap();
        write_source(&nested, "review.md", &[("title", json!("Nested"))], "Deep.\n");
        write_source(
            dir.path(),
            "review.md",
            &[("title", json!("Root"))],
            "Shallow.\n",
        );
        write_yaml(
            &nested,
            "task.yaml",
            &json!({ "kind": "task", "prompt": "review.md" }),
        );
        let source = one_step_source(
            dir.path(),
            json!({ "name": "alpha", "task": "nested/task.yaml" }),
        );
        let fixture = Fixture::build(dir, &source).unwrap();
        let recorder = Recorder::default();
        let shell = FakeTaskShell::default();
        let prompt = FakePrompt::succeeding("out");
        let mut wiring = Wiring::new(&recorder, &shell);
        wiring.prompt = &prompt;

        fixture.execute(&wiring);

        let path = prompt.last().path;
        assert!(
            path.parent().unwrap().ends_with("nested"),
            "the task file's own directory anchors its reference, got {}",
            path.display(),
        );
    }
}

// -- outputs, interruption, and equivalence ---------------------------------

mod outcome_contract {
    use super::*;

    #[test]
    fn a_successful_task_with_no_output_still_appends_one_entry() {
        let dir = TempDir::new().unwrap();
        let source = one_step_source(dir.path(), json!({ "name": "alpha", "shell": "quiet" }));
        let fixture = Fixture::build(dir, &source).unwrap();
        let recorder = Recorder::default();
        let shell = FakeTaskShell::with_results(vec![ShellCommandOutput {
            stdout: String::new(),
            exit_code: 0,
            timed_out: false,
        }]);
        let runtime = RuntimeState::new();
        let mut wiring = Wiring::new(&recorder, &shell);
        wiring.runtime = Some(&runtime);

        let outcome = fixture.execute(&wiring);

        assert!(outcome.succeeded(), "{}", failure_message(&outcome));
        assert!(outcome.output_committed);
        assert_eq!(
            runtime.output_count(),
            1,
            "entries stay aligned with executed work",
        );
        assert_eq!(runtime.last_output_text().as_deref(), Some(""));
    }

    #[test]
    fn the_captured_entry_preserves_interior_whitespace() {
        let dir = TempDir::new().unwrap();
        let source = one_step_source(dir.path(), json!({ "name": "alpha", "shell": "spaced" }));
        let fixture = Fixture::build(dir, &source).unwrap();
        let recorder = Recorder::default();
        let shell = FakeTaskShell::with_results(vec![ShellCommandOutput {
            stdout: "  leading and\n\ntrailing blank line\n\n".to_string(),
            exit_code: 0,
            timed_out: false,
        }]);
        let runtime = RuntimeState::new();
        let mut wiring = Wiring::new(&recorder, &shell);
        wiring.runtime = Some(&runtime);

        fixture.execute(&wiring);

        assert_eq!(
            runtime.last_output_text().as_deref(),
            Some("  leading and\n\ntrailing blank line\n"),
            "exactly one transport newline is removed",
        );
    }

    #[test]
    fn an_interrupt_before_the_task_starts_runs_nothing_at_all() {
        let dir = TempDir::new().unwrap();
        let source = one_step_source(
            dir.path(),
            json!({
                "name": "alpha",
                "shell": "primary-command",
                "setup": [{ "action": { "shell": "setup-command" } }],
                "teardown": [{ "action": { "shell": "teardown-command" } }],
            }),
        );
        let fixture = Fixture::build(dir, &source).unwrap();
        let recorder = Recorder::default();
        let shell = FakeTaskShell::default();
        let interrupt = AtomicBool::new(true);
        let mut wiring = Wiring::new(&recorder, &shell);
        wiring.interrupt = Some(&interrupt);

        let outcome = fixture.execute(&wiring);

        assert_eq!(outcome.status, TaskStatus::Interrupted);
        assert!(
            recorder.events().is_empty() && shell.commands().is_empty(),
            "no teardown is owed for a task that never started: {:?}",
            recorder.events(),
        );
    }

    /// The same task, authored inline in a step and externalized to a
    /// `kind: task` file, must produce the same outcome.
    #[test]
    fn a_task_behaves_identically_inline_and_as_an_external_file() {
        let task_body = json!({
            "shell": ["first", "second"],
            "timeout": "45s",
            "teardown": [{ "action": { "message": "cleaned" } }],
        });

        let inline_dir = TempDir::new().unwrap();
        let mut inline_step = task_body.as_object().unwrap().clone();
        inline_step.insert("name".to_string(), json!("alpha"));
        let inline_source = one_step_source(inline_dir.path(), Value::Object(inline_step));

        let external_dir = TempDir::new().unwrap();
        let mut external = task_body.as_object().unwrap().clone();
        external.insert("kind".to_string(), json!("task"));
        write_yaml(external_dir.path(), "task.yaml", &Value::Object(external));
        let external_source = one_step_source(
            external_dir.path(),
            json!({ "name": "alpha", "task": "task.yaml" }),
        );

        let mut results = Vec::new();
        for (dir, source) in [(inline_dir, inline_source), (external_dir, external_source)] {
            let fixture = Fixture::build(dir, &source).unwrap();
            let recorder = Recorder::default();
            let shell = FakeTaskShell::with_stdout(&["one", "two"]);
            let runtime = RuntimeState::new();
            let mut wiring = Wiring::new(&recorder, &shell);
            wiring.runtime = Some(&runtime);

            let outcome = fixture.execute(&wiring);
            results.push((
                outcome.status,
                outcome.stdout,
                shell.commands(),
                shell.timeouts(),
                recorder.events(),
                runtime.outputs_value(),
            ));
        }

        assert_eq!(results[0], results[1], "an externalized task is the same task");
        assert_eq!(results[0].0, TaskStatus::Succeeded);
        assert_eq!(results[0].3, vec![Duration::from_secs(45); 2]);
    }

    /// The external reference is exclusive *and* immutable: the referencing site
    /// contributes the reference and nothing else.
    #[test]
    fn a_referencing_site_override_on_an_external_task_is_rejected() {
        let dir = TempDir::new().unwrap();
        write_yaml(
            dir.path(),
            "task.yaml",
            &json!({ "kind": "task", "shell": "work", "name": "authored-name" }),
        );
        write_yaml(
            dir.path(),
            "group.yaml",
            &json!({
                "kind": "group",
                "name": "bundle",
                "tasks": [{ "task": "task.yaml", "name": "patched-name" }],
            }),
        );
        let source = one_step_source(dir.path(), json!({ "name": "alpha", "group": "group.yaml" }));

        let error = Fixture::build(dir, &source)
            .err()
            .expect("a referencing-site patch must be rejected");
        assert!(
            matches!(&error, CompositionError::SequenceReferenceInvalid { problem, .. }
                if problem.contains("meaningless for a `task` task")),
            "got: {error}",
        );
    }

    /// A group reaching the atomic executor is a scheduling mistake, not work:
    /// group execution lands with its own scheduler.
    #[test]
    fn a_group_action_is_not_executable_as_an_atomic_task() {
        let dir = TempDir::new().unwrap();
        write_yaml(
            dir.path(),
            "group.yaml",
            &json!({ "kind": "group", "name": "bundle", "tasks": [{ "shell": "work" }] }),
        );
        let source = one_step_source(dir.path(), json!({ "name": "alpha", "group": "group.yaml" }));
        let fixture = Fixture::build(dir, &source).unwrap();
        let recorder = Recorder::default();
        let shell = FakeTaskShell::default();

        let outcome = fixture.execute(&Wiring::new(&recorder, &shell));

        assert_eq!(outcome.status, TaskStatus::Failed);
        assert!(
            failure_message(&outcome).contains("is not executable yet"),
            "{}",
            failure_message(&outcome),
        );
        assert!(shell.commands().is_empty());
    }
}
