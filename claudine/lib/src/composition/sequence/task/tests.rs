//! Phase 7 — atomic task execution and lifecycle semantics.
//!
//! Every test drives the public seam ([`TaskExecution::run`]) over a task node
//! produced by the *real* preflight walk against files on disk, because the
//! contract under test starts at the node: approved shell bytes, the authoring
//! directory a reference resolved from, and the composition mode of the
//! referenced document are all decided before execution and must survive it
//! unchanged.


use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::sync::atomic::AtomicBool;

use biscuit_terminal::discovery::detection::ColorDepth;
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
use crate::render::{TaskLiveOutput, TaskStreamSink};

// -- fixtures ---------------------------------------------------------------

/// Which of a [`TaskStreamSink`]'s two channels a write arrived on.
///
/// Shared vocabulary rather than per-module, because the channel a frame lands
/// on *is* the contract under test in both the streaming and group-framing
/// suites.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Channel {
    /// Headers, footers, warnings, and a shell command's stderr — stderr in
    /// production.
    Status,
    /// Task and provider data — stdout in production.
    Data,
}

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

    /// Render against `term` instead of the detected one.
    ///
    /// `Terminal::default()` reads `NO_COLOR` / `COLORTERM` / terminfo from the
    /// host process, so any assertion about *styling* would otherwise be decided
    /// by the ambient environment the gate happens to run in.
    fn with_terminal(mut self, term: Terminal) -> Self {
        self.term = term;
        self
    }

    fn task(&self) -> &PreflightTask {
        self.graph.steps[0]
            .task
            .as_ref()
            .expect("fixture's step declares an executable")
    }

    /// The task declared by step `index`.
    fn task_at(&self, index: usize) -> &PreflightTask {
        self.graph.steps[index]
            .task
            .as_ref()
            .expect("fixture's step declares an executable")
    }

    /// Run the fixture's task with the given doubles.
    fn execute(&self, wiring: &Wiring<'_>) -> TaskOutcome {
        self.execute_step(0, wiring)
    }

    /// Run one specific step's task with the given doubles.
    fn execute_step(&self, index: usize, wiring: &Wiring<'_>) -> TaskOutcome {
        let live = std::sync::Mutex::new(self.frontmatter.clone());
        let stack = StackExecutionContext {
            signal: LifecycleSignal::Start,
            frontmatter: &self.frontmatter,
            live_frontmatter: Some(&live),
            runtime_state: wiring.runtime.map(Arc::as_ref),
            err: None,
            timing: None,
            current: None,
            group: None,
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
            task: self.task_at(index),
            graph: &self.graph,
            stack: &stack,
            state: &self.state,
            runtime: wiring.runtime,
            user_setters: wiring.user_setters,
            overlay: Some(&self.overlay),
            shell: wiring.shell,
            prompt: wiring.prompt,
            interrupt: wiring.interrupt,
            stream: wiring.stream.as_ref(),
            // Group scheduling builds a member's live stream from `stream`; a
            // lone task under test has no enclosing scheduler to give it one.
            live: None,
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
    runtime: Option<&'a Arc<RuntimeState>>,
    user_setters: Option<&'a Value>,
    interrupt: Option<&'a AtomicBool>,
    /// Where group-task header/footer frames land. `None` is the `--silent`
    /// shape: nothing is rendered at all. Owned rather than borrowed because
    /// the executor shares the sink with per-member live streams.
    stream: Option<Arc<dyn TaskStreamSink>>,
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
            stream: None,
        }
    }

    /// Route this run's group-task frames into `sink`.
    fn with_stream(mut self, sink: Arc<dyn TaskStreamSink>) -> Self {
        self.stream = Some(sink);
        self
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
    /// How long a given command blocks before reporting, so a test can decide
    /// completion order independently of declaration order.
    delays: Mutex<HashMap<String, Duration>>,
    /// Per-command stdout. A shared FIFO of results is order-dependent, which a
    /// concurrent group has no ordering guarantee for.
    by_command: Mutex<HashMap<String, String>>,
    /// Commands whose exit code is non-zero.
    failing: Mutex<HashSet<String>>,
    /// `(currently in flight, high-water mark)`. A parallel test asserts the
    /// high-water mark against `max_parallel`.
    in_flight: Mutex<(usize, usize)>,
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
                interrupted: false,
                aborted: false,
            })
            .collect();
        shell
    }
    fn with_results(results: Vec<ShellCommandOutput>) -> Self {
        let shell = Self::default();
        *shell.results.lock().unwrap() = results;
        shell
    }
    /// Give `command` its own stdout, so a concurrent run's ordering cannot
    /// decide which result a task receives.
    fn stdout_for(self, command: &str, stdout: &str) -> Self {
        self.by_command
            .lock()
            .unwrap()
            .insert(command.to_string(), stdout.to_string());
        self
    }
    /// Make `command` exit non-zero.
    fn failing(self, command: &str) -> Self {
        self.failing.lock().unwrap().insert(command.to_string());
        self
    }
    /// The most commands that were ever running at the same time.
    fn peak_in_flight(&self) -> usize {
        self.in_flight.lock().unwrap().1
    }
    /// Make `command` block for `millis` before it reports.
    fn delay(self, command: &str, millis: u64) -> Self {
        self.delays
            .lock()
            .unwrap()
            .insert(command.to_string(), Duration::from_millis(millis));
        self
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
    fn run(
        &self,
        command: &str,
        timeout: Duration,
        interrupt: Option<&AtomicBool>,
        live: Option<&Arc<TaskLiveOutput>>,
    ) -> Result<ShellCommandOutput, std::io::Error> {
        // Streaming is the runner's job now, so the fake owes it too: a fake
        // that only returned bytes would leave every group-framing test
        // asserting against a body line the production path emits and this one
        // does not.
        let stream = |output: ShellCommandOutput| {
            if let Some(live) = live {
                live.append(&output.stdout);
                live.flush();
            }
            output
        };
        {
            let mut gauge = self.in_flight.lock().unwrap();
            gauge.0 += 1;
            gauge.1 = gauge.1.max(gauge.0);
        }
        // Per-command delays let a parallel test invert completion order
        // without a real process: the *last*-declared task can finish first.
        if let Some(delay) = self.delays.lock().unwrap().get(command).copied() {
            std::thread::sleep(delay);
        }
        self.in_flight.lock().unwrap().0 -= 1;
        if interrupt.is_some_and(|flag| flag.load(std::sync::atomic::Ordering::SeqCst)) {
            return Ok(stream(ShellCommandOutput {
                stdout: String::new(),
                exit_code: -1,
                timed_out: false,
                interrupted: true,
                aborted: false,
            }));
        }
        self.seen
            .lock()
            .unwrap()
            .push((command.to_string(), timeout));
        if let Some(stdout) = self.by_command.lock().unwrap().get(command).cloned() {
            let failing = self.failing.lock().unwrap().contains(command);
            return Ok(stream(ShellCommandOutput {
                stdout,
                exit_code: i32::from(failing),
                timed_out: false,
                interrupted: false,
                aborted: false,
            }));
        }
        if *self.spawn_failure.lock().unwrap() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "no such file or directory",
            ));
        }
        let mut results = self.results.lock().unwrap();
        if results.is_empty() {
            return Ok(stream(ShellCommandOutput {
                stdout: String::new(),
                exit_code: 0,
                timed_out: false,
                interrupted: false,
                aborted: false,
            }));
        }
        Ok(stream(results.remove(0)))
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
            interrupted: false,
            aborted: false,
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
        let runtime = Arc::new(RuntimeState::new());
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
            interrupted: false,
            aborted: false,
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
        let runtime = Arc::new(RuntimeState::new());
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
            interrupted: false,
            aborted: false,
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
            interrupted: false,
            aborted: false,
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
        let output = SystemTaskShell::default()
            .run("echo task-shell-ok", Duration::from_secs(30), None, None)
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
        let output = SystemTaskShell::default()
            .run("sleep 5", Duration::from_millis(150), None, None)
            .unwrap();
        assert!(output.timed_out, "the child must be killed, not awaited");
    }

    /// A very generous non-hang bound. The fixtures below outlive it several
    /// times over, so crossing it means the call never returned rather than
    /// that this host was slow — which it routinely is.
    const NON_HANG_BOUND: Duration = Duration::from_secs(45);

    /// The core regression for review 5 finding 1.
    ///
    /// `echo early; (sleep 300; echo late) &` leaves a *descendant* holding the
    /// inherited stdout write end. Killing only the direct shell leaves that
    /// pipe open, so the pre-fix runner reported a kill and then blocked in
    /// `reader.join()` for the descendant's full lifetime. Owning the process
    /// group means the descendant dies with it.
    #[cfg(unix)]
    #[test]
    fn the_system_shell_kills_a_backgrounded_descendant_holding_stdout() {
        let start = std::time::Instant::now();
        let output = SystemTaskShell::default()
            .run(
                "(sleep 300; echo late) & echo early; sleep 300",
                Duration::from_secs(2),
                None,
                None,
            )
            .unwrap();

        assert!(start.elapsed() < NON_HANG_BOUND, "the call must not hang");
        assert!(output.timed_out, "the tree must be killed, not awaited");
        assert!(
            output.stdout.contains("early"),
            "output written before the kill survives it: {:?}",
            output.stdout,
        );
        assert!(
            !output.stdout.contains("late"),
            "the descendant must not outlive the deadline: {:?}",
            output.stdout,
        );
    }

    /// The same contract in `cmd` terms. Compile-checked under
    /// `just check-windows`; not executed on a Windows host.
    #[cfg(windows)]
    #[test]
    fn the_system_shell_kills_a_backgrounded_descendant_holding_stdout() {
        let start = std::time::Instant::now();
        let output = SystemTaskShell::default()
            .run(
                "start /b cmd /c \"timeout /t 300 >nul & echo late\" & echo early & timeout /t 300 >nul",
                Duration::from_secs(2),
                None,
                None,
            )
            .unwrap();

        assert!(start.elapsed() < NON_HANG_BOUND, "the call must not hang");
        assert!(output.timed_out, "the tree must be killed, not awaited");
        assert!(
            !output.stdout.contains("late"),
            "the descendant must not outlive the deadline: {:?}",
            output.stdout,
        );
    }

    /// A pipeline is two processes plus the shell: capture must come from the
    /// tail of the pipeline, not the shell's own (empty) stdout.
    #[cfg(unix)]
    #[test]
    fn the_system_shell_captures_a_pipeline() {
        let output = SystemTaskShell::default()
            .run("echo hello | tr a-z A-Z", Duration::from_secs(30), None, None)
            .unwrap();

        assert_eq!(output.exit_code, 0);
        assert_eq!(output.stdout.trim_end(), "HELLO");
        assert!(!output.timed_out);
        assert!(!output.aborted);
        assert!(!output.interrupted);
    }

    /// Compile-checked under `just check-windows`; not executed on a Windows
    /// host.
    #[cfg(windows)]
    #[test]
    fn the_system_shell_captures_a_pipeline() {
        let output = SystemTaskShell::default()
            .run("echo hello | findstr hello", Duration::from_secs(30), None, None)
            .unwrap();

        assert_eq!(output.exit_code, 0);
        assert_eq!(output.stdout.trim_end(), "hello");
        assert!(!output.timed_out);
        assert!(!output.aborted);
        assert!(!output.interrupted);
    }

    /// The deadline holds against a *nested* shell, not just a direct child.
    #[cfg(unix)]
    #[test]
    fn the_system_shell_times_out_a_nested_tree() {
        let start = std::time::Instant::now();
        let output = SystemTaskShell::default()
            .run("sh -c 'sh -c \"sleep 300\"'", Duration::from_secs(2), None, None)
            .unwrap();

        assert!(start.elapsed() < NON_HANG_BOUND, "the call must not hang");
        assert!(output.timed_out);
        assert!(!output.aborted);
    }

    /// Compile-checked under `just check-windows`; not executed on a Windows
    /// host.
    #[cfg(windows)]
    #[test]
    fn the_system_shell_times_out_a_nested_tree() {
        let start = std::time::Instant::now();
        let output = SystemTaskShell::default()
            .run(
                "cmd /c \"cmd /c timeout /t 300 >nul\"",
                Duration::from_secs(2),
                None,
                None,
            )
            .unwrap();

        assert!(start.elapsed() < NON_HANG_BOUND, "the call must not hang");
        assert!(output.timed_out);
        assert!(!output.aborted);
    }

    /// Ctrl+C reaching a *running* tree. The budget is far longer than the
    /// fixture's life, so `interrupted` cannot be a disguised timeout.
    #[cfg(unix)]
    #[test]
    fn the_system_shell_interrupts_a_running_tree() {
        let flag = Arc::new(AtomicBool::new(false));
        let setter = Arc::clone(&flag);
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(300));
            setter.store(true, std::sync::atomic::Ordering::SeqCst);
        });

        let start = std::time::Instant::now();
        let output = SystemTaskShell::default()
            .run(
                "(sleep 300) & sleep 300",
                Duration::from_secs(300),
                Some(&flag),
                None,
            )
            .unwrap();

        assert!(start.elapsed() < NON_HANG_BOUND, "the call must not hang");
        assert!(output.interrupted, "the interrupt flag must end the tree");
        assert!(
            !output.timed_out,
            "the budget outlives the fixture, so this cannot be a timeout",
        );
    }

    /// A command that floods stdout is stopped by the volume cap rather than
    /// growing this process's memory without bound. The cap is injected tiny so
    /// the trip happens on the first chunk.
    #[cfg(unix)]
    #[test]
    fn the_system_shell_aborts_a_command_that_floods_stdout() {
        let shell = SystemTaskShell::with_volume_cap(crate::runaway::CaptureVolumeCap::new(
            true, 16, 1024,
        ));

        let start = std::time::Instant::now();
        let output = shell
            .run("yes claudine-runaway", Duration::from_secs(300), None, None)
            .unwrap();

        assert!(start.elapsed() < NON_HANG_BOUND, "the call must not hang");
        assert!(output.aborted, "the volume cap must stop the command");
        assert!(
            !output.timed_out,
            "the budget outlives the fixture, so this cannot be a timeout",
        );
        // One 8 KiB read past a 1 KiB cap is the expected shape; the bound is
        // loose because the assertion under test is "bounded", not "exact".
        assert!(
            output.stdout.len() < 512 * 1024,
            "capture must stay bounded, saw {} bytes",
            output.stdout.len(),
        );
    }
}

