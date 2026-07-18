//! Atomic task execution: one outcome contract for every task variant.
//!
//! A [`PreflightTask`] describes *what could run*; [`TaskExecution`] runs it.
//! Every variant — `prompt`, `shell`, `side_effect` — passes through the same
//! three stages and reports the same [`TaskOutcome`], so a scheduler (serial
//! steps in phase 8, groups in phases 9–10) never learns what kind of work it
//! just ran.
//!
//! ## Stage contract
//!
//! `setup` → primary → `teardown`, with these ratified rules (spec → *Task
//! Resolution and Lifecycle Semantics*):
//!
//! - a failed `setup` skips the primary action, but **not** `teardown`
//! - `teardown` runs exactly once whenever `setup` has *started*, including
//!   after a failed or interrupted primary, and reads the current `err`
//! - a failed `teardown` converts an otherwise successful task to failure; when
//!   the primary already failed, its error stays primary and the teardown error
//!   attaches as a secondary diagnostic
//! - the output is appended only after `teardown` completes, so no later work
//!   observes an output from a task whose teardown failed it
//!
//! A stack shape that will not parse fails the task *before* setup starts, so
//! nothing runs and no teardown is owed.
//!
//! ## Seams
//!
//! Composing and launching a `prompt:` document is the wrapper's job, so it
//! enters through [`PromptTaskRunner`]; shell commands enter through
//! [`TaskShellRunner`]. Both are injected, which is what lets the whole stage
//! contract be tested without a provider or a process.

mod shell;

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use darkmatter::markdown::MarkdownError;
use darkmatter::markdown::compose::EffectiveState;
use darkmatter::markdown::compose::expression::ResolutionContext;
use darkmatter::markdown::compose::subtree::SubtreeCompose;
use serde_json::{Map, Value};

use super::super::error::CompositionError;
use super::super::lifecycle::actions::{
    LifecycleAction, LifecycleActionKind, LifecycleStackItem, is_known_side_effect,
};
use super::super::lifecycle::context::LifecycleErrorInfo;
use super::super::lifecycle::executor::StackExecutionContext;
use super::super::lifecycle::{LifecycleSignal, parse_single_action, parse_task_action_stack};
use super::super::runtime_state::{RuntimeState, layered_set_overrides, trim_transport_newline};
use super::model::RuntimeMutation;
use super::preflight::{PreflightAction, PreflightGraph, PreflightTask};
use super::reserved;
use crate::harness::parse_timeout;

pub use shell::{
    DEFAULT_COMMAND_TIMEOUT, ShellCommandOutput, SystemTaskShell, TaskShellRunner,
};

/// Which stage of a task produced a diagnostic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskStage {
    /// The `setup:` action stack.
    Setup,
    /// The task's executable field.
    Primary,
    /// The `teardown:` action stack.
    Teardown,
}

impl TaskStage {
    /// The authoring key this stage corresponds to.
    pub fn key(self) -> &'static str {
        match self {
            TaskStage::Setup => "setup",
            TaskStage::Primary => "primary",
            TaskStage::Teardown => "teardown",
        }
    }
}

/// How a task finished.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskStatus {
    /// Every stage that ran succeeded.
    Succeeded,
    /// A stage failed. The primary diagnostic names which.
    Failed,
    /// Ctrl+C was observed at a stage boundary, or the primary action reported
    /// an interruption. Distinct from failure: no work is owed a retry and the
    /// sequence exits `130`.
    Interrupted,
}

/// One stage failure, with the diagnostic snapshot the `err` global projects.
#[derive(Debug, Clone, PartialEq)]
pub struct TaskDiagnostic {
    /// Where it happened.
    pub stage: TaskStage,
    /// The projected error, shared with `err.*` rendering and machine output.
    pub info: LifecycleErrorInfo,
}

