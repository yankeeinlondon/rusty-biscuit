//! Claudine's host adapter for Darkmatter's lifecycle stack executor.
//!
//! The engine — ordered evaluation, `set` snapshot semantics, deferred
//! `{{ }}` spans, control directives, `no_error`, and error routing — is
//! [`darkmatter::lifecycle::executor`]. This module keeps what is Claudine's:
//! [`StackExecutionContext`], the per-event configuration every driver builds,
//! which answers the executor's [`LifecycleHost`] questions from Claudine's
//! binding catalog, runtime state, loop slot, `prep` runner, and diagnostic
//! catalog, and delivers communication through [`ClaudineLifecycleEffects`];
//! and the process-backed [`ShellRunner`]s, which start children through
//! Claudine's child-environment and owned-process-tree code.

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use biscuit_terminal::terminal::Terminal;
use darkmatter::effects::EffectEngine;
use darkmatter::lifecycle::executor::{
    LifecycleHost, LifecycleRunFailure, PrepError, PrepRequest, StackExecutor, StackInputs,
};
use darkmatter::lifecycle::run_guard::{GuardedEvent, GuardedEventStack};
use darkmatter::markdown::compose::expression::{BindingView, ResolutionContext, RuntimeBinding};
use darkmatter::markdown::compose::shell_expansion::template::FilledChain;
use darkmatter::markdown::compose::shell_expansion::{ShellChildEnvironment, ShellRuleSet};
use darkmatter::markdown::compose::{ComposeContext, ContextBuildError, CurrentAuthority};
use indexmap::IndexMap;
use serde_json::{Map, Value};

use super::super::error::{CompositionError, TaskGuardFailure};
use super::context::{LifecycleErrorInfo, LifecycleTiming};
use super::bindings::{LifecycleScope, LifecycleValues, runtime_bindings_in_loop};
use super::effects::ClaudineLifecycleEffects;
use super::{LifecycleConfig, LifecycleEmitter, LifecycleSignal};
use crate::composition::LifecycleErrorInfoExt;
use crate::composition::looping::LoopAmbient;
use crate::diagnostics::ClaudineDiagnostics;
use crate::events::GlobalSettings;
use crate::invocation_context::EnvBaseline;
use crate::messaging::RuntimeMessagingSettings;

pub use darkmatter::lifecycle::LifecycleExprError;
pub use darkmatter::lifecycle::executor::{
    DisabledShellRunner, LifecycleEventOutcome, ShellRunError, ShellRunner, ShellValueRequest, StackControl,
    evaluate_operand_in, execute_set_shell_values,
};

/// A poisoned live-frontmatter cell means a task panicked mid-stack; there is
/// no partial document state worth recovering.
const LIVE_POISONED: &str = "live frontmatter mutex poisoned by a panicking task";

/// Production [`ShellRunner`] that runs commands through the system shell,
/// inheriting this process's environment.
#[derive(Debug, Default, Clone, Copy)]
pub struct SystemShellRunner;

impl ShellRunner for SystemShellRunner {
    fn run(&self, command: &str) -> Result<i32, ShellRunError> {
        run_system_shell(command, None)
    }

    fn run_structured(&self, chain: &FilledChain) -> Result<i32, ShellRunError> {
        super::structured_shell::run_filled_chain(chain, None)
    }

    fn for_child_environment(&self, environment: &ShellChildEnvironment) -> Option<Box<dyn ShellRunner>> {
        Some(Box::new(DocumentShellRunner::new(EnvBaseline::from_entries(environment.iter()))))
    }
}

/// [`SystemShellRunner`] for one document: every child, including a `set`
/// value's `$( … )`, starts from exactly that document's environment
/// ([`crate::invocation_context::InvocationContext::env_baseline`]) rather than
/// from this process's.
#[derive(Debug, Clone)]
pub struct DocumentShellRunner {
    environment: EnvBaseline,
}

impl DocumentShellRunner {
    /// A runner whose children start from exactly `environment`.
    pub fn new(environment: EnvBaseline) -> Self {
        Self { environment }
    }