// -- live shell streaming ---------------------------------------------------

/// Review 5 finding 2: a shell task's output must be *live*, on the right
/// channel, in arrival order, and never torn by a concurrent sibling.
///
/// These drive [`SystemTaskShell`] with real processes rather than
/// [`FakeTaskShell`], because every claim here is about when bytes leave a pipe
/// — which a fake decides by construction and therefore cannot falsify.
mod shell_streaming {
    use std::time::Instant;

    use super::*;
    use crate::render::{TaskBar, TaskStream};

    /// One frame group, stamped with when the sink received it.
    #[derive(Debug, Clone)]
    struct Recorded {
        channel: Channel,
        lines: Vec<String>,
        at: Duration,
    }

    /// A [`TaskStreamSink`] that timestamps every write against a fixed origin.
    ///
    /// The timestamp is the whole point: "streamed" and "buffered until the
    /// command returned" produce identical *content*, and differ only in when
    /// the sink saw it.
    struct TimedSink {
        origin: Instant,
        writes: Mutex<Vec<Recorded>>,
    }

    impl TimedSink {
        fn new() -> Arc<Self> {
            Arc::new(Self {
                origin: Instant::now(),
                writes: Mutex::new(Vec::new()),
            })
        }

        fn record(&self, channel: Channel, frames: &[String]) {
            self.writes.lock().unwrap().push(Recorded {
                channel,
                lines: frames.iter().map(|line| strip_ansi(line)).collect(),
                at: self.origin.elapsed(),
            });
        }

        fn writes(&self) -> Vec<Recorded> {
            self.writes.lock().unwrap().clone()
        }

        /// Every recorded line on one channel, in arrival order, gutter removed.
        ///
        /// The gutter has to go for the tearing assertion to be exact: what is
        /// under test is that a frame's *payload* is one whole line, and the
        /// bar in front of it is decoration this comparison must not see.
        fn payloads(&self, channel: Channel) -> Vec<String> {
            self.writes()
                .into_iter()
                .filter(|write| write.channel == channel)
                .flat_map(|write| write.lines)
                .map(|line| {
                    line.split_once('│')
                        .map_or(line.clone(), |(_, rest)| rest.to_string())
                        .trim()
                        .to_string()
                })
                .filter(|line| !line.is_empty())
                .collect()
        }

        /// When the sink first saw `needle` on `channel`.
        fn first_seen(&self, channel: Channel, needle: &str) -> Option<Duration> {
            self.writes()
                .into_iter()
                .find(|write| {
                    write.channel == channel && write.lines.iter().any(|l| l.contains(needle))
                })
                .map(|write| write.at)
        }
    }

    impl TaskStreamSink for TimedSink {
        fn write_frames(&self, frames: &[String]) {
            self.record(Channel::Status, frames);
        }

        fn write_data_frames(&self, frames: &[String]) {
            self.record(Channel::Data, frames);
        }
    }

    /// A live stream for `label` bound to `sink`.
    fn live(label: &str, bar: TaskBar, sink: &Arc<TimedSink>) -> Arc<TaskLiveOutput> {
        Arc::new(TaskLiveOutput::new(
            TaskStream::new(label, bar, Terminal::default()),
            Arc::clone(sink) as Arc<dyn TaskStreamSink>,
        ))
    }

    /// A command emitting one stdout line and one stderr line.
    #[cfg(unix)]
    const TWO_CHANNEL_COMMAND: &str = "printf 'out-payload\\n'; printf 'err-payload\\n' >&2";

    /// The `cmd` twin. Compile-checked under `just check-windows`; not executed
    /// on a Windows host.
    #[cfg(windows)]
    const TWO_CHANNEL_COMMAND: &str = "echo out-payload& echo err-payload 1>&2";

    /// The channel contract: stdout is data, stderr is status, and only stdout
    /// is captured.
    ///
    /// Before this, stderr was `Stdio::inherit()` — it reached the terminal
    /// with no bar, no label, and no coordination with a sibling's writes.
    #[test]
    fn stdout_streams_to_the_data_channel_and_stderr_to_the_status_channel() {
        let sink = TimedSink::new();
        let stream = live("task", TaskBar::for_index(0), &sink);

        let output = SystemTaskShell::default()
            .run(
                TWO_CHANNEL_COMMAND,
                Duration::from_secs(30),
                None,
                Some(&stream),
            )
            .unwrap();

        assert_eq!(output.exit_code, 0);
        let data = sink.payloads(Channel::Data);
        let status = sink.payloads(Channel::Status);
        assert!(
            data.iter().any(|line| line.contains("out-payload")),
            "stdout never reached the data channel: {data:?}"
        );
        assert!(
            status.iter().any(|line| line.contains("err-payload")),
            "stderr never reached the status channel: {status:?}"
        );
        assert!(
            data.iter().all(|line| !line.contains("err-payload")),
            "stderr leaked onto the data channel, so `2>/dev/null` and `| jq` \
             now disagree about what the task produced: {data:?}"
        );

        // The capture boundary is unchanged: stderr is displayed, never
        // captured, so it cannot reach `outputs`.
        assert_eq!(output.stdout.trim_end(), "out-payload");
        assert!(
            !output.stdout.contains("err-payload"),
            "stderr entered the captured payload: {:?}",
            output.stdout
        );
    }