impl TaskDiagnostic {
    /// Build from any typed composition error.
    fn from_composition(stage: TaskStage, error: &CompositionError) -> Self {
        Self {
            stage,
            info: LifecycleErrorInfo::from_composition_error(error),
        }
    }

    /// The concise message, as `err.msg` reads it.
    pub fn message(&self) -> &str {
        &self.info.msg
    }
}

/// The structured result of running one task.
#[derive(Debug, Clone, PartialEq)]
pub struct TaskOutcome {
    /// Success, failure, or interruption.
    pub status: TaskStatus,
    /// The task's captured stdout — provider final text, concatenated command
    /// stdout, or a side effect's textual return. Empty for successful work
    /// that produced none.
    ///
    /// One trailing transport newline is removed per producing command, so a
    /// multi-command task reads as its results one per line and a blank line
    /// the command actually wrote survives.
    pub stdout: String,
    /// `true` when this outcome's `stdout` was committed to `outputs`.
    pub output_committed: bool,
    /// Runtime state written by this task's stages, in first-write order.
    pub mutations: Vec<RuntimeMutation>,
    /// Wall-clock time across all three stages.
    pub duration: Duration,
    /// The failure that decided the outcome, if any.
    pub error: Option<TaskDiagnostic>,
    /// Failures that did not decide the outcome — a teardown error behind a
    /// primary error, most often.
    pub secondary_errors: Vec<TaskDiagnostic>,
}

impl TaskOutcome {
    /// Whether the task succeeded.
    pub fn succeeded(&self) -> bool {
        self.status == TaskStatus::Succeeded
    }
}

/// What a `prompt:` task asks its runner to do.
///
/// The executor resolves everything statically knowable — which document, which
/// composition mode, the fully layered overrides — so the runner's only job is
/// to compose and launch.
#[derive(Debug, Clone)]
pub struct PromptTaskRequest {
    /// The canonical document path resolved at preflight.
    pub path: std::path::PathBuf,
    /// The reference as authored, for diagnostics.
    pub reference: String,
    /// `true` when the document's own frontmatter carries a string `prompt:`,
    /// making this an inline-compose run that rewrites the document's body.
    pub inline_compose: bool,
    /// The fully layered `set_overrides` object: task `params` < sequence user
    /// setters < accumulated runtime mutations < the reserved overlay.
    pub set_overrides: Value,
    /// The evaluated `params` alone, for diagnostics and reporting.
    pub params: Map<String, Value>,
    /// Effective operation after group defaults and task overrides.
    pub operation: Option<String>,
    /// Effective flow after group defaults and task overrides.
    pub flow: Option<String>,
}

/// What a `prompt:` task's runner reports back.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PromptRunOutcome {
    /// The provider's final assistant text, undecorated.
    ///
    /// The runner must **not** commit this to `outputs`: the task executor owns
    /// output timing, because only it knows whether teardown converted the task
    /// to failure.
    pub stdout: String,
    /// The provider's exit code. Non-zero fails the task.
    pub exit_code: i32,
    /// `true` when the run ended because the user interrupted it.
    pub interrupted: bool,
}

/// Composes and launches a `prompt:` task's document.
///
/// The wrapper implements this; the library never owns provider launch.
pub trait PromptTaskRunner {
    /// Run one prompt task.
    ///
    /// ## Errors
    ///
    /// Returns a typed [`CompositionError`] when the document could not be
    /// composed or launched at all. A launched provider that failed reports its
    /// code through [`PromptRunOutcome::exit_code`].
    fn run(&self, request: &PromptTaskRequest) -> Result<PromptRunOutcome, CompositionError>;
}

/// A [`PromptTaskRunner`] that refuses every request.
///
/// The default for callers that execute only `shell`/`side_effect` work, so a
/// prompt task reaching them is a typed refusal rather than a panic.
#[derive(Debug, Default, Clone, Copy)]
pub struct UnavailablePromptRunner;