    /// The environment every child starts from.
    pub fn environment(&self) -> &EnvBaseline {
        &self.environment
    }
}

impl ShellRunner for DocumentShellRunner {
    fn run(&self, command: &str) -> Result<i32, ShellRunError> {
        run_system_shell(command, Some(&self.environment))
    }

    fn run_structured(&self, chain: &FilledChain) -> Result<i32, ShellRunError> {
        super::structured_shell::run_filled_chain(chain, Some(&self.environment))
    }

    fn run_values(
        &self,
        request: &ShellValueRequest<'_>,
    ) -> Result<Vec<(String, Value)>, ShellRunError> {
        execute_set_shell_values(request, Some(&self.environment.shell_child_environment()))
    }

    fn for_child_environment(&self, environment: &ShellChildEnvironment) -> Option<Box<dyn ShellRunner>> {
        Some(Box::new(Self::new(EnvBaseline::from_entries(environment.iter()))))
    }
}

fn run_system_shell(command: &str, environment: Option<&EnvBaseline>) -> Result<i32, ShellRunError> {
    let mut cmd = system_shell_command(command, environment).map_err(|error| ShellRunError::Spawn {
        command: command.to_string(),
        source: std::io::Error::other(error),
    })?;
    super::super::sequence::task::run_owned_lifecycle_command(&mut cmd)
        .map_err(|source| ShellRunError::Spawn { command: command.to_string(), source })
}

/// `program` as a child starting from `environment`, or inheriting this
/// process's when there is none.
fn shell_program(
    program: &str,
    environment: Option<&EnvBaseline>,
) -> Result<std::process::Command, crate::child_environment::ChildEnvironmentError> {
    match environment {
        Some(environment) => {
            crate::child_environment::command_with_environment(program, &environment.to_child_map())
        }
        None => crate::child_environment::command(program),
    }
}

/// Build the platform `Command` that runs `command` through the system shell.
#[cfg(windows)]
fn system_shell_command(
    command: &str,
    environment: Option<&EnvBaseline>,
) -> Result<std::process::Command, crate::child_environment::ChildEnvironmentError> {
    use std::os::windows::process::CommandExt;

    let mut cmd = shell_program("cmd", environment)?;
    // `cmd.exe` owns the command-tail grammar. Passing it through the Windows
    // argv encoder changes nested quotes before the shell can parse them.
    cmd.arg("/D").arg("/C").raw_arg(command);
    Ok(cmd)
}

/// Build the platform `Command` that runs `command` through the system shell.
#[cfg(not(windows))]
fn system_shell_command(
    command: &str,
    environment: Option<&EnvBaseline>,
) -> Result<std::process::Command, crate::child_environment::ChildEnvironmentError> {
    let mut cmd = shell_program("sh", environment)?;
    cmd.arg("-c").arg(command);
    Ok(cmd)
}