    /// Frames must arrive *while* the command runs, not in one batch after it
    /// returns.
    ///
    /// The sleep between the two lines is the discriminator. A buffered runner
    /// records both stamps at the end, a hair apart; a streaming one records
    /// them a sleep apart.
    #[cfg(unix)]
    #[test]
    fn a_long_running_command_streams_its_first_line_before_it_finishes() {
        let sink = TimedSink::new();
        let stream = live("task", TaskBar::for_index(0), &sink);

        let started = Instant::now();
        let output = SystemTaskShell::default()
            .run(
                "printf 'first-line\\n'; sleep 2; printf 'second-line\\n'",
                Duration::from_secs(30),
                None,
                Some(&stream),
            )
            .unwrap();
        let ran_for = started.elapsed();

        assert_eq!(output.exit_code, 0);
        assert!(
            ran_for >= Duration::from_secs(2),
            "the fixture did not actually sleep, so it discriminates nothing"
        );
        let first = sink
            .first_seen(Channel::Data, "first-line")
            .expect("the first line reached the data channel");
        let second = sink
            .first_seen(Channel::Data, "second-line")
            .expect("the second line reached the data channel");
        // A full second of the fixture's two-second sleep, so a loaded host
        // cannot turn a streaming run into a failure.
        assert!(
            second.saturating_sub(first) >= Duration::from_secs(1),
            "both lines reached the sink together, so the command was silent \
             until it exited: first at {first:?}, second at {second:?}"
        );
    }

    /// Two concurrent tasks interleave by line *arrival*, not by completion.
    ///
    /// The staggered sleeps make arrival order deterministic and different from
    /// completion order: `beta` produces its first line after `alpha`'s but
    /// finishes at the same time. A completion-ordered stream would emit
    /// `alpha-1 alpha-2 beta-1 beta-2`; an arrival-ordered one alternates.
    #[cfg(unix)]
    #[test]
    fn two_concurrent_tasks_interleave_in_line_arrival_order() {
        let sink = TimedSink::new();
        let alpha = live("alpha", TaskBar::for_index(0), &sink);
        let beta = live("beta", TaskBar::for_index(1), &sink);

        std::thread::scope(|scope| {
            for (stream, command) in [
                (
                    &alpha,
                    "printf 'alpha-1\\n'; sleep 1.2; printf 'alpha-2\\n'",
                ),
                (
                    &beta,
                    "sleep 0.6; printf 'beta-1\\n'; sleep 1.2; printf 'beta-2\\n'",
                ),
            ] {
                scope.spawn(move || {
                    SystemTaskShell::default()
                        .run(command, Duration::from_secs(30), None, Some(stream))
                        .unwrap();
                });
            }
        });

        let data = sink.payloads(Channel::Data);
        let markers: Vec<&str> = data
            .iter()
            .filter_map(|line| {
                ["alpha-1", "alpha-2", "beta-1", "beta-2"]
                    .into_iter()
                    .find(|marker| line.contains(marker))
            })
            .collect();
        assert_eq!(
            markers,
            vec!["alpha-1", "beta-1", "alpha-2", "beta-2"],
            "concurrent output is not in line arrival order: {data:?}"
        );
    }

    /// Under concurrent load every frame is a whole line — never a splice of
    /// two tasks' lines, never half of one.
    ///
    /// The synchronized sink is what guarantees this; the volume of lines is
    /// what would expose its absence. A frame carrying two markers, or a
    /// truncated marker, is a torn write.
    #[cfg(unix)]
    #[test]
    fn concurrent_tasks_never_tear_a_line_at_the_sink() {
        const LINES: usize = 300;

        let sink = TimedSink::new();
        let alpha = live("alpha", TaskBar::for_index(0), &sink);
        let beta = live("beta", TaskBar::for_index(1), &sink);

        std::thread::scope(|scope| {
            for (stream, prefix) in [(&alpha, "alpha"), (&beta, "beta")] {
                scope.spawn(move || {
                    SystemTaskShell::default()
                        .run(
                            &format!("for i in $(seq 1 {LINES}); do echo {prefix}-line-$i; done"),
                            Duration::from_secs(60),
                            None,
                            Some(stream),
                        )
                        .unwrap();
                });
            }
        });

        let expected: std::collections::HashSet<String> = ["alpha", "beta"]
            .into_iter()
            .flat_map(|prefix| (1..=LINES).map(move |i| format!("{prefix}-line-{i}")))
            .collect();
        let data = sink.payloads(Channel::Data);
        assert_eq!(
            data.len(),
            expected.len(),
            "every emitted line must reach the sink exactly once"
        );
        for line in &data {
            assert!(
                expected.contains(line),
                "a frame carried something other than one whole line — either a \
                 splice of two tasks or a fragment of one: {line:?}"
            );
        }
    }

    /// A command whose last line carries no newline still reaches the sink,
    /// per command rather than at task close.
    #[cfg(unix)]
    #[test]
    fn a_trailing_fragment_is_flushed_when_the_command_ends() {
        let sink = TimedSink::new();
        let stream = live("task", TaskBar::for_index(0), &sink);

        SystemTaskShell::default()
            .run(
                "printf 'no-trailing-newline'",
                Duration::from_secs(30),
                None,
                Some(&stream),
            )
            .unwrap();

        let data = sink.payloads(Channel::Data);
        assert!(
            data.iter().any(|line| line.contains("no-trailing-newline")),
            "a final line without its newline was held forever: {data:?}"
        );
    }

    /// Streaming must not change what the command reports as captured.
    #[cfg(unix)]
    #[test]
    fn multibyte_output_survives_the_chunked_reader_intact() {
        let sink = TimedSink::new();
        let stream = live("task", TaskBar::for_index(0), &sink);

        // Long enough to cross the 8 KiB read boundary many times over, so a
        // decoder that split a code point would show it.
        let output = SystemTaskShell::default()
            .run(
                "for i in $(seq 1 2000); do printf '日本語のタスク — ✅ 完了\\n'; done",
                Duration::from_secs(60),
                None,
                Some(&stream),
            )
            .unwrap();

        assert_eq!(output.stdout.lines().count(), 2000);
        assert!(
            !output.stdout.contains('\u{FFFD}'),
            "the capture buffer split a code point"
        );
        let data = sink.payloads(Channel::Data);
        assert_eq!(data.len(), 2000, "every line must reach the sink once");
        assert!(
            data.iter().all(|line| line.contains("日本語のタスク")),
            "the streamed text lost characters to a chunk boundary"
        );
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
        let runtime = Arc::new(RuntimeState::new());
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
        let runtime = Arc::new(RuntimeState::new());
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
        let runtime = Arc::new(RuntimeState::new());
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
        let runtime = Arc::new(RuntimeState::new());
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
        let runtime = Arc::new(RuntimeState::new());
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
        let runtime = Arc::new(RuntimeState::new());
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
        let runtime = Arc::new(RuntimeState::new());
        let mut wiring = Wiring::new(&recorder, &shell);
        wiring.prompt = &prompt;
        wiring.runtime = Some(&runtime);

        let outcome = fixture.execute(&wiring);

        assert_eq!(outcome.status, TaskStatus::Interrupted);
        assert!(outcome.error.is_none());
        assert_eq!(runtime.output_count(), 0);
    }

    /// The `interrupted` flag decides the status; the exit code does not get a
    /// vote. `130` is a Unix single-press accident, not the contract: the second
    /// press yields `137`, `CTRL_BREAK_EVENT` yields `0xC000013A`,
    /// `TerminateJobObject` yields `1`, and a provider that traps `SIGINT` and
    /// shuts down cleanly yields `0`. Each of those must still be recorded as an
    /// interruption rather than a failure (or, for `0`, rather than a success
    /// that commits output). The sibling test above pins `130` with the flag
    /// set, so it cannot tell the two signals apart — this one can.
    #[test]
    fn an_interrupted_provider_is_recorded_as_interrupted_whatever_its_exit_code() {
        for exit_code in [137, 1, 0xC000_013A_u32 as i32, 0] {
            let (dir, source) = prompt_fixture(json!({}));
            let fixture = Fixture::build(dir, &source).unwrap();
            let recorder = Recorder::default();
            let shell = FakeTaskShell::default();
            let prompt = FakePrompt::with(PromptRunOutcome {
                stdout: "partial".to_string(),
                exit_code,
                interrupted: true,
            });
            let runtime = Arc::new(RuntimeState::new());
            let mut wiring = Wiring::new(&recorder, &shell);
            wiring.prompt = &prompt;
            wiring.runtime = Some(&runtime);

            let outcome = fixture.execute(&wiring);

            assert_eq!(
                outcome.status,
                TaskStatus::Interrupted,
                "exit {exit_code} with the interrupt flag set must be an interruption",
            );
            assert!(outcome.error.is_none(), "exit {exit_code}");
            assert_eq!(
                runtime.output_count(),
                0,
                "an interrupted task commits nothing, even on exit {exit_code}",
            );
        }
    }

    /// The mirror of the case above: this contract reads only the flag, so a
    /// bare `130` arriving with the flag clear is an ordinary failure here. The
    /// `130`-as-witness rule lives one layer up, in the CLI's
    /// `run_was_interrupted`, which folds it into the flag before this point —
    /// keeping the exit-code heuristic out of the library contract entirely.
    #[test]
    fn exit_130_without_the_interrupt_flag_is_an_ordinary_failure_here() {
        let (dir, source) = prompt_fixture(json!({}));
        let fixture = Fixture::build(dir, &source).unwrap();
        let recorder = Recorder::default();
        let shell = FakeTaskShell::default();
        let prompt = FakePrompt::with(PromptRunOutcome {
            stdout: "partial".to_string(),
            exit_code: 130,
            interrupted: false,
        });
        let runtime = Arc::new(RuntimeState::new());
        let mut wiring = Wiring::new(&recorder, &shell);
        wiring.prompt = &prompt;
        wiring.runtime = Some(&runtime);

        let outcome = fixture.execute(&wiring);

        assert_eq!(outcome.status, TaskStatus::Failed);
        assert!(
            failure_message(&outcome).contains("exited with code 130"),
            "{}",
            failure_message(&outcome),
        );
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
            interrupted: false,
            aborted: false,
        }]);
        let runtime = Arc::new(RuntimeState::new());
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
            interrupted: false,
            aborted: false,
        }]);
        let runtime = Arc::new(RuntimeState::new());
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
            let runtime = Arc::new(RuntimeState::new());
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

}

// -- serial groups (phase 9) ------------------------------------------------

mod serial_groups {
    use super::*;

    /// The two-task bundle every definition site defines.
    fn bundle_tasks() -> Value {
        json!([
            { "name": "first", "shell": "one" },
            { "name": "second", "shell": "two" },
        ])
    }