impl PromptTaskRunner for UnavailablePromptRunner {
    fn run(&self, request: &PromptTaskRequest) -> Result<PromptRunOutcome, CompositionError> {
        Err(CompositionError::SequenceTaskUnsupported {
            task: request.reference.clone(),
            construct: "a `prompt:` task".to_string(),
            detail: "this caller executes no prompt documents".to_string(),
        })
    }
}

/// Everything one task needs to run.
pub struct TaskExecution<'a> {
    /// The immutable preflight description of the work.
    pub task: &'a PreflightTask,
    /// The graph the task was discovered in, consulted for the composition mode
    /// of a `prompt:` document.
    pub graph: &'a PreflightGraph,
    /// The lifecycle machinery, already carrying this task's effective
    /// frontmatter, runtime cell, and emitter routes.
    pub stack: &'a StackExecutionContext<'a>,
    /// The state `params` and `timeout` evaluate against.
    pub state: &'a EffectiveState,
    /// The invocation runtime cell. `None` runs the task without accumulating
    /// mutations or outputs.
    pub runtime: Option<&'a RuntimeState>,
    /// Sequence user setters (`--set` / `key=value`), which outrank `params`.
    pub user_setters: Option<&'a Value>,
    /// The reserved per-step overlay, which outranks everything.
    pub overlay: Option<&'a Value>,
    /// Runner for `shell:` commands.
    pub shell: &'a dyn TaskShellRunner,
    /// Runner for `prompt:` documents.
    pub prompt: &'a dyn PromptTaskRunner,
    /// Cooperative interrupt flag, checked at every stage boundary.
    pub interrupt: Option<&'a AtomicBool>,
}