/// Everything Claudine supplies to run one lifecycle event.
///
/// Construct one per event with the active [`LifecycleSignal`], the composed
/// frontmatter, the lifecycle globals (`err`/`timing`), the invocation's
/// `current` refresh authority, and the side-effect / shell / emitter routes.
/// Each run hands Darkmatter's [`StackExecutor`] the neutral inputs, this
/// context as its [`LifecycleHost`], and the emitter configuration as its
/// [`ClaudineLifecycleEffects`].
pub struct StackExecutionContext<'a> {
    /// The event being processed.
    pub signal: LifecycleSignal,
    /// The binding scope expressions evaluate in; `None` is the event
    /// `signal` names. A sequence `side_effect:` task's action sets its own
    /// scope ([`Self::in_scope`]) because it runs in the step's context, not in
    /// a document event.
    pub scope: Option<LifecycleScope>,
    /// Composed frontmatter — the base namespace for expression evaluation.
    pub frontmatter: &'a Map<String, Value>,
    /// Shared cross-event live document frontmatter for the current attempt.
    ///
    /// When `Some`, lifecycle frontmatter side effects (`set_frontmatter` and
    /// friends) that target the document persist into this cell so a *later*
    /// event in the same provider attempt reads the mutated value — satisfying
    /// the spec's "current effective document state at the moment the event
    /// fires" contract across events. When `None`, behavior is exactly as for a
    /// single-event caller: `frontmatter` is the only base state and stack
    /// mutations are visible intra-stack only.
    pub live_frontmatter: Option<&'a std::sync::Mutex<Map<String, Value>>>,
    /// The invocation-local runtime state cell.
    ///
    /// `live_frontmatter` is per-*attempt*; this cell spans the whole
    /// invocation, so a `set` written by one lifecycle event survives loop
    /// rematerialization and (from phase 8) later sequence steps. When `None`,
    /// `set` still validates its key and mutates the intra-stack working state,
    /// but the write is not accumulated beyond this event.
    pub runtime_state: Option<&'a super::super::runtime_state::RuntimeState>,
    /// The `err` global snapshot (only meaningful for error-carrying events).
    pub err: Option<&'a LifecycleErrorInfo>,
    /// The `timing` global snapshot.
    pub timing: Option<&'a LifecycleTiming>,
    /// The invocation's refresh authority for Darkmatter's lazy `current` root.
    ///
    /// `None` fails closed: every `current.<key>` renders `null` and records a
    /// `PartialRuntimeCapture` diagnostic rather than probing the host. Only a
    /// caller holding launch evidence supplies one.
    ///
    /// `current` is observed once per event: a constructor passes
    /// [`CurrentAuthority::memoized`], so every read in the event's stack
    /// agrees, and [`Self::with_signal`] starts a
    /// new memo for another event.
    ///
    /// Owned rather than borrowed because the authority is a cheap handle to
    /// shared invocation state, and every derived context clones it — sharing
    /// the same provider and diagnostic sink.
    pub current: Option<CurrentAuthority>,
    /// The `group` global: a sequence group's variables, in scope only while
    /// that group's tasks run.
    ///
    /// Unlike `err`/`timing`, this is not event-derived — it is a lexical
    /// scope the group scheduler enters and leaves, which is why it arrives as
    /// a borrowed map rather than a snapshot type.
    pub group: Option<&'a Map<String, Value>>,
    /// Complete lexical sequence position; runtime outputs remain live.
    pub seq: Option<&'a Value>,
    /// The `loop` global: the enclosing loop iteration's values, in scope for
    /// the loop gate and every event of one iteration. `None` is known to be
    /// outside any loop, where reading `loop` is unavailable.
    pub loop_values: Option<&'a LoopAmbient>,
    /// Base directory for read-side expression functions and file references.
    pub base_dir: Option<&'a Path>,
    /// Base directory for `ctx.*` capture only (the launch area); when `None`,
    /// falls back to `base_dir`.
    pub ctx_base_dir: Option<&'a Path>,
    /// The single early-binding context snapshot captured once at composition
    /// start. When `Some`, the executor reuses it for `ctx.*`/`env.*` instead
    /// of re-capturing per event (so body and lifecycle cannot diverge); when
    /// `None`, it falls back to a demand-driven capture rooted at
    /// `ctx_base_dir`/`base_dir`.
    pub prepared_context: Option<&'a ComposeContext>,
    /// The request's file-resolution context, used by document-authored file
    /// expressions.
    pub file_resolution_context: &'a biscuit_file::FileResolutionContext,
    /// Darkmatter side-effect engine.
    pub effect_engine: &'a EffectEngine,
    /// Approved-shell runner.
    pub shell_runner: &'a dyn ShellRunner,
    /// Communication emitter (stderr/info/warn/message/notify/say/effect).
    pub emitter: &'a dyn LifecycleEmitter,
    /// Terminal for rendering status lines.
    pub term: &'a Terminal,
    /// Composition source path (for messenger provenance).
    pub source_path: &'a Path,
    /// Repository root, if any.
    pub repo_root: Option<&'a Path>,
    /// Messaging routes.
    pub messaging: &'a RuntimeMessagingSettings,
    /// Global settings (TTS configuration).
    pub settings: &'a GlobalSettings,
}