    /// Write the group's file-backed definition sites into `root` and return
    /// the sequence step that reaches the bundle through `site`.
    fn step_for_site(root: &Path, site: &str) -> Value {
        let tasks = bundle_tasks();
        write_yaml(
            root,
            "group.yaml",
            &json!({ "kind": "group", "name": "bundle", "tasks": tasks }),
        );
        write_yaml(
            root,
            "catalog.yaml",
            &json!({
                "kind": "group-catalog",
                "groups": [
                    { "name": "other", "tasks": [{ "shell": "unused" }] },
                    { "name": "bundle", "tasks": tasks },
                ],
            }),
        );
        match site {
            "inline" => json!({ "name": "alpha", "group": { "name": "bundle", "tasks": tasks } }),
            "file" => json!({ "name": "alpha", "group": "group.yaml" }),
            "catalog" => json!({ "name": "alpha", "group": "bundle@catalog.yaml" }),
            other => panic!("unknown definition site `{other}`"),
        }
    }

    /// Inline, `kind: group`, and `{name}@{catalog}` are three spellings of one
    /// bundle — they must be indistinguishable once executed.
    #[test]
    fn inline_file_and_catalog_groups_are_behaviorally_equivalent() {
        let mut observed = Vec::new();
        for site in ["inline", "file", "catalog"] {
            // Each site gets its own directory so a stale write cannot leak
            // between the three runs.
            let dir = TempDir::new().unwrap();
            let source = one_step_source(dir.path(), step_for_site(dir.path(), site));
            let fixture = Fixture::build(dir, &source).unwrap();
            let recorder = Recorder::default();
            let shell = FakeTaskShell::with_stdout(&["out one", "out two"]);
            let runtime = Arc::new(RuntimeState::new());
            let mut wiring = Wiring::new(&recorder, &shell);
            wiring.runtime = Some(&runtime);

            let outcome = fixture.execute(&wiring);

            assert!(outcome.succeeded(), "{site}: {}", failure_message(&outcome));
            observed.push((
                site,
                shell.commands(),
                outcome
                    .group_tasks
                    .iter()
                    .map(|task| (task.name.clone(), task.status))
                    .collect::<Vec<_>>(),
                runtime.outputs_value(),
            ));
        }

        let (_, commands, tasks, outputs) = observed[0].clone();
        assert_eq!(commands, vec!["one".to_string(), "two".to_string()]);
        assert_eq!(
            tasks,
            vec![
                ("first".to_string(), TaskStatus::Succeeded),
                ("second".to_string(), TaskStatus::Succeeded),
            ],
        );
        assert_eq!(outputs, json!(["out one", "out two"]));
        for (label, other_commands, other_tasks, other_outputs) in &observed[1..] {
            assert_eq!(*other_commands, commands, "{label} commands diverged");
            assert_eq!(*other_tasks, tasks, "{label} task results diverged");
            assert_eq!(*other_outputs, outputs, "{label} outputs diverged");
        }
    }

    /// A serial group grows `outputs` entry by entry. It never adds a wrapper
    /// entry of its own on top of its members'.
    #[test]
    fn a_serial_group_appends_one_entry_per_task_and_none_of_its_own() {
        let dir = TempDir::new().unwrap();
        let source = one_step_source(
            dir.path(),
            json!({
                "name": "alpha",
                "group": {
                    "name": "bundle",
                    "tasks": [{ "shell": "one" }, { "shell": "two" }],
                },
            }),
        );
        let fixture = Fixture::build(dir, &source).unwrap();
        let recorder = Recorder::default();
        let shell = FakeTaskShell::with_stdout(&["alpha out", "beta out"]);
        let runtime = Arc::new(RuntimeState::new());
        let mut wiring = Wiring::new(&recorder, &shell);
        wiring.runtime = Some(&runtime);

        let outcome = fixture.execute(&wiring);

        assert!(outcome.succeeded(), "{}", failure_message(&outcome));
        assert_eq!(runtime.output_count(), 2);
        assert_eq!(runtime.outputs_value(), json!(["alpha out", "beta out"]));
        assert!(
            !outcome.output_committed,
            "the group itself must not commit an entry",
        );
    }

    /// Declaration order is the contract: task 2 reads what task 1 wrote and
    /// what task 1 produced.
    #[test]
    fn a_later_task_sees_the_earlier_task_mutation_and_output() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("target.md"), "---\ntitle: t\n---\n\nBody.\n").unwrap();
        let source = one_step_source(
            dir.path(),
            json!({
                "name": "alpha",
                "group": {
                    "name": "bundle",
                    "tasks": [
                        {
                            "name": "writer",
                            "shell": "one",
                            "teardown": [{ "action": { "set": ["marker", "written"] } }],
                        },
                        {
                            "name": "reader",
                            "prompt": "target.md",
                            "params": {
                                "seen": "{{ marker }}",
                                "prior": "{{ last(outputs) }}",
                            },
                        },
                    ],
                },
            }),
        );
        let fixture = Fixture::build(dir, &source).unwrap();
        let recorder = Recorder::default();
        let shell = FakeTaskShell::with_stdout(&["writer said this"]);
        let prompt = FakePrompt::succeeding("reader said that");
        let runtime = Arc::new(RuntimeState::new());
        let mut wiring = Wiring::new(&recorder, &shell);
        wiring.runtime = Some(&runtime);
        wiring.prompt = &prompt;

        let outcome = fixture.execute(&wiring);

        assert!(outcome.succeeded(), "{}", failure_message(&outcome));
        let request = prompt.last();
        assert_eq!(request.params["seen"], json!("written"));
        assert_eq!(request.params["prior"], json!("writer said this"));
        assert_eq!(
            runtime.outputs_value(),
            json!(["writer said this", "reader said that"]),
        );
    }

    /// Group variables are a scope: readable by member tasks, gone afterwards.
    #[test]
    fn group_variables_are_in_scope_for_members() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("target.md"), "---\ntitle: t\n---\n\nBody.\n").unwrap();
        let source = one_step_source(
            dir.path(),
            json!({
                "name": "alpha",
                "group": {
                    "name": "bundle",
                    "variables": { "label": "release", "attempt": 2 },
                    "tasks": [{
                        "name": "member",
                        "prompt": "target.md",
                        "params": { "which": "{{ group.label }}", "attempt": "{{ group.attempt }}" },
                        "setup": [{ "action": { "info": "starting {{ group.label }}" } }],
                    }],
                },
            }),
        );
        let fixture = Fixture::build(dir, &source).unwrap();
        let recorder = Recorder::default();
        let shell = FakeTaskShell::default();
        let prompt = FakePrompt::succeeding("done");
        let mut wiring = Wiring::new(&recorder, &shell);
        wiring.prompt = &prompt;

        let outcome = fixture.execute(&wiring);

        assert!(outcome.succeeded(), "{}", failure_message(&outcome));
        let request = prompt.last();
        assert_eq!(request.params["which"], json!("release"));
        // Whole-value typing survives the scope: `attempt` stays a number.
        assert_eq!(request.params["attempt"], json!(2));
        assert_eq!(recorder.events(), vec!["info:starting release".to_string()]);
        assert_eq!(request.set_overrides["group"]["label"], json!("release"));
    }

    /// The scope ends with the group: a later sequence step referencing
    /// `group.*` gets the unknown-root refusal, not a stale value.
    #[test]
    fn group_variables_do_not_leak_to_a_later_step() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("target.md"), "---\ntitle: t\n---\n\nBody.\n").unwrap();
        let source = write_source(
            dir.path(),
            "seq.md",
            &[(
                "sequence",
                json!([
                    {
                        "name": "alpha",
                        "group": {
                            "name": "bundle",
                            "variables": { "label": "release" },
                            "tasks": [{ "shell": "one" }],
                        },
                    },
                    {
                        "name": "beta",
                        "prompt": "target.md",
                        "params": { "which": "{{ group.label }}" },
                    },
                ]),
            )],
            "Body.\n",
        );
        let fixture = Fixture::build(dir, &source).unwrap();
        let recorder = Recorder::default();
        let shell = FakeTaskShell::with_stdout(&["one out"]);
        let prompt = FakePrompt::succeeding("done");
        let mut wiring = Wiring::new(&recorder, &shell);
        wiring.prompt = &prompt;

        let group_outcome = fixture.execute_step(0, &wiring);
        assert!(
            group_outcome.succeeded(),
            "{}",
            failure_message(&group_outcome),
        );

        let later = fixture.execute_step(1, &wiring);

        assert_eq!(later.status, TaskStatus::Failed);
        assert!(
            failure_message(&later).contains("params.which"),
            "{}",
            failure_message(&later),
        );
        assert!(
            prompt.requests.lock().unwrap().is_empty(),
            "the later step must not have launched with a leaked scope",
        );
    }

    /// The first failure stops the group; the rest of the bundle is left
    /// unexecuted and the owning step is failed.
    #[test]
    fn the_first_failed_task_stops_the_group() {
        let dir = TempDir::new().unwrap();
        let source = one_step_source(
            dir.path(),
            json!({
                "name": "alpha",
                "group": {
                    "name": "bundle",
                    "tasks": [
                        { "name": "ok", "shell": "one" },
                        { "name": "boom", "shell": "two" },
                        { "name": "never", "shell": "three" },
                    ],
                },
            }),
        );
        let fixture = Fixture::build(dir, &source).unwrap();
        let recorder = Recorder::default();
        let shell = FakeTaskShell::with_results(vec![
            ShellCommandOutput {
                stdout: "one out\n".to_string(),
                exit_code: 0,
                timed_out: false,
                interrupted: false,
                aborted: false,
            },
            ShellCommandOutput {
                stdout: String::new(),
                exit_code: 3,
                timed_out: false,
                interrupted: false,
                aborted: false,
            },
        ]);
        let runtime = Arc::new(RuntimeState::new());
        let mut wiring = Wiring::new(&recorder, &shell);
        wiring.runtime = Some(&runtime);

        let outcome = fixture.execute(&wiring);

        assert_eq!(outcome.status, TaskStatus::Failed);
        assert_eq!(shell.commands(), vec!["one".to_string(), "two".to_string()]);
        assert_eq!(
            outcome
                .group_tasks
                .iter()
                .map(|task| (task.name.as_str(), task.status))
                .collect::<Vec<_>>(),
            vec![("ok", TaskStatus::Succeeded), ("boom", TaskStatus::Failed)],
        );
        // Only the successful member committed an entry.
        assert_eq!(runtime.outputs_value(), json!(["one out"]));
        assert!(failure_message(&outcome).contains("exited with code 3"));
    }

    /// An interrupt inside a group stops it as an interruption, not a failure,
    /// so the sequence exits `130` rather than consulting `fail_fast`.
    #[test]
    fn an_interrupt_stops_the_group_as_an_interruption() {
        let dir = TempDir::new().unwrap();
        let source = one_step_source(
            dir.path(),
            json!({
                "name": "alpha",
                "group": {
                    "name": "bundle",
                    "tasks": [{ "shell": "one" }, { "shell": "two" }],
                },
            }),
        );
        let fixture = Fixture::build(dir, &source).unwrap();
        let recorder = Recorder::default();
        let shell = FakeTaskShell::with_stdout(&["one out"]);
        let interrupt = AtomicBool::new(true);
        let runtime = Arc::new(RuntimeState::new());
        let mut wiring = Wiring::new(&recorder, &shell);
        wiring.runtime = Some(&runtime);
        wiring.interrupt = Some(&interrupt);

        let outcome = fixture.execute(&wiring);

        assert_eq!(outcome.status, TaskStatus::Interrupted);
        assert!(shell.commands().is_empty());
        assert_eq!(runtime.output_count(), 0);
    }

    /// A single-task group is legal (`min(1)`) and behaves like the task alone,
    /// except that the group reports it as a member.
    #[test]
    fn a_single_task_group_runs_that_task() {
        let dir = TempDir::new().unwrap();
        let source = one_step_source(
            dir.path(),
            json!({
                "name": "alpha",
                "group": { "name": "bundle", "tasks": [{ "name": "only", "shell": "solo" }] },
            }),
        );
        let fixture = Fixture::build(dir, &source).unwrap();
        let recorder = Recorder::default();
        let shell = FakeTaskShell::with_stdout(&["solo out"]);
        let runtime = Arc::new(RuntimeState::new());
        let mut wiring = Wiring::new(&recorder, &shell);
        wiring.runtime = Some(&runtime);

        let outcome = fixture.execute(&wiring);

        assert!(outcome.succeeded(), "{}", failure_message(&outcome));
        assert_eq!(shell.commands(), vec!["solo".to_string()]);
        assert_eq!(outcome.group_tasks.len(), 1);
        assert_eq!(outcome.group_tasks[0].name, "only");
        assert_eq!(runtime.outputs_value(), json!(["solo out"]));
    }

    /// A group task may itself be an external `kind: task` file; the reference
    /// resolves from the directory of the group document that authored it.
    #[test]
    fn an_external_task_inside_a_group_resolves_from_the_group_document() {
        let dir = TempDir::new().unwrap();
        let nested = dir.path().join("bundle");
        fs::create_dir(&nested).unwrap();
        write_yaml(
            &nested,
            "task.yaml",
            &json!({ "kind": "task", "name": "external", "shell": "nested-command" }),
        );
        write_yaml(
            &nested,
            "group.yaml",
            &json!({
                "kind": "group",
                "name": "bundle",
                "tasks": [{ "task": "task.yaml" }],
            }),
        );
        let source = one_step_source(
            dir.path(),
            json!({ "name": "alpha", "group": "bundle/group.yaml" }),
        );
        let fixture = Fixture::build(dir, &source).unwrap();
        let recorder = Recorder::default();
        let shell = FakeTaskShell::with_stdout(&["nested out"]);

        let outcome = fixture.execute(&Wiring::new(&recorder, &shell));

        assert!(outcome.succeeded(), "{}", failure_message(&outcome));
        assert_eq!(shell.commands(), vec!["nested-command".to_string()]);
        assert_eq!(outcome.group_tasks[0].name, "external");
    }
}