impl TaskExecution<'_> {
    /// Run the task through setup, its primary action, and teardown.
    ///
    /// Never returns `Err`: every failure is part of the task's reported
    /// outcome, because a scheduler must decide continuation from `fail_fast`
    /// rather than from an error propagating out of one task.
    pub fn run(&self) -> TaskOutcome {
        let started = Instant::now();
        let mutations_before = self.mutation_snapshot();

        let outcome = self.run_stages();
        let (status, stdout, error, secondary_errors) = outcome;

        let mut output_committed = false;
        if status == TaskStatus::Succeeded
            && let Some(runtime) = self.runtime
        {
            // The transport newline was already removed where the text was
            // produced — per command for a shell task, once for a provider —
            // so this appends the entry as-is rather than trimming again.
            runtime.append_output_entry(Value::String(stdout.clone()));
            output_committed = true;
        }

        TaskOutcome {
            status,
            stdout,
            output_committed,
            mutations: self.mutation_delta(&mutations_before),
            duration: started.elapsed(),
            error,
            secondary_errors,
        }
    }

    /// The stage machine, without output commit or timing bookkeeping.
    fn run_stages(
        &self,
    ) -> (TaskStatus, String, Option<TaskDiagnostic>, Vec<TaskDiagnostic>) {
        // Parsing precedes setup deliberately: a stack that will not parse is an
        // authoring error, and running teardown for a task that never started is
        // worse than not running it.
        let stacks = match self.parse_stacks() {
            Ok(stacks) => stacks,
            Err(error) => {
                return (
                    TaskStatus::Failed,
                    String::new(),
                    Some(TaskDiagnostic::from_composition(TaskStage::Setup, &error)),
                    Vec::new(),
                );
            }
        };

        if self.interrupted() {
            return (TaskStatus::Interrupted, String::new(), None, Vec::new());
        }

        let mut primary_error: Option<TaskDiagnostic> = None;
        let mut secondary: Vec<TaskDiagnostic> = Vec::new();
        let mut status = TaskStatus::Succeeded;
        let mut stdout = String::new();

        if let Some(items) = &stacks.setup
            && let Some(diagnostic) = self.run_stack(TaskStage::Setup, items, None)
        {
            primary_error = Some(diagnostic);
            status = TaskStatus::Failed;
        }

        if primary_error.is_none() {
            if self.interrupted() {
                status = TaskStatus::Interrupted;
            } else {
                let primary = self.run_primary();
                stdout = primary.stdout;
                match primary.result {
                    PrimaryResult::Succeeded => {}
                    PrimaryResult::Failed(diagnostic) => {
                        primary_error = Some(diagnostic);
                        status = TaskStatus::Failed;
                    }
                    PrimaryResult::Interrupted => status = TaskStatus::Interrupted,
                }
            }
        }

        // Teardown is owed from the moment setup started, which is right here:
        // parse succeeded and the interrupt check passed.
        if let Some(items) = &stacks.teardown {
            let err = primary_error.as_ref().map(|d| d.info.clone());
            if let Some(diagnostic) = self.run_stack(TaskStage::Teardown, items, err.as_ref()) {
                match primary_error {
                    // A teardown failure behind a primary failure is secondary:
                    // the primary error is what the author must fix first.
                    Some(_) => secondary.push(diagnostic),
                    None => {
                        primary_error = Some(diagnostic);
                        status = TaskStatus::Failed;
                    }
                }
            }
        }

        (status, stdout, primary_error, secondary)
    }

    /// Run one action stack, returning its failure when it had one.
    fn run_stack(
        &self,
        stage: TaskStage,
        items: &[LifecycleStackItem],
        err: Option<&LifecycleErrorInfo>,
    ) -> Option<TaskDiagnostic> {
        let with_err;
        let context = match err {
            Some(info) => {
                with_err = self.stack.with_error(info);
                &with_err
            }
            None => self.stack,
        };
        let outcome = context.execute_action_stack(items);
        outcome
            .evaluation_error
            .or(outcome.action_error)
            .map(|info| TaskDiagnostic { stage, info })
    }

    /// Run the task's executable field.
    fn run_primary(&self) -> PrimaryOutcome {
        match &self.task.action {
            PreflightAction::Prompt { path, reference } => self.run_prompt(path, reference),
            PreflightAction::Shell { commands } => self.run_shell(commands),
            PreflightAction::SideEffect { action } => self.run_side_effect(action),
            PreflightAction::Group(group) => {
                PrimaryOutcome::failed(TaskDiagnostic::from_composition(
                    TaskStage::Primary,
                    &CompositionError::SequenceTaskUnsupported {
                        task: self.label(),
                        construct: format!("group `{}`", group.name),
                        detail: "a group is scheduled by the sequence orchestrator, \
                                 not executed as an atomic task"
                            .to_string(),
                    },
                ))
            }
        }
    }

    /// Compose and launch a `prompt:` document through the injected runner.
    fn run_prompt(&self, path: &std::path::Path, reference: &str) -> PrimaryOutcome {
        let params = match self.evaluate_params() {
            Ok(params) => params,
            Err(error) => {
                return PrimaryOutcome::failed(TaskDiagnostic::from_composition(
                    TaskStage::Primary,
                    &error,
                ));
            }
        };

        let request = PromptTaskRequest {
            path: path.to_path_buf(),
            reference: reference.to_string(),
            // The referenced document's own frontmatter decides the mode: a
            // string `prompt:` there makes composing it an inline-compose run
            // that rewrites its body.
            inline_compose: self
                .graph
                .prompt_document(path)
                .is_some_and(|document| document.inline_compose),
            set_overrides: self.layered_overrides(&params),
            params,
            operation: self.task.operation.clone(),
            flow: self.task.flow.clone(),
        };

        match self.prompt.run(&request) {
            Ok(run) if run.interrupted => {
                PrimaryOutcome::interrupted(trim_transport_newline(&run.stdout).to_string())
            }
            Ok(run) if run.exit_code == 0 => {
                PrimaryOutcome::succeeded(trim_transport_newline(&run.stdout).to_string())
            }
            Ok(run) => PrimaryOutcome::failed(TaskDiagnostic::from_composition(
                TaskStage::Primary,
                &CompositionError::SequenceTaskPromptFailed {
                    task: self.label(),
                    path: path.to_path_buf(),
                    code: run.exit_code,
                },
            )),
            Err(error) => PrimaryOutcome::failed(TaskDiagnostic::from_composition(
                TaskStage::Primary,
                &error,
            )),
        }
    }

    /// Run every approved command in declaration order.
    ///
    /// The commands are preflight's resolved bytes and are passed through
    /// untouched — approved *is* executed. Each command's stdout contributes one
    /// line-terminated block to the task's output, so a two-command task reads
    /// as its two results in declaration order.
    fn run_shell(&self, commands: &[String]) -> PrimaryOutcome {
        let timeout = match self.command_timeout() {
            Ok(timeout) => timeout,
            Err(error) => {
                return PrimaryOutcome::failed(TaskDiagnostic::from_composition(
                    TaskStage::Primary,
                    &error,
                ));
            }
        };

        let mut collected: Vec<String> = Vec::with_capacity(commands.len());
        for command in commands {
            if self.interrupted() {
                return PrimaryOutcome::interrupted(collected.join("\n"));
            }
            let output = match self.shell.run(command, timeout) {
                Ok(output) => output,
                Err(source) => {
                    return PrimaryOutcome::failed(TaskDiagnostic::from_composition(
                        TaskStage::Primary,
                        &CompositionError::SequenceTaskShellSpawn {
                            task: self.label(),
                            command: command.clone(),
                            source,
                        },
                    ));
                }
            };
            collected.push(trim_transport_newline(&output.stdout).to_string());

            if output.timed_out {
                return PrimaryOutcome::failed_with(
                    collected.join("\n"),
                    TaskDiagnostic::from_composition(
                    TaskStage::Primary,
                        &CompositionError::SequenceTaskShellTimeout {
                            task: self.label(),
                            command: command.clone(),
                            seconds: timeout.as_secs_f64(),
                        },
                    ),
                );
            }
            if output.exit_code != 0 {
                return PrimaryOutcome::failed_with(
                    collected.join("\n"),
                    TaskDiagnostic::from_composition(
                    TaskStage::Primary,
                        &CompositionError::SequenceTaskShellExit {
                            task: self.label(),
                            command: command.clone(),
                            code: output.exit_code,
                        },
                    ),
                );
            }
        }
        PrimaryOutcome::succeeded(collected.join("\n"))
    }

    /// Dispatch a `side_effect:` action and capture its textual return.
    fn run_side_effect(&self, action: &Value) -> PrimaryOutcome {
        let parsed = match parse_single_action(
            LifecycleSignal::Start,
            action,
            &self.task.origin_path,
            "side_effect",
        ) {
            Ok(parsed) => parsed,
            Err(error) => {
                return PrimaryOutcome::failed(TaskDiagnostic::from_composition(
                    TaskStage::Primary,
                    &error,
                ));
            }
        };

        // The action grammar also admits communication and flow-control verbs;
        // only a side effect is executable work, so the mismatch is named here
        // rather than reported as a generic dispatch failure.
        if !is_side_effect_action(&parsed) {
            return PrimaryOutcome::failed(TaskDiagnostic::from_composition(
                TaskStage::Primary,
                &CompositionError::SequenceTaskInvalidSideEffect {
                    task: self.label(),
                    problem: describe_action(&parsed),
                },
            ));
        }

        match self.stack.dispatch_task_side_effect(&parsed) {
            // A side effect that returns nothing contributes the empty string,
            // keeping one `outputs` entry per executed task.
            Ok(Value::Null) => PrimaryOutcome::succeeded(String::new()),
            Ok(Value::String(text)) => PrimaryOutcome::succeeded(text),
            Ok(other) => PrimaryOutcome::succeeded(other.to_string()),
            Err(info) => PrimaryOutcome::failed(TaskDiagnostic {
                stage: TaskStage::Primary,
                info,
            }),
        }
    }

    /// Parse both action stacks up front.
    fn parse_stacks(&self) -> Result<ParsedStacks, CompositionError> {
        let parse = |raw: Option<&Value>, signal, property| match raw {
            None | Some(Value::Null) => Ok(None),
            Some(value) => {
                parse_task_action_stack(signal, value, &self.task.origin_path, property)
                    .map(|items| (!items.is_empty()).then_some(items))
            }
        };
        Ok(ParsedStacks {
            setup: parse(self.task.setup.as_ref(), LifecycleSignal::Start, "setup")?,
            teardown: parse(
                self.task.teardown.as_ref(),
                LifecycleSignal::Finalize,
                "teardown",
            )?,
        })
    }

    /// Evaluate `params` just in time against the caller's effective state.
    ///
    /// Whole-value typing survives: `topic: "{{ state.index }}"` binds the
    /// number, not its rendering.
    fn evaluate_params(&self) -> Result<Map<String, Value>, CompositionError> {
        let mut evaluated = Map::new();
        for (key, value) in &self.task.params {
            if reserved::is_reserved_state_key(key) {
                return Err(CompositionError::SequenceTaskParamReserved {
                    task: self.label(),
                    key: key.clone(),
                });
            }
            evaluated.insert(key.clone(), self.resolve_value(value, &format!("params.{key}"))?);
        }
        Ok(evaluated)
    }

    /// Fold the four layers into the overrides a prompt task composes against.
    ///
    /// Precedence, lowest first: task `params`, sequence user setters,
    /// accumulated runtime mutations, the reserved overlay (spec → *Task
    /// Resolution and Lifecycle Semantics*).
    fn layered_overrides(&self, params: &Map<String, Value>) -> Value {
        let mut base = params.clone();
        if let Some(Value::Object(setters)) = self.user_setters {
            for (key, value) in setters {
                base.insert(key.clone(), value.clone());
            }
        }
        let snapshot = self.runtime.map(RuntimeState::snapshot);
        layered_set_overrides(
            Some(&Value::Object(base)),
            snapshot.as_ref(),
            self.overlay,
        )
    }

    /// The per-command budget: the authored `timeout:`, else 30 seconds.
    fn command_timeout(&self) -> Result<Duration, CompositionError> {
        let Some(raw) = &self.task.timeout else {
            return Ok(DEFAULT_COMMAND_TIMEOUT);
        };
        let resolved = self.resolve_value(raw, "timeout")?;
        let rendered = match &resolved {
            Value::String(text) => text.clone(),
            other => other.to_string(),
        };
        parse_timeout(&rendered, &self.task.origin_path).map_err(|source| {
            CompositionError::SequenceTaskTimeoutInvalid {
                context: Box::new(super::super::error::TaskTimeoutContext {
                    task: self.label(),
                    raw: rendered,
                }),
                source,
            }
        })
    }

    /// Resolve one authored value's `{{ … }}` spans against the effective state.
    fn resolve_value(&self, value: &Value, field: &str) -> Result<Value, CompositionError> {
        if !contains_interpolation(value) {
            return Ok(value.clone());
        }
        let resolution = ResolutionContext::new(self.task.origin_dir.clone());
        SubtreeCompose::new(value, self.state)
            .with_resolution_context(resolution)
            .strict()
            .compose()
            .map_err(|error: MarkdownError| CompositionError::SequenceTaskValueResolution {
                task: self.label(),
                field: field.to_string(),
                message: error.to_string(),
                source: Box::new(error),
            })
    }

    /// The runtime mutation map as it stands right now.
    fn mutation_snapshot(&self) -> Map<String, Value> {
        self.runtime
            .map(|runtime| runtime.snapshot().mutations.into_iter().collect())
            .unwrap_or_default()
    }

    /// Everything this task wrote to the runtime layer, in first-write order.
    fn mutation_delta(&self, before: &Map<String, Value>) -> Vec<RuntimeMutation> {
        let Some(runtime) = self.runtime else {
            return Vec::new();
        };
        runtime
            .snapshot()
            .mutations
            .into_iter()
            .filter(|(key, value)| before.get(key) != Some(value))
            .map(|(key, value)| RuntimeMutation {
                prior: before.get(&key).cloned().unwrap_or(Value::Null),
                key,
                value,
            })
            .collect()
    }

    fn interrupted(&self) -> bool {
        self.interrupt
            .is_some_and(|flag| flag.load(Ordering::SeqCst))
    }

    /// The task's authored `name:`, falling back to its generated label.
    fn label(&self) -> String {
        self.task
            .name
            .clone()
            .unwrap_or_else(|| self.task.label.clone())
    }
}