impl StackExecutionContext<'_> {
    /// Run the event's stack.
    pub fn execute_event(&self, config: &LifecycleConfig) -> LifecycleEventOutcome {
        self.execute_stack_for_signal(config)
    }

    /// Run the stack for this event's signal.
    pub fn execute_stack_for_signal(&self, config: &LifecycleConfig) -> LifecycleEventOutcome {
        self.run(|executor| executor.execute_event(config))
    }

    /// Run one already-parsed action stack — a task's `setup:` or `teardown:`;
    /// see [`StackExecutor::execute_action_stack`].
    pub fn execute_action_stack(&self, nodes: &[super::actions::LifecycleNode]) -> LifecycleEventOutcome {
        self.run(|executor| executor.execute_action_stack(nodes))
    }

    /// Evaluate a task's `when:` guard against this context's frontmatter and
    /// bindings, with the lifecycle `when` truthiness rule.
    ///
    /// The guard is read-only: it runs no stack and dispatches nothing.
    ///
    /// ## Errors
    ///
    /// The parse or evaluation failure. A guard that raises is never read as
    /// false.
    pub fn evaluate_task_guard(&self, condition: &str) -> Result<bool, TaskGuardFailure> {
        let expr = darkmatter::markdown::compose::expression::parse_condition(condition)?;
        self.run(|executor| executor.evaluate_guard(&expr)).map_err(Into::into)
    }

    /// Dispatch one action as a task's `side_effect:` primary and return the
    /// value the effect produced; see
    /// [`StackExecutor::dispatch_task_side_effect`].
    ///
    /// ## Errors
    ///
    /// Returns the error snapshot for an unsuppressed dispatch failure, an
    /// expression-layer raise, or a non-side-effect action.
    pub fn dispatch_task_side_effect(
        &self,
        action: &super::actions::LifecycleAction,
        property: &str,
    ) -> Result<Value, LifecycleErrorInfo> {
        self.run(|executor| executor.dispatch_task_side_effect(action, property))
    }

    /// Return a copy of this context targeting a different lifecycle signal.
    ///
    /// Used by the run guard ([`GuardedEventStack`]) so one constructed
    /// context can service every signal in a run.
    ///
    /// Another signal is another event, and `current` is observed once per
    /// event, so the copy starts a new `current` memo; the same signal keeps
    /// the event's memo.
    pub fn with_signal(&self, signal: LifecycleSignal) -> StackExecutionContext<'_> {
        StackExecutionContext {
            seq: self.seq,
            signal,
            scope: if signal == self.signal { self.scope } else { None },
            frontmatter: self.frontmatter,
            live_frontmatter: self.live_frontmatter,
            runtime_state: self.runtime_state,
            err: self.err,
            timing: self.timing,
            current: match signal == self.signal {
                true => self.current.clone(),
                false => self
                    .current
                    .as_ref()
                    .map(darkmatter::markdown::compose::CurrentAuthority::memoized),
            },
            group: self.group,
            loop_values: self.loop_values,
            base_dir: self.base_dir,
            ctx_base_dir: self.ctx_base_dir,
            prepared_context: self.prepared_context,
            file_resolution_context: self.file_resolution_context,
            effect_engine: self.effect_engine,
            shell_runner: self.shell_runner,
            emitter: self.emitter,
            term: self.term,
            source_path: self.source_path,
            repo_root: self.repo_root,
            messaging: self.messaging,
            settings: self.settings,
        }
    }

    /// Return a copy of this context with the `err` global attached.
    ///
    /// Used when routing an unintentional action error into the `failure`
    /// event so `err.kind`/`err.variant`/`err.msg` are available to the
    /// failure stack.
    pub fn with_error<'a>(
        &'a self,
        err: &'a LifecycleErrorInfo,
    ) -> StackExecutionContext<'a> {
        StackExecutionContext {
            seq: self.seq,
            signal: self.signal,
            scope: self.scope,
            frontmatter: self.frontmatter,
            live_frontmatter: self.live_frontmatter,
            runtime_state: self.runtime_state,
            err: Some(err),
            timing: self.timing,
            current: self.current.clone(),
            group: self.group,
            loop_values: self.loop_values,
            base_dir: self.base_dir,
            ctx_base_dir: self.ctx_base_dir,
            prepared_context: self.prepared_context,
            file_resolution_context: self.file_resolution_context,
            effect_engine: self.effect_engine,
            shell_runner: self.shell_runner,
            emitter: self.emitter,
            term: self.term,
            source_path: self.source_path,
            repo_root: self.repo_root,
            messaging: self.messaging,
            settings: self.settings,
        }
    }

    /// Return a copy of this context with a group's variables in scope.
    ///
    /// The scope lives exactly as long as the returned context: a group's
    /// tasks read `group.*`, and the sequence step after it cannot, because it
    /// runs against the original context.
    pub fn with_group<'a>(
        &'a self,
        variables: &'a Map<String, Value>,
    ) -> StackExecutionContext<'a> {
        StackExecutionContext {
            group: Some(variables),
            ..self.with_signal(self.signal)
        }
    }

    /// Return a copy of this context evaluating in binding `scope`.
    pub fn in_scope(&self, scope: LifecycleScope) -> StackExecutionContext<'_> {
        StackExecutionContext {
            scope: Some(scope),
            ..self.with_signal(self.signal)
        }
    }

    /// The binding scope this context's expressions evaluate in.
    pub fn binding_scope(&self) -> LifecycleScope {
        self.scope.unwrap_or(LifecycleScope::Event(self.signal))
    }

    /// Redirect this context's two mutable cells to private ones.
    ///
    /// A parallel group member runs against a private runtime buffer, so its
    /// lifecycle `set` must land there rather than in the sequence's cell —
    /// otherwise the write is visible to a sibling mid-group and the
    /// deterministic declaration-order merge has nothing left to merge.
    ///
    /// The live document cell has to be redirected in the same breath: `set`
    /// mirrors its write onto the working map that the executor's stack run
    /// seeds from and writes back to that cell, so a shared cell leaks the
    /// value to a sibling through the second channel no matter how private the
    /// runtime buffer is. `None` keeps the snapshot-less shape, where the
    /// immutable `frontmatter` is the only base state and nothing can leak.
    pub fn with_private_cells<'a>(
        &'a self,
        runtime_state: &'a super::super::runtime_state::RuntimeState,
        live_frontmatter: Option<&'a std::sync::Mutex<Map<String, Value>>>,
    ) -> StackExecutionContext<'a> {
        StackExecutionContext {
            runtime_state: Some(runtime_state),
            live_frontmatter,
            ..self.with_signal(self.signal)
        }
    }

    /// Copy the live document cell's current contents, when one is wired.
    ///
    /// The seed for a private cell handed back through
    /// [`Self::with_private_cells`].
    pub fn live_frontmatter_snapshot(&self) -> Option<Map<String, Value>> {
        self.live_frontmatter
            .map(|cell| cell.lock().expect(LIVE_POISONED).clone())
    }

    /// Run `f` against Darkmatter's executor over this context: the neutral
    /// inputs, this context as the host, and the emitter configuration as the
    /// communication service.
    fn run<R>(&self, f: impl FnOnce(&StackExecutor<'_>) -> R) -> R {
        let effects = ClaudineLifecycleEffects::new(
            self.emitter,
            self.settings,
            self.messaging,
            self.term,
            self.source_path,
            self.repo_root,
        );
        f(&StackExecutor::with_services(self.inputs(), &effects, &ClaudineDiagnostics))
    }

    /// The executor's view of this context.
    fn inputs(&self) -> StackInputs<'_> {
        StackInputs {
            signal: self.signal,
            frontmatter: self.frontmatter,
            live_frontmatter: self.live_frontmatter,
            current: self.current.clone(),
            base_dir: self.base_dir,
            ctx_base_dir: self.ctx_base_dir,
            prepared_context: self.prepared_context,
            file_resolution_context: self.file_resolution_context,
            effect_engine: self.effect_engine,
            shell_runner: self.shell_runner,
            source_path: self.source_path,
            host: self,
        }
    }

    #[cfg(test)]
    fn resolution_context(&self) -> Result<ResolutionContext, ContextBuildError> {
        self.run(|executor| executor.resolution_context())
    }

    #[cfg(test)]
    fn eval_expr(
        &self,
        expr: &darkmatter::markdown::compose::expression::Expr,
        fm: &Map<String, Value>,
    ) -> Result<Value, LifecycleExprError> {
        self.run(|executor| executor.eval_expr(expr, fm))
    }

    #[cfg(test)]
    fn resolve_string_value(&self, s: &str, fm: &Map<String, Value>) -> Result<Value, LifecycleExprError> {
        self.run(|executor| executor.resolve_string_value(s, fm))
    }
}