// -- parallel groups (phase 10) ---------------------------------------------

/// Concurrency is only worth testing if the tests cannot pass by accident under
/// serial execution. Every case here therefore either inverts completion order
/// against declaration order, observes the in-flight high-water mark, or asserts
/// an isolation property a shared-cell serial run would violate.
mod parallel_groups {
    use super::*;

    /// A parallel group of `commands`, each task named after its command.
    fn parallel_step(commands: &[&str], max_parallel: Option<usize>) -> Value {
        let mut group = serde_json::Map::new();
        group.insert("name".to_string(), json!("bundle"));
        group.insert("execution".to_string(), json!("parallel"));
        group.insert(
            "tasks".to_string(),
            Value::Array(
                commands
                    .iter()
                    .map(|command| json!({ "name": command, "shell": command }))
                    .collect(),
            ),
        );
        if let Some(cap) = max_parallel {
            group.insert("max_parallel".to_string(), json!(cap));
        }
        json!({ "name": "alpha", "group": Value::Object(group) })
    }

    /// `max_parallel` is a hard ceiling on simultaneous tasks, not a hint.
    #[test]
    fn max_parallel_caps_the_number_of_tasks_in_flight() {
        let dir = TempDir::new().unwrap();
        let source = one_step_source(dir.path(), parallel_step(&["a", "b", "c", "d"], Some(2)));
        let fixture = Fixture::build(dir, &source).unwrap();
        let recorder = Recorder::default();
        // Every task blocks, so all four would overlap without the cap.
        let shell = ["a", "b", "c", "d"]
            .iter()
            .fold(FakeTaskShell::default(), |shell, command| {
                shell.stdout_for(command, command).delay(command, 40)
            });
        let runtime = Arc::new(RuntimeState::new());
        let mut wiring = Wiring::new(&recorder, &shell);
        wiring.runtime = Some(&runtime);

        let outcome = fixture.execute(&wiring);

        assert!(outcome.succeeded(), "{}", failure_message(&outcome));
        assert_eq!(shell.peak_in_flight(), 2, "`max_parallel: 2` was exceeded");
        assert_eq!(runtime.outputs_value(), json!([["a", "b", "c", "d"]]));
    }

    /// Absent `max_parallel` means "launch them all".
    #[test]
    fn an_absent_cap_launches_every_task_at_once() {
        let dir = TempDir::new().unwrap();
        let source = one_step_source(dir.path(), parallel_step(&["a", "b", "c"], None));
        let fixture = Fixture::build(dir, &source).unwrap();
        let recorder = Recorder::default();
        let shell = ["a", "b", "c"]
            .iter()
            .fold(FakeTaskShell::default(), |shell, command| {
                shell.stdout_for(command, command).delay(command, 60)
            });
        let mut wiring = Wiring::new(&recorder, &shell);
        let runtime = Arc::new(RuntimeState::new());
        wiring.runtime = Some(&runtime);

        let outcome = fixture.execute(&wiring);

        assert!(outcome.succeeded(), "{}", failure_message(&outcome));
        assert_eq!(shell.peak_in_flight(), 3);
    }

    /// A cap of one is still concurrency machinery, but nothing ever overlaps.
    #[test]
    fn a_cap_of_one_never_overlaps() {
        let dir = TempDir::new().unwrap();
        let source = one_step_source(dir.path(), parallel_step(&["a", "b"], Some(1)));
        let fixture = Fixture::build(dir, &source).unwrap();
        let recorder = Recorder::default();
        let shell = FakeTaskShell::default()
            .stdout_for("a", "a")
            .stdout_for("b", "b")
            .delay("a", 30)
            .delay("b", 30);
        let runtime = Arc::new(RuntimeState::new());
        let mut wiring = Wiring::new(&recorder, &shell);
        wiring.runtime = Some(&runtime);

        let outcome = fixture.execute(&wiring);

        assert!(outcome.succeeded(), "{}", failure_message(&outcome));
        assert_eq!(shell.peak_in_flight(), 1);
        assert_eq!(runtime.outputs_value(), json!([["a", "b"]]));
    }

    /// The nested entry is positional, so inverting completion order against
    /// declaration order must not move a single string.
    #[test]
    fn the_nested_entry_is_declaration_ordered_under_inverted_completion() {
        let dir = TempDir::new().unwrap();
        let source = one_step_source(dir.path(), parallel_step(&["first", "second", "third"], None));
        let fixture = Fixture::build(dir, &source).unwrap();
        let recorder = Recorder::default();
        // Declaration order is first→third; completion order is third→first.
        let shell = FakeTaskShell::default()
            .stdout_for("first", "out first")
            .stdout_for("second", "out second")
            .stdout_for("third", "out third")
            .delay("first", 90)
            .delay("second", 45)
            .delay("third", 0);
        let runtime = Arc::new(RuntimeState::new());
        let mut wiring = Wiring::new(&recorder, &shell);
        wiring.runtime = Some(&runtime);

        let outcome = fixture.execute(&wiring);

        assert!(outcome.succeeded(), "{}", failure_message(&outcome));
        assert_eq!(
            runtime.outputs_value(),
            json!([["out first", "out second", "out third"]]),
            "the entry must follow declaration order, not completion order",
        );
        assert_eq!(
            outcome
                .group_tasks
                .iter()
                .map(|task| task.name.clone())
                .collect::<Vec<_>>(),
            vec!["first", "second", "third"],
        );
    }

    /// One nested entry, not one per member — that is what distinguishes a
    /// parallel group from a serial one in `outputs`.
    #[test]
    fn a_parallel_group_commits_exactly_one_nested_entry() {
        let dir = TempDir::new().unwrap();
        let source = one_step_source(dir.path(), parallel_step(&["a", "b"], None));
        let fixture = Fixture::build(dir, &source).unwrap();
        let recorder = Recorder::default();
        let shell = FakeTaskShell::default()
            .stdout_for("a", "one")
            .stdout_for("b", "two");
        let runtime = Arc::new(RuntimeState::new());
        let mut wiring = Wiring::new(&recorder, &shell);
        wiring.runtime = Some(&runtime);

        let outcome = fixture.execute(&wiring);

        assert!(outcome.succeeded(), "{}", failure_message(&outcome));
        assert_eq!(runtime.output_count(), 1);
        assert_eq!(runtime.outputs_value(), json!([["one", "two"]]));
        assert!(
            outcome.output_committed,
            "the group owns the entry its members do not commit",
        );
    }