/// Both parsed action stacks; `None` for an absent or empty one.
struct ParsedStacks {
    setup: Option<Vec<LifecycleStackItem>>,
    teardown: Option<Vec<LifecycleStackItem>>,
}

/// What the primary action did, plus whatever it produced before finishing.
///
/// Partial output survives a failure or interruption: it is never *committed*
/// to `outputs`, but a scheduler reporting the task — and a parallel group
/// keeping a slot per sibling — needs the text the work managed to produce.
struct PrimaryOutcome {
    stdout: String,
    result: PrimaryResult,
}

impl PrimaryOutcome {
    fn succeeded(stdout: String) -> Self {
        Self {
            stdout,
            result: PrimaryResult::Succeeded,
        }
    }

    fn failed(diagnostic: TaskDiagnostic) -> Self {
        Self::failed_with(String::new(), diagnostic)
    }

    fn failed_with(stdout: String, diagnostic: TaskDiagnostic) -> Self {
        Self {
            stdout,
            result: PrimaryResult::Failed(diagnostic),
        }
    }

    fn interrupted(stdout: String) -> Self {
        Self {
            stdout,
            result: PrimaryResult::Interrupted,
        }
    }
}

/// The primary action's terminal state.
enum PrimaryResult {
    Succeeded,
    Failed(TaskDiagnostic),
    Interrupted,
}