impl LifecycleHost for StackExecutionContext<'_> {
    fn registry(&self) -> &darkmatter::lifecycle::registry::EventRegistry {
        super::claudine_registry()
    }

    /// The binding catalog's view for [`Self::binding_scope`], with the
    /// runtime state's sequence outputs under `seq`.
    fn bindings(&self) -> (Arc<BindingView>, HashMap<String, RuntimeBinding<'static>>) {
        let mut snapshot = self.runtime_state.map_or_else(Default::default, |runtime| runtime.snapshot());
        if let Some(seq) = self.seq { snapshot.seq = Some(seq.clone()); }
        let seq = snapshot.sequence_value();
        runtime_bindings_in_loop(
            self.binding_scope(),
            LifecycleValues {
                err: self.err,
                timing: self.timing,
                group: self.group,
                seq: Some(&seq),
            },
            self.loop_values,
        )
    }

    fn name_coercion_keys(&self) -> Vec<String> {
        super::super::sequence::reserved::NAME_COERCION_KEYS.iter().map(|key| key.to_string()).collect()
    }

    fn resolution_context(
        &self,
        source_path: &Path,
        prepared_context: Option<&ComposeContext>,
        file_resolution_context: &biscuit_file::FileResolutionContext,
        file_ref_fallback_dir: Option<&Path>,
    ) -> Result<ResolutionContext, ContextBuildError> {
        super::super::document_expression_resolution_context(
            source_path,
            prepared_context,
            file_resolution_context,
            file_ref_fallback_dir,
        )
    }

    fn capture_overlay_context(&self, base: &Path, content: &str) -> ComposeContext {
        capture_proxy_with_fallback(base, content)
    }

    fn persisted_data(&self, value: &Value) -> Value {
        crate::composition::closure::persisted_data(value)
    }

    /// Commits to the invocation's runtime state, or — when none is wired —
    /// to a fresh one, so `set` still validates its keys.
    fn commit_set(
        &self,
        engine: &EffectEngine,
        updates: &IndexMap<String, Value>,
        prior_base: &Map<String, Value>,
    ) -> Result<IndexMap<String, Value>, Box<dyn std::error::Error + Send + Sync>> {
        let fallback;
        let state = match self.runtime_state {
            Some(state) => state,
            None => {
                fallback = super::super::runtime_state::RuntimeState::new();
                &fallback
            }
        };
        state.set_batch(engine, updates, prior_base).map_err(Into::into)
    }

    fn in_loop(&self) -> bool {
        self.loop_values.is_some()
    }

    fn record_break(&self, reason: Option<String>, code: u8) {
        if let Some(ambient) = self.loop_values {
            ambient.breaks.record(crate::composition::looping::LoopBreak { reason, code });
        }
    }

    /// Runs the target through this thread's installed runner.
    fn prep(&self, call: &PrepRequest<'_>) -> Result<(), PrepError> {
        let runner = super::prep::current().ok_or(PrepError::Unavailable)?;
        let call = super::prep::PrepCall {
            target: call.target,
            overlay: call.overlay,
            caller: call.caller,
            file_resolution_context: call.file_resolution_context,
            property: call.property,
        };
        runner
            .run(&call)
            .map_err(|failure| PrepError::Failed { message: failure.message, snapshot: failure.snapshot })
    }

    fn user_blacklist(&self, policy_root: Option<&Path>) -> ShellRuleSet {
        crate::harness::shell::user_blacklist(policy_root)
    }

    fn interrupted(&self) -> bool {
        crate::interrupt::interrupted()
    }

    fn is_known_effect(&self, name: &str) -> bool {
        playa::SoundEffect::from_name(name).is_some()
    }

    fn failure(&self, failure: LifecycleRunFailure) -> LifecycleErrorInfo {
        classify_run_failure(failure)
    }
}