    /// Snapshot isolation: a sibling's `set` is not observable mid-group, so a
    /// task interpolating the key reads the value from *before* the group.
    #[test]
    fn a_sibling_mutation_is_invisible_until_the_group_completes() {
        let dir = TempDir::new().unwrap();
        let source = write_source(
            dir.path(),
            "seq.md",
            &[
                ("marker", json!("initial")),
                ("sequence", json!([{
                "name": "alpha",
                "group": {
                    "name": "bundle",
                    "execution": "parallel",
                    "tasks": [
                        { "name": "writer", "shell": "write", "setup": [{ "action": [{ "set": ["marker", "written"] }] }] },
                        { "name": "reader", "shell": "read", "setup": [{ "action": [{ "set": ["observed", "{{ marker }}"] }] }] },
                    ],
                },
            }])),
            ],
            "Document body.\n",
        );
        let fixture = Fixture::build(dir, &source).unwrap();
        let recorder = Recorder::default();
        // The writer finishes first in wall-clock terms; a shared cell would let
        // the reader observe it.
        let shell = FakeTaskShell::default()
            .stdout_for("write", "w")
            .stdout_for("read", "r")
            .delay("read", 60);
        let runtime = Arc::new(RuntimeState::new());
        let mut wiring = Wiring::new(&recorder, &shell);
        wiring.runtime = Some(&runtime);

        let outcome = fixture.execute(&wiring);

        assert!(outcome.succeeded(), "{}", failure_message(&outcome));
        let merged = runtime.snapshot().mutations;
        assert_eq!(
            merged.get("observed"),
            Some(&json!("initial")),
            "the reader must see the group-start value, not the sibling's write",
        );
        assert_eq!(
            merged.get("marker"),
            Some(&json!("written")),
            "the write must still land once the group completes",
        );
    }

    /// Snapshot isolation for `outputs`: a member commits into its own buffer,
    /// so the sequence's accumulator does not grow task by task the way a serial
    /// group's does — it gains exactly one nested entry when the group finishes.
    #[test]
    fn member_entries_land_in_private_buffers_not_the_sequence_accumulator() {
        let dir = TempDir::new().unwrap();
        let source = one_step_source(dir.path(), parallel_step(&["p", "k", "z"], None));
        let fixture = Fixture::build(dir, &source).unwrap();
        let recorder = Recorder::default();
        let shell = ["p", "k", "z"]
            .iter()
            .fold(FakeTaskShell::default(), |shell, command| {
                shell.stdout_for(command, command)
            });
        let runtime = Arc::new(RuntimeState::new());
        runtime.append_output("before the group");
        let mut wiring = Wiring::new(&recorder, &shell);
        wiring.runtime = Some(&runtime);

        let outcome = fixture.execute(&wiring);

        assert!(outcome.succeeded(), "{}", failure_message(&outcome));
        assert_eq!(
            runtime.output_count(),
            2,
            "three members must add one nested entry, not three flat ones",
        );
        assert_eq!(
            runtime.outputs_value(),
            json!(["before the group", ["p", "k", "z"]]),
        );
    }

    /// A failure never cancels a sibling: canceling a mid-flight agent run
    /// discards useful work.
    #[test]
    fn every_sibling_runs_to_completion_after_one_fails() {
        let dir = TempDir::new().unwrap();
        let source = one_step_source(dir.path(), parallel_step(&["boom", "slow"], None));
        let fixture = Fixture::build(dir, &source).unwrap();
        let recorder = Recorder::default();
        // The failure lands first; the survivor is still running when it does.
        let shell = FakeTaskShell::default()
            .stdout_for("boom", "partial output")
            .failing("boom")
            .stdout_for("slow", "finished anyway")
            .delay("slow", 60);
        let runtime = Arc::new(RuntimeState::new());
        let mut wiring = Wiring::new(&recorder, &shell);
        wiring.runtime = Some(&runtime);

        let outcome = fixture.execute(&wiring);

        assert_eq!(outcome.status, TaskStatus::Failed, "any failure fails the group");
        assert_eq!(
            outcome
                .group_tasks
                .iter()
                .map(|task| (task.name.clone(), task.status))
                .collect::<Vec<_>>(),
            vec![
                ("boom".to_string(), TaskStatus::Failed),
                ("slow".to_string(), TaskStatus::Succeeded),
            ],
        );
        assert_eq!(
            runtime.outputs_value(),
            json!([["partial output", "finished anyway"]]),
            "a failed task keeps its slot and its partial stdout",
        );
    }

    /// Disjoint keys — the expected case — all survive, and nothing warns.
    #[test]
    fn disjoint_mutations_all_merge_without_a_warning() {
        let dir = TempDir::new().unwrap();
        let source = one_step_source(
            dir.path(),
            json!({
                "name": "alpha",
                "group": {
                    "name": "bundle",
                    "execution": "parallel",
                    "tasks": [
                        { "name": "left", "shell": "l", "setup": [{ "action": [{ "set": ["left_key", "L"] }] }] },
                        { "name": "right", "shell": "r", "setup": [{ "action": [{ "set": ["right_key", "R"] }] }] },
                    ],
                },
            }),
        );
        let fixture = Fixture::build(dir, &source).unwrap();
        let recorder = Recorder::default();
        let shell = FakeTaskShell::default()
            .stdout_for("l", "l")
            .stdout_for("r", "r");
        let runtime = Arc::new(RuntimeState::new());
        let mut wiring = Wiring::new(&recorder, &shell);
        wiring.runtime = Some(&runtime);

        let outcome = fixture.execute(&wiring);

        assert!(outcome.succeeded(), "{}", failure_message(&outcome));
        let mutations = runtime.snapshot().mutations;
        assert_eq!(mutations.get("left_key"), Some(&json!("L")));
        assert_eq!(mutations.get("right_key"), Some(&json!("R")));
        assert!(
            !recorder.events().iter().any(|event| event.starts_with("warn:")),
            "disjoint writes are the expected case: {:?}",
            recorder.events(),
        );
    }

    /// A contested key resolves by declaration order, not completion order, and
    /// says so on stderr.
    #[test]
    fn a_contested_key_resolves_to_the_later_declared_task_and_warns() {
        let dir = TempDir::new().unwrap();
        let source = one_step_source(
            dir.path(),
            json!({
                "name": "alpha",
                "group": {
                    "name": "bundle",
                    "execution": "parallel",
                    "tasks": [
                        { "name": "early", "shell": "e", "setup": [{ "action": [{ "set": ["shared", "from-early"] }] }] },
                        { "name": "late", "shell": "l", "setup": [{ "action": [{ "set": ["shared", "from-late"] }] }] },
                    ],
                },
            }),
        );
        let fixture = Fixture::build(dir, &source).unwrap();
        let recorder = Recorder::default();
        // The later-declared task finishes *first*, so a completion-order merge
        // would leave `from-early` behind.
        let shell = FakeTaskShell::default()
            .stdout_for("e", "e")
            .stdout_for("l", "l")
            .delay("e", 60);
        let runtime = Arc::new(RuntimeState::new());
        let mut wiring = Wiring::new(&recorder, &shell);
        wiring.runtime = Some(&runtime);

        let outcome = fixture.execute(&wiring);

        assert!(outcome.succeeded(), "{}", failure_message(&outcome));
        assert_eq!(
            runtime.snapshot().mutations.get("shared"),
            Some(&json!("from-late")),
            "the later-declared task wins regardless of who finished first",
        );
        let warning = recorder
            .events()
            .into_iter()
            .find(|event| event.starts_with("warn:"))
            .expect("a contested key must warn");
        for fragment in ["shared", "early", "late"] {
            assert!(
                warning.contains(fragment),
                "the warning must name the key and both tasks: {warning}",
            );
        }
    }

    /// Ctrl+C fans out: every member observes the shared flag, the group reports
    /// interruption, and nothing is committed.
    #[test]
    fn an_interrupt_fans_out_and_commits_neither_outputs_nor_mutations() {
        let dir = TempDir::new().unwrap();
        let source = one_step_source(
            dir.path(),
            json!({
                "name": "alpha",
                "group": {
                    "name": "bundle",
                    "execution": "parallel",
                    "tasks": [
                        { "name": "a", "shell": "a", "setup": [{ "action": [{ "set": ["touched", "yes"] }] }] },
                        { "name": "b", "shell": "b" },
                    ],
                },
            }),
        );
        let fixture = Fixture::build(dir, &source).unwrap();
        let recorder = Recorder::default();
        let shell = FakeTaskShell::default()
            .stdout_for("a", "a")
            .stdout_for("b", "b");
        let runtime = Arc::new(RuntimeState::new());
        let interrupt = AtomicBool::new(true);
        let mut wiring = Wiring::new(&recorder, &shell);
        wiring.runtime = Some(&runtime);
        wiring.interrupt = Some(&interrupt);

        let outcome = fixture.execute(&wiring);

        assert_eq!(outcome.status, TaskStatus::Interrupted);
        assert_eq!(
            runtime.output_count(),
            0,
            "an interrupted group commits no nested entry",
        );
        assert!(
            runtime.snapshot().mutations.is_empty(),
            "an interrupted group merges no mutations",
        );
    }