/// Whether a parsed action is executable side-effect work.
///
/// A positional side effect can parse as either kind: `set:` reaches
/// [`LifecycleActionKind::SideEffect`], while a verb the expression catalog also
/// knows arrives as an expression function and is resolved by name.
fn is_side_effect_action(action: &LifecycleAction) -> bool {
    match &action.kind {
        LifecycleActionKind::SideEffect(_) => true,
        LifecycleActionKind::ExpressionFunction(func) => is_known_side_effect(&func.function),
        _ => false,
    }
}

/// Name a non-side-effect action for the rejection message.
fn describe_action(action: &LifecycleAction) -> String {
    match &action.kind {
        LifecycleActionKind::LifecycleControl(control) => {
            format!("`{}` is a flow-control action", control.verb())
        }
        LifecycleActionKind::Communication(comm) => {
            format!("`{}` is a communication action", comm.channel.verb())
        }
        LifecycleActionKind::Shell(_) => {
            "`shell` is a shell action; use a `shell:` task instead".to_string()
        }
        LifecycleActionKind::ExpressionFunction(func) => {
            format!("`{}` is a read-only expression function", func.function)
        }
        LifecycleActionKind::SideEffect(effect) => format!("`{}` is a side effect", effect.verb),
    }
}

/// Whether any string anywhere in `value` carries an interpolation span.
fn contains_interpolation(value: &Value) -> bool {
    match value {
        Value::String(text) => text.contains("{{"),
        Value::Array(items) => items.iter().any(contains_interpolation),
        Value::Object(map) => map.values().any(contains_interpolation),
        _ => false,
    }
}

#[cfg(test)]
mod tests;