/// Projects each lifecycle failure through its [`CompositionError`] twin, so
/// `err.*`, rendering, and machine output read Claudine's catalog.
pub(crate) fn classify_run_failure(failure: LifecycleRunFailure) -> LifecycleErrorInfo {
    let error = match failure {
            LifecycleRunFailure::UnknownEffect { event, name } => CompositionError::LifecycleUnknownEffect(event, name),
            LifecycleRunFailure::ShellTemplateRefused { source_path, property, command, message, error } => {
                CompositionError::LifecycleShellTemplateRefused {
                    source_path,
                    property,
                    command,
                    message,
                    source: Box::new(darkmatter::markdown::MarkdownError::ShellExpansion(Box::new(error))),
                }
            }
            LifecycleRunFailure::EvaluationFailed { source_path, event, surface, message, property, reason, cause } => {
                CompositionError::LifecycleEvaluationError {
                    source_path,
                    event,
                    surface,
                    message,
                    property,
                    reason,
                    cause,
                }
            }
            LifecycleRunFailure::BreakWithoutLoop { source_path, property } => {
                CompositionError::LifecycleBreakWithoutLoop { source_path, property }
            }
            LifecycleRunFailure::PrepUnavailable { source_path, property } => {
                CompositionError::LifecyclePrepUnavailable { source_path, property }
            }
            LifecycleRunFailure::PrepFailed { source_path, property, target, message, snapshot } => {
                CompositionError::LifecyclePrepFailed { source_path, property, target, message, snapshot }
            }
            LifecycleRunFailure::WithEvaluationFailed { source_path, property, path, target, message, cause } => {
                CompositionError::LifecycleWithEvaluationFailed {
                    source_path,
                    property,
                    path,
                    target,
                    message,
                    cause,
                }
            }
            LifecycleRunFailure::CancellationRecovery { source_path, action } => {
                CompositionError::LifecycleCancellationRecovery { source_path, action }
            }
        };
        LifecycleErrorInfo::from_composition_error(&error)
}