    /// The whole observable tuple must be byte-identical across runs whose
    /// completion order differs.
    #[test]
    fn repeated_runs_produce_identical_results_regardless_of_completion_order() {
        // Two orderings of the same delays: first-slowest, then last-slowest.
        let orderings = [[80_u64, 40, 0], [0, 40, 80]];
        let mut observed = Vec::new();
        for delays in orderings {
            let dir = TempDir::new().unwrap();
            let source = one_step_source(
                dir.path(),
                json!({
                    "name": "alpha",
                    "group": {
                        "name": "bundle",
                        "execution": "parallel",
                        "tasks": [
                            { "name": "one", "shell": "one", "setup": [{ "action": [{ "set": ["k1", 1] }] }] },
                            { "name": "two", "shell": "two", "setup": [{ "action": [{ "set": ["k2", 2] }] }] },
                            { "name": "three", "shell": "three", "setup": [{ "action": [{ "set": ["k3", 3] }] }] },
                        ],
                    },
                }),
            );
            let fixture = Fixture::build(dir, &source).unwrap();
            let recorder = Recorder::default();
            let mut shell = FakeTaskShell::default();
            for (command, delay) in ["one", "two", "three"].iter().zip(delays) {
                shell = shell.stdout_for(command, command).delay(command, delay);
            }
            let runtime = Arc::new(RuntimeState::new());
            let mut wiring = Wiring::new(&recorder, &shell);
            wiring.runtime = Some(&runtime);

            let outcome = fixture.execute(&wiring);

            assert!(outcome.succeeded(), "{}", failure_message(&outcome));
            observed.push((
                runtime.outputs_value(),
                Value::Object(runtime.snapshot().mutations.into_iter().collect()),
                outcome
                    .group_tasks
                    .iter()
                    .map(|task| (task.name.clone(), task.status))
                    .collect::<Vec<_>>(),
            ));
        }

        assert_eq!(
            observed[0], observed[1],
            "completion order must not reach any observable result",
        );
        assert_eq!(observed[0].0, json!([["one", "two", "three"]]));
    }

    /// Concurrency must never reach process-global state: `setenv` is not
    /// thread-safe and a shared CWD cannot describe two tasks at once.
    #[test]
    fn parallel_execution_leaves_process_env_and_cwd_untouched() {
        let dir = TempDir::new().unwrap();
        let source = one_step_source(dir.path(), parallel_step(&["a", "b", "c"], None));
        let fixture = Fixture::build(dir, &source).unwrap();
        let recorder = Recorder::default();
        let shell = ["a", "b", "c"]
            .iter()
            .fold(FakeTaskShell::default(), |shell, command| {
                shell.stdout_for(command, command)
            });
        let runtime = Arc::new(RuntimeState::new());
        let mut wiring = Wiring::new(&recorder, &shell);
        wiring.runtime = Some(&runtime);

        let cwd_before = std::env::current_dir().unwrap();
        let env_before: Vec<(String, String)> = std::env::vars().collect();

        let outcome = fixture.execute(&wiring);

        assert!(outcome.succeeded(), "{}", failure_message(&outcome));
        assert_eq!(std::env::current_dir().unwrap(), cwd_before);
        assert_eq!(std::env::vars().collect::<Vec<_>>(), env_before);
    }

    /// A parallel group has no interactive surface: a task's own value
    /// resolution failing is a task failure, never a prompt. N concurrent tasks
    /// cannot share one terminal.
    #[test]
    fn an_unresolvable_task_value_fails_the_task_rather_than_prompting() {
        let dir = TempDir::new().unwrap();
        let source = one_step_source(
            dir.path(),
            json!({
                "name": "alpha",
                "group": {
                    "name": "bundle",
                    "execution": "parallel",
                    "tasks": [
                        { "name": "ok", "shell": "fine" },
                        { "name": "broken", "shell": "x", "timeout": "{{ nope.missing }}" },
                    ],
                },
            }),
        );
        let fixture = Fixture::build(dir, &source).unwrap();
        let recorder = Recorder::default();
        let shell = FakeTaskShell::default()
            .stdout_for("fine", "fine")
            .stdout_for("x", "x");
        let runtime = Arc::new(RuntimeState::new());
        let mut wiring = Wiring::new(&recorder, &shell);
        wiring.runtime = Some(&runtime);

        let outcome = fixture.execute(&wiring);

        assert_eq!(outcome.status, TaskStatus::Failed);
        assert_eq!(
            outcome
                .group_tasks
                .iter()
                .map(|task| (task.name.clone(), task.status))
                .collect::<Vec<_>>(),
            vec![
                ("ok".to_string(), TaskStatus::Succeeded),
                ("broken".to_string(), TaskStatus::Failed),
            ],
        );
        assert_eq!(
            runtime.outputs_value(),
            json!([["fine", ""]]),
            "the failed task keeps an (empty) slot",
        );
    }
}

/// Attribution of group-task frames, from the scheduler's side.
///
/// The renderer's own contract — geometry, wrapping, palette, degradation —
/// lives in `render::task_stream::tests`. These cases prove the *scheduler*
/// picks the right bar, opens and closes exactly one stream per member task,
/// and never tears a sibling's frame group under real concurrency.
mod group_framing {
    use super::*;

    /// Records whole write calls, so a torn write shows up as a split group.
    #[derive(Default)]
    struct FrameSink {
        writes: Mutex<Vec<(Channel, Vec<String>)>>,
    }

    impl FrameSink {
        fn writes(&self) -> Vec<(Channel, Vec<String>)> {
            self.writes.lock().unwrap().clone()
        }

        /// Every rendered line on either channel, flattened.
        fn lines(&self) -> Vec<String> {
            self.writes()
                .into_iter()
                .flat_map(|(_, frames)| frames)
                .collect()
        }

        /// Every rendered line on one channel, flattened.
        fn channel_lines(&self, channel: Channel) -> Vec<String> {
            self.writes()
                .into_iter()
                .filter(|(seen, _)| *seen == channel)
                .flat_map(|(_, frames)| frames)
                .collect()
        }

        /// [`Self::lines`] with SGR sequences removed, for content assertions.
        fn visible_lines(&self) -> Vec<String> {
            self.lines().iter().map(|line| strip_ansi(line)).collect()
        }

        /// [`Self::channel_lines`] with SGR sequences removed.
        fn visible_channel_lines(&self, channel: Channel) -> Vec<String> {
            self.channel_lines(channel)
                .iter()
                .map(|line| strip_ansi(line))
                .collect()
        }
    }

    impl TaskStreamSink for FrameSink {
        fn write_frames(&self, frames: &[String]) {
            self.writes
                .lock()
                .unwrap()
                .push((Channel::Status, frames.to_vec()));
        }

        fn write_data_frames(&self, frames: &[String]) {
            self.writes
                .lock()
                .unwrap()
                .push((Channel::Data, frames.to_vec()));
        }
    }

    /// A group of shell tasks, one per command, named after its command.
    fn group_step(commands: &[&str], execution: &str) -> Value {
        json!({
            "name": "alpha",
            "group": {
                "name": "bundle",
                "execution": execution,
                "tasks": commands
                    .iter()
                    .map(|command| json!({ "name": command, "shell": command }))
                    .collect::<Vec<_>>(),
            },
        })
    }

    fn run(commands: &[&str], execution: &str, sink: &Arc<FrameSink>) -> TaskOutcome {
        run_on(commands, execution, sink, Terminal::default())
    }

    /// [`run`], rendering against an explicitly constructed terminal.
    fn run_on(
        commands: &[&str],
        execution: &str,
        sink: &Arc<FrameSink>,
        term: Terminal,
    ) -> TaskOutcome {
        let dir = TempDir::new().unwrap();
        let source = one_step_source(dir.path(), group_step(commands, execution));
        let fixture = Fixture::build(dir, &source).unwrap().with_terminal(term);
        let recorder = Recorder::default();
        let shell = commands
            .iter()
            .fold(FakeTaskShell::default(), |shell, command| {
                shell.stdout_for(command, command)
            });
        let runtime = Arc::new(RuntimeState::new());
        let mut wiring = Wiring::new(&recorder, &shell).with_stream(Arc::clone(sink) as Arc<dyn TaskStreamSink>);
        wiring.runtime = Some(&runtime);
        fixture.execute(&wiring)
    }

    #[test]
    fn every_member_task_opens_and_closes_exactly_one_stream() {
        let sink = Arc::new(FrameSink::default());
        let outcome = run(&["alpha", "bravo", "charlie"], "parallel", &sink);

        assert!(outcome.succeeded(), "{}", failure_message(&outcome));
        let lines = sink.visible_lines();
        for name in ["alpha", "bravo", "charlie"] {
            let headers = lines.iter().filter(|l| l.contains(&format!("▶ {name}"))).count();
            let footers = lines
                .iter()
                .filter(|l| l.contains(name) && l.contains("succeeded"))
                .count();
            assert_eq!(headers, 1, "task `{name}` headers in {lines:?}");
            assert_eq!(footers, 1, "task `{name}` footers in {lines:?}");
        }
    }

    #[test]
    fn a_parallel_group_gives_each_task_its_own_palette_entry() {
        let sink = Arc::new(FrameSink::default());
        // `new_forced` rather than `default`: the palette only exists on a
        // color-capable terminal, and a gate must not change verdict because the
        // host exported `NO_COLOR` or a colorless `TERM`.
        run_on(
            &["alpha", "bravo", "charlie"],
            "parallel",
            &sink,
            Terminal::new_forced(),
        );

        let bars: Vec<String> = sink
            .lines()
            .iter()
            .filter(|line| line.contains('│'))
            .map(|line| line[..line.find('│').unwrap() + '│'.len_utf8()].to_string())
            .collect();
        let distinct: std::collections::HashSet<&String> = bars.iter().collect();
        assert!(
            !bars.is_empty(),
            "a parallel group drew no bars: {:?}",
            sink.lines()
        );
        assert_eq!(distinct.len(), 3, "three tasks shared a color: {bars:?}");
    }

    #[test]
    fn a_no_color_terminal_still_attributes_every_task_by_name() {
        let sink = Arc::new(FrameSink::default());
        let term = Terminal::builder().color_depth(ColorDepth::None).build();
        run_on(&["alpha", "bravo", "charlie"], "parallel", &sink, term);

        let lines = sink.lines();
        assert!(
            lines.iter().all(|line| !line.contains('\u{1b}')),
            "a colorless terminal was still sent an escape sequence: {lines:?}"
        );
        // Color is the redundant channel; the name on every header and footer is
        // the one that must survive its loss (spec → *Reporting Concurrency*).
        for name in ["alpha", "bravo", "charlie"] {
            assert!(
                lines.iter().any(|l| l.contains(name) && l.contains("succeeded")),
                "task `{name}` lost its footer attribution: {lines:?}"
            );
        }
    }

    #[test]
    fn a_serial_group_uses_the_invisible_bar_at_the_same_left_edge() {
        let serial = Arc::new(FrameSink::default());
        run(&["alpha", "bravo"], "serial", &serial);
        let parallel = Arc::new(FrameSink::default());
        run(&["alpha", "bravo"], "parallel", &parallel);

        let serial_lines = serial.visible_lines();
        assert!(
            serial_lines.iter().all(|line| !line.contains('│')),
            "serial work drew a visible bar: {serial_lines:?}"
        );
        // Same geometry: the content column is identical in both modes.
        let column = |lines: &[String], needle: &str| -> usize {
            let line = lines
                .iter()
                .find(|line| line.contains(needle))
                .unwrap_or_else(|| panic!("no line carrying {needle:?} in {lines:?}"));
            let byte = line.find(needle).expect("needle present");
            line[..byte].chars().count()
        };
        assert_eq!(
            column(&serial_lines, "▶"),
            column(&parallel.visible_lines(), "▶"),
            "serial and parallel headers start at different columns"
        );
    }

    #[test]
    fn a_failed_task_reports_its_own_outcome_in_its_footer() {
        let dir = TempDir::new().unwrap();
        let source = one_step_source(dir.path(), group_step(&["fine-task", "bad-task"], "parallel"));
        let fixture = Fixture::build(dir, &source).unwrap();
        let recorder = Recorder::default();
        let shell = FakeTaskShell::default()
            .stdout_for("fine-task", "fine")
            .stdout_for("bad-task", "partial")
            .failing("bad-task");
        let runtime = Arc::new(RuntimeState::new());
        let sink = Arc::new(FrameSink::default());
        let mut wiring = Wiring::new(&recorder, &shell).with_stream(Arc::clone(&sink) as Arc<dyn TaskStreamSink>);
        wiring.runtime = Some(&runtime);

        let outcome = fixture.execute(&wiring);

        assert_eq!(outcome.status, TaskStatus::Failed);
        let lines = sink.visible_lines();
        assert!(
            lines.iter().any(|l| l.contains("bad-task") && l.contains("failed")),
            "no failure footer in {lines:?}"
        );
        assert!(
            lines.iter().any(|l| l.contains("fine-task") && l.contains("succeeded")),
            "a sibling's success footer was lost in {lines:?}"
        );
    }

    #[test]
    fn concurrent_siblings_never_split_one_frame_group() {
        let sink = Arc::new(FrameSink::default());
        let dir = TempDir::new().unwrap();
        let names = ["one", "two", "three", "four", "five", "six"];
        let source = one_step_source(dir.path(), group_step(&names, "parallel"));
        let fixture = Fixture::build(dir, &source).unwrap();
        let recorder = Recorder::default();
        // Inverted delays: completion order is the reverse of declaration order,
        // so a frame group interleaving with a sibling would be visible.
        let shell = names.iter().enumerate().fold(
            FakeTaskShell::default(),
            |shell, (index, command)| {
                shell
                    .stdout_for(command, command)
                    .delay(command, (names.len() - index) as u64 * 10)
            },
        );
        let runtime = Arc::new(RuntimeState::new());
        let mut wiring = Wiring::new(&recorder, &shell).with_stream(Arc::clone(&sink) as Arc<dyn TaskStreamSink>);
        wiring.runtime = Some(&runtime);

        fixture.execute(&wiring);

        for (_, group) in sink.writes() {
            let visible: Vec<String> = group.iter().map(|line| strip_ansi(line)).collect();
            let owners: std::collections::HashSet<&str> = names
                .iter()
                .filter(|name| visible.iter().any(|line| line.contains(**name)))
                .copied()
                .collect();
            assert!(
                owners.len() <= 1,
                "a frame group mixed tasks {owners:?}: {group:?}"
            );
        }
    }

    #[test]
    fn a_silent_run_renders_no_frames_at_all() {
        let dir = TempDir::new().unwrap();
        let source = one_step_source(dir.path(), group_step(&["task-a", "task-b"], "parallel"));
        let fixture = Fixture::build(dir, &source).unwrap();
        let recorder = Recorder::default();
        let shell = FakeTaskShell::default()
            .stdout_for("task-a", "a")
            .stdout_for("task-b", "b");
        let runtime = Arc::new(RuntimeState::new());
        // No sink is the `--silent` shape.
        let mut wiring = Wiring::new(&recorder, &shell);
        wiring.runtime = Some(&runtime);

        let outcome = fixture.execute(&wiring);

        assert!(outcome.succeeded(), "{}", failure_message(&outcome));
        assert_eq!(
            runtime.outputs_value(),
            json!([["a", "b"]]),
            "silencing the frames must not change the run"
        );
        assert!(
            sink_free(&outcome),
            "silencing must not change what the task captured"
        );
    }

    /// A task's captured stdout is unaffected by whether frames were rendered.
    fn sink_free(outcome: &TaskOutcome) -> bool {
        !outcome.stdout.contains('│')
    }

    /// One framed line's payload, with the bar gutter removed.
    fn body_text(line: &str) -> String {
        let visible = strip_ansi(line);
        visible
            .split_once('│')
            .map_or(visible.clone(), |(_, rest)| rest.to_string())
            .trim()
            .to_string()
    }

    #[test]
    fn a_members_body_output_lands_on_the_data_channel_not_the_status_one() {
        let sink = Arc::new(FrameSink::default());
        let outcome = run(&["alpha", "bravo"], "parallel", &sink);

        assert!(outcome.succeeded(), "{}", failure_message(&outcome));
        let data = sink.visible_channel_lines(Channel::Data);
        let status = sink.visible_channel_lines(Channel::Status);
        for name in ["alpha", "bravo"] {
            assert!(
                data.iter().any(|line| body_text(line) == name),
                "task `{name}` produced no framed body line: {data:?}"
            );
        }
        // The status channel carries only the brackets, never the payload.
        assert!(
            status.iter().all(|line| line.contains('▶')
                || line.contains("succeeded")
                || line.contains("failed")
                || line.contains("interrupted")),
            "task data leaked onto the status channel: {status:?}"
        );
    }

    #[test]
    fn every_body_line_carries_its_own_tasks_bar() {
        let sink = Arc::new(FrameSink::default());
        // Forced color, not `default`: the prefixes compared below are the SGR
        // runs ahead of the bar. On a colorless host every prefix collapses to a
        // bare `│` and the equality holds vacuously, so the case would silently
        // stop testing anything.
        run_on(
            &["alpha", "bravo", "charlie"],
            "parallel",
            &sink,
            Terminal::new_forced(),
        );

        // The bar prefix each body line carries must be the same prefix that
        // task's header carried — that is the whole attribution contract.
        let prefix = |line: &String| line[..line.find('│').map_or(0, |i| i + '│'.len_utf8())].to_string();
        for (channel, frames) in sink.writes() {
            if channel != Channel::Data {
                continue;
            }
            for frame in &frames {
                let name = body_text(frame);
                let header = sink
                    .channel_lines(Channel::Status)
                    .into_iter()
                    .find(|line| strip_ansi(line).contains(&format!("▶ {name}")))
                    .unwrap_or_else(|| panic!("no header for body line {frame:?}"));
                assert_eq!(
                    prefix(frame),
                    prefix(&header),
                    "body line {frame:?} does not carry its task's bar"
                );
            }
        }
    }

    #[test]
    fn a_serial_members_body_shares_the_invisible_bar_geometry() {
        let sink = Arc::new(FrameSink::default());
        run(&["alpha", "bravo"], "serial", &sink);

        let data = sink.visible_channel_lines(Channel::Data);
        assert!(
            data.iter().all(|line| !line.contains('│')),
            "serial body output drew a visible bar: {data:?}"
        );
        // Same left edge as a parallel body line: two columns of gutter.
        for line in &data {
            assert!(
                line.starts_with("  ") && !line.starts_with("   "),
                "serial body output shifted its left edge: {line:?}"
            );
        }
    }

    #[test]
    fn partial_output_from_a_failing_task_is_still_framed() {
        let dir = TempDir::new().unwrap();
        let source = one_step_source(dir.path(), group_step(&["bad-task"], "parallel"));
        let fixture = Fixture::build(dir, &source).unwrap();
        let recorder = Recorder::default();
        let shell = FakeTaskShell::default()
            .stdout_for("bad-task", "partial work")
            .failing("bad-task");
        let runtime = Arc::new(RuntimeState::new());
        let sink = Arc::new(FrameSink::default());
        let mut wiring = Wiring::new(&recorder, &shell).with_stream(Arc::clone(&sink) as Arc<dyn TaskStreamSink>);
        wiring.runtime = Some(&runtime);

        let outcome = fixture.execute(&wiring);

        assert_eq!(outcome.status, TaskStatus::Failed);
        // What the work managed to produce before failing is exactly what a
        // reader needs; it is framed even though it never reaches `outputs`.
        let data = sink.visible_channel_lines(Channel::Data);
        assert!(
            data.iter().any(|line| line.contains("partial work")),
            "a failed task's partial output was never framed: {data:?}"
        );
    }

    #[test]
    fn framing_never_reaches_the_captured_payload() {
        let sink = Arc::new(FrameSink::default());
        let outcome = run(&["alpha", "bravo"], "parallel", &sink);

        // The bar is display decoration; `outputs` is the undecorated stdout the
        // frames were rendered from (spec → *Output capture boundary*).
        assert!(
            !sink.visible_channel_lines(Channel::Data).is_empty(),
            "nothing was framed, so this proves nothing"
        );
        assert!(sink_free(&outcome), "framing leaked into {:?}", outcome.stdout);
    }
}

/// Everything visible after removing SGR/CSI sequences.
fn strip_ansi(text: &str) -> String {
    let mut out = String::new();
    let mut chars = text.chars();
    while let Some(ch) = chars.next() {
        if ch != '\u{1b}' {
            out.push(ch);
            continue;
        }
        for next in chars.by_ref() {
            if ('\u{40}'..='\u{7e}').contains(&next) && next != '[' {
                break;
            }
        }
    }
    out
}