/// The run guard derives each event's context with [`StackExecutionContext::with_signal`],
/// so a different event starts a new `current` memo.
impl GuardedEventStack for StackExecutionContext<'_> {
    fn run_guarded(&self, event: GuardedEvent<'_>) -> LifecycleEventOutcome {
        let for_signal = self.with_signal(event.signal);
        let mut context = match event.error {
            Some(error) => for_signal.with_error(error),
            None => for_signal,
        };
        if let Some(runner) = event.shell_runner {
            context.shell_runner = runner;
        }
        context.execute_event(event.config)
    }
}

fn capture_proxy_with_fallback(base: &Path, content: &str) -> ComposeContext {
    #[cfg(test)]
    PROXY_WITH_FALLBACK_CAPTURE_HINTS.with(|hints| hints.borrow_mut().push(content.to_string()));

    ComposeContext::capture_for_content(base, content)
}

#[cfg(test)]
std::thread_local! {
    static PROXY_WITH_FALLBACK_CAPTURE_HINTS: std::cell::RefCell<Vec<String>> =
        const { std::cell::RefCell::new(Vec::new()) };
}

#[cfg(test)]
fn take_proxy_with_fallback_capture_hints() -> Vec<String> {
    PROXY_WITH_FALLBACK_CAPTURE_HINTS.with(|hints| std::mem::take(&mut *hints.borrow_mut()))
}

#[cfg(test)]
mod tests;
