//! Lifecycle stack execution engine.
//!
//! Given a parsed [`LifecycleConfig`] and a [`LifecycleSignal`], this module
//! runs one lifecycle event: it walks the event's ordered nodes serially. A
//! `when` guard is evaluated once, when its item is reached, against the live
//! state, and only the selected body runs. The first flow-control directive
//! reached ends the whole event, however deeply it is nested.
//!
//! ## Scope
//!
//! This is the **engine**. It dispatches communication, side-effect, shell,
//! and expression-function actions through their existing routes and reports
//! which lifecycle control action fired (if any) plus whether an
//! unintentional action error must route the run to `failure`. It does **not**
//! perform the runtime control flow those outcomes imply (`Skip` opt-out,
//! `Proxy` hand-off, `Retry`/`Resume`/`Defer` re-entry) — that wiring lives
//! in the composition runtime.
//!
//! ## Error propagation
//!
//! Two distinct failure channels are reported, because they have different
//! halting policies:
//!
//! - A side-effect **dispatch** failure — a channel/TTS/shell/effect-engine
//!   error after the expression layer resolved cleanly — populates
//!   [`LifecycleEventOutcome::action_error`]. It honors `no_error: true` and
//!   the per-phase routing policy: at a setup-phase event
//!   (`initialize`/`start`/`blocked`) it routes the run to `failure`; at a
//!   terminal-phase event (`success`/`failure`/`finalize`/`loop`) the
//!   composition outcome is unchanged.
//! - A late-binding **evaluation** error — a `when:` guard, an interpolation,
//!   or any event-time expression that *raised* — populates
//!   [`LifecycleEventOutcome::evaluation_error`]. It is **not** suppressible by
//!   `no_error` and halts on every phase (the orchestration wiring lives in the
//!   composition runtime).
//!
//! The explicit `Error` lifecycle action is distinct from both — it is a
//! deliberate author choice surfaced as [`StackControl::Error`] for the
//! runtime to act on.

// rustfmt doesn't support let-chains yet, so nested ifs are required
#![allow(clippy::collapsible_if)]

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::invocation_context::EnvBaseline;

use biscuit_file::{FileReference, FileReferenceKind};
use biscuit_terminal::terminal::Terminal;
use darkmatter::effects::EffectEngine;
use std::sync::Arc;

use darkmatter::markdown::compose::expression::{
    BindingView, Expr, ExpressionError, ExpressionFinder, ResolutionContext, RuntimeBinding,
    evaluate, is_truthy, scalar_string,
};
use darkmatter::markdown::compose::subtree::{SubtreeCompose, layered_session};
use darkmatter::markdown::compose::shell_expansion::template::{
    FilledChain, HoleEvaluator, LookupEvaluator, TemplateShape, fill_template,
};
use darkmatter::markdown::compose::{
    ComposeContext, ComposeOptions, CurrentAuthority, EffectiveState, EffectiveStateBuilder,
    ResolvedShellValue, ShellExpansionError, execute_shell_values_with,
};
use indexmap::IndexMap;
use serde_json::{Map, Value};
use tracing::warn;

use super::super::error::{CompositionError, LifecycleEvaluationReason};
use super::{LifecycleConfig, LifecycleEmitter, LifecycleSignal, tts_config_from_settings};
use super::bindings::{LifecycleScope, LifecycleValues, runtime_bindings_in_loop};
use crate::composition::looping::LoopAmbient;
use super::actions::{
    CommunicationChannel, LifecycleAction, LifecycleActionKind, LifecycleControlAction,
    LifecycleNode, WithOverlay, WithValue, RetryBackoff, RuntimeSet, is_known_side_effect,
};
use crate::composition::coordinator::ActionLocation;
use super::context::{LifecycleCause, LifecycleErrorInfo, LifecycleTiming};
use crate::events::GlobalSettings;
use crate::messaging::RuntimeMessagingSettings;

/// A poisoned live-frontmatter cell means a task panicked mid-stack; there is
/// no partial document state worth recovering.
const LIVE_POISONED: &str = "live frontmatter mutex poisoned by a panicking task";

/// A resolved lifecycle control action — the runtime-flow effect a stack item
/// requested, with every expression argument already evaluated.
///
/// The parse-time [`LifecycleControlAction`] carries unevaluated [`Expr`]
/// arguments; this is its post-evaluation form, suitable for the composition
/// runtime to act on directly.
// `Eq` is intentionally omitted: `Proxy` carries an evaluated overlay of
// `serde_json::Value` (not `Eq`).
#[derive(Debug, Clone, PartialEq)]
pub enum StackControl {
    /// End this event's stack cleanly; outcome unchanged.
    Stop,

    /// Whole-document opt-out (valid only at `initialize`).
    Skip,

    /// Mark this event as failed, with an optional reason.
    Error {
        /// Evaluated human-readable reason, if one was authored.
        reason: Option<String>,
    },

    /// Hand off to another prompt document.
    ///
    /// This is everything *evaluation* can know about a handoff. The
    /// coordinator turns it into an
    /// [`EvaluatedProxyRequest`][crate::composition::EvaluatedProxyRequest] by
    /// adding the one thing the lifecycle surface does not own: the
    /// invocation-wide proxy chain from its run ledger.
    Proxy {
        /// Evaluated target prompt reference (e.g. `@prompts/foo.md`).
        target: String,
        /// The evaluated `with:` overlay, empty when `with:` was omitted or
        /// authored as `{}`. Values are resolved data, never templates: no
        /// `{{ … }}` span survives here into target-time evaluation.
        overlay: IndexMap<String, Value>,
        /// Which authored action requested the handoff.
        location: ActionLocation,
    },

    /// Try the current prompt again.
    Retry {
        /// Additional attempts beyond the original (default `1`).
        max_attempts: u32,
        /// Backoff strategy (default [`RetryBackoff::Fixed`]).
        backoff: RetryBackoff,
        /// Evaluated delay duration (default `"0s"`).
        delay: String,
        /// The evaluated `retry.with` overlay, empty when `with:` was omitted
        /// or authored as `{}`. Resolved data, never templates, evaluated by
        /// the same path as [`StackControl::Proxy`]'s overlay. It applies to the
        /// fresh read of this one transition only; the runtime never retains
        /// it for a later retry or resume.
        overlay: IndexMap<String, Value>,
    },

    /// Resume the agent session with a follow-up message.
    Resume {
        /// Evaluated follow-up prompt.
        message: String,
        /// Additional attempts beyond the original (default `1`).
        max_attempts: u32,
    },

    /// Push this prompt onto the deferred-execution queue.
    Defer {
        /// Evaluated delay duration.
        delay: String,
        /// Evaluated optional reason.
        reason: Option<String>,
    },
}

/// The result of running one lifecycle event's communication + stack.
// `Eq` is intentionally omitted: `LifecycleErrorInfo` carries a `serde_json::Value`
// detail payload (not `Eq`). `PartialEq` is sufficient for the tests that compare
// outcomes.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct LifecycleEventOutcome {
    /// The lifecycle control action that terminated stack processing, if any.
    /// `None` means the stack ran to completion (or there was no stack).
    pub control: Option<StackControl>,

    /// Set when a side-effect **dispatch** failure (a channel/TTS/shell/effect
    /// engine error that was *not* an expression-layer raise, and not marked
    /// `no_error: true`) stopped the stack. Subject to the existing per-phase
    /// routing policy via [`Self::routes_to_failure`]. Carries the error
    /// snapshot so a routed-to `failure` event can expose
    /// `err.kind`/`err.variant`/`err.msg`.
    pub action_error: Option<LifecycleErrorInfo>,

    /// Set when a late-binding **expression-layer evaluation** raised — a
    /// `when:` guard, a top-level or action-string interpolation, or any
    /// event-time expression that threw. Unlike [`Self::action_error`], this is
    /// **not** suppressible by `no_error` and is **not** gated by
    /// [`LifecycleSignal::routes_action_error_to_failure`]: an evaluation error
    /// must surface and halt on every event phase, including terminal-phase
    /// events. Orchestration consults it via [`Self::has_evaluation_error`].
    pub evaluation_error: Option<LifecycleErrorInfo>,
}

impl LifecycleEventOutcome {
    /// Whether the unintentional action error recorded by this outcome must
    /// route the run to `failure`.
    ///
    /// True only when a side-effect dispatch [`Self::action_error`] occurred
    /// **and** `signal` is a setup-phase event
    /// ([`LifecycleSignal::routes_action_error_to_failure`]). Evaluation errors
    /// are deliberately excluded — they halt on every phase regardless of this
    /// setup-phase policy (see [`Self::has_evaluation_error`]).
    pub fn routes_to_failure(&self, signal: LifecycleSignal) -> bool {
        self.action_error.is_some() && signal.routes_action_error_to_failure()
    }

    /// Whether this outcome carries a late-binding expression-layer evaluation
    /// error.
    ///
    /// Evaluation errors surface and halt regardless of event phase, so
    /// orchestration checks this independently of [`Self::routes_to_failure`].
    pub fn has_evaluation_error(&self) -> bool {
        self.evaluation_error.is_some()
    }
}

/// Why an event-time lifecycle expression failed.
///
/// The `Err` type of the executor's expression layer: `when:` guards,
/// interpolation, control-argument evaluation, and side-effect argument
/// evaluation all fail through it. Every arm reaches
/// [`LifecycleErrorInfo::from_error_or_action`], the §D9 snapshot boundary, so
/// the typed value is projected once rather than pre-flattened into prose at
/// each raise site.
///
/// Like [`MarkdownLoadCause`][super::super::error::MarkdownLoadCause], the
/// lower-layer arms are `transparent`: they *replace* the concrete error in the
/// chain rather than adding a hop to it, so a handler recovers the stage by
/// downcasting to this enum and matching its arm.
///
/// The heavy Darkmatter members are boxed to keep the enum (and the `Result`s
/// carrying it) small — `clippy::result_large_err` fires on these hot helpers
/// otherwise.
#[derive(Debug, thiserror::Error)]
pub enum LifecycleExprError {
    /// A parsed expression could not be evaluated.
    #[error(transparent)]
    Evaluate(#[from] Box<darkmatter::markdown::compose::expression::ExpressionError>),

    /// Event-time interpolation (DM2 subtree compose) raised.
    #[error(transparent)]
    Compose(#[from] Box<darkmatter::markdown::MarkdownError>),

    /// A failure the expression layer describes itself, with no lower-layer
    /// error in hand: a control argument of the wrong shape, or template syntax
    /// in a `proxy.with` value that becomes the target's lifecycle
    /// configuration.
    #[error("{0}")]
    Prose(String),

    /// The request's file-resolution context failed validation.
    #[error(transparent)]
    Context(#[from] darkmatter::markdown::compose::ContextBuildError),
}

impl LifecycleExprError {
    /// Build a [`Self::Prose`] arm from anything string-shaped.
    fn prose(message: impl Into<String>) -> Self {
        LifecycleExprError::Prose(message.into())
    }
}

/// A shell command refused by the lifecycle boundary or unable to start.
/// A command that ran and exited nonzero is reported through
/// [`ShellRunner::run`]'s (or [`ShellRunner::run_structured`]'s) `Ok` arm.
/// Spawn failures retain their I/O source.
#[derive(Debug, thiserror::Error)]
pub enum ShellRunError {
    /// The lifecycle has not crossed the preflight boundary.
    #[error("shell commands are forbidden during initialize and before preflight completes; move the command to start or a later event")]
    BeforePreflight,
    /// A `set` value's command did not produce a value: it is missing,
    /// blacklisted, denied, not pre-approved, timed out, ended by a signal, or
    /// (without a result suffix) exited non-zero.
    #[error("{source}")]
    Value {
        /// The Darkmatter shell failure.
        #[source]
        source: darkmatter::markdown::compose::shell_expansion::ShellExpansionError,
    },
    /// The shell process could not be spawned or waited on.
    #[error("command `{command}` failed to run: {source}")]
    Spawn {
        /// The approved command that was attempted.
        command: String,
        /// The underlying spawn/wait failure.
        #[source]
        source: std::io::Error,
    },
    /// The request's file-resolution context failed validation, so no value
    /// could be evaluated against it.
    #[error("{source}")]
    Context {
        /// The builder's validation failure.
        #[source]
        source: darkmatter::markdown::compose::ContextBuildError,
    },
    /// A template-mode program is not a native executable on the child's
    /// search path, so no segment of its command started.
    #[error("{reason}")]
    Target {
        /// The program word as authored.
        program: String,
        /// Why it cannot run without an interpreter.
        #[source]
        reason: Box<darkmatter::markdown::compose::shell_expansion::template::TemplateTargetError>,
    },
}

/// Abstraction over running an approved shell command.
///
/// Lifecycle shell actions are audited against Claudine's command whitelist
/// during pre-flight; this trait runs an already-approved command. Injectable
/// so tests can assert command dispatch without spawning real processes.
pub trait ShellRunner: Sync {
    /// Return the exit code, or [`ShellRunError`] when execution is prohibited
    /// or the process could not be started.
    fn run(&self, command: &str) -> Result<i32, ShellRunError>;

    /// Run a template-mode command whose every argument is already filled,
    /// validated, and checked against the blacklists, and return the exit code
    /// of the last segment that ran.
    ///
    /// Each program is started directly, never through a shell, under the
    /// same contract as [`Self::run`]: inherited standard streams, the same
    /// working directory and environment, and no deadline.
    ///
    /// ## Errors
    ///
    /// [`ShellRunError::BeforePreflight`] where shell execution is prohibited,
    /// [`ShellRunError::Target`] when a program is not a native executable
    /// (nothing starts then), or [`ShellRunError::Spawn`].
    fn run_structured(&self, chain: &FilledChain) -> Result<i32, ShellRunError>;

    /// Whether this runner may start a command at all.
    ///
    /// A template-mode site asks before it reads any placeholder, so a
    /// prohibited route reports the prohibition rather than a value error.
    fn ensure_permitted(&self) -> Result<(), ShellRunError> {
        Ok(())
    }

    /// Execute one `set` action's whole-value `$( … )` assignments and return
    /// each destination's value, in order.
    ///
    /// The default runs them through Darkmatter's frontmatter shell executor
    /// ([`execute_set_shell_values`]); a runner that prohibits shell execution
    /// overrides it.
    fn run_values(
        &self,
        request: &ShellValueRequest<'_>,
    ) -> Result<Vec<(String, Value)>, ShellRunError> {
        execute_set_shell_values(request, None)
    }

    /// This runner, starting its children from `environment` instead of this
    /// process's: the shell route of a document entered under its own map.
    ///
    /// `None` keeps `self`, which is right for a runner that starts no process
    /// of its own (a recorder, [`DisabledShellRunner`]).
    fn for_child_environment(&self, _environment: &EnvBaseline) -> Option<Box<dyn ShellRunner>> {
        None
    }
}

/// One `set` action's shell assignments, ready to execute.
pub struct ShellValueRequest<'a> {
    /// The approved values, in authored order.
    pub values: &'a [&'a ResolvedShellValue],
    /// The pre-write state for conditions, value branches, and template-mode
    /// placeholders. Resolved-byte commands stay fixed at preflight.
    pub state: &'a Map<String, Value>,
    /// The composition source, for diagnostics and file references.
    pub source_path: &'a Path,
    /// The early-binding `ctx.*`/`env.*` snapshot.
    pub context: ComposeContext,
    /// The request's file-resolution context.
    pub file_resolution_context: &'a biscuit_file::FileResolutionContext,
    /// Reads a template-mode value's placeholders against [`Self::state`]
    /// with the event's globals, exactly as the action's other values read.
    pub holes: Option<&'a dyn HoleEvaluator>,
    /// The shell policy root a template-mode value was approved under; its
    /// user blacklist judges the filled command.
    pub policy_root: Option<&'a Path>,
}

/// Runs a `set` action's shell values through Darkmatter in a fresh result
/// cache, so a command two values name runs once and nothing is reused from an
/// earlier event.
///
/// A resolved-byte value's commands are exactly the bytes preflight approved,
/// so they are its pre-approved set; a lifecycle `shell` action runs its
/// approved bytes on the same terms. A template-mode value
/// ([`ResolvedShellValue::template`]) is pre-approved by its shape instead:
/// its placeholders are read through [`ShellValueRequest::holes`], each fills
/// one argument, and the filled command meets both blacklists before it
/// starts. Commands run in the request's launch directory, as a
/// lifecycle `shell` action does, and from `child_environment` when one is
/// given (otherwise they inherit this process's).
///
/// ## Errors
///
/// [`ShellRunError::Value`] carrying the first value's Darkmatter failure, or
/// [`ShellRunError::Context`] when the request's context fails validation.
pub fn execute_set_shell_values(
    request: &ShellValueRequest<'_>,
    child_environment: Option<&EnvBaseline>,
) -> Result<Vec<(String, Value)>, ShellRunError> {
    let mut approved = darkmatter::markdown::compose::shell_expansion::template::ShellPreApprovals::new();
    for value in request.values {
        match value.template() {
            Some(template) => {
                approved.insert_template(TemplateShape::from_template(template));
            }
            None => approved.extend_exact(value.commands()),
        }
    }
    let mut options = ComposeOptions::new_with_context(request.context.clone())
        .with_source_file(request.source_path)
        .with_pre_approved_commands(approved)
        .with_shell_working_directory(request.file_resolution_context.request_cwd());
    if let Some(policy_root) = request.policy_root {
        options = options.with_shell_policy_root(policy_root);
    }
    if let Some(environment) = child_environment {
        options = options.with_shell_child_environment(environment.shell_child_environment());
    }
    let compose_request =
        crate::composition::compose_request(options, request.file_resolution_context.clone())
            .map_err(|source| ShellRunError::Context { source })?;
    let state = request
        .state
        .iter()
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect();
    execute_shell_values_with(request.values, state, &compose_request, request.holes)
        .map_err(|source| ShellRunError::Value { source })
}

/// Runner for initialization and its early catch handlers. Approvals cannot
/// enable shell execution on this route.
#[derive(Debug, Default, Clone, Copy)]
pub struct DisabledShellRunner;

impl ShellRunner for DisabledShellRunner {
    fn run(&self, _command: &str) -> Result<i32, ShellRunError> {
        Err(ShellRunError::BeforePreflight)
    }

    fn run_structured(&self, _chain: &FilledChain) -> Result<i32, ShellRunError> {
        Err(ShellRunError::BeforePreflight)
    }

    fn ensure_permitted(&self) -> Result<(), ShellRunError> {
        Err(ShellRunError::BeforePreflight)
    }

    fn run_values(
        &self,
        _request: &ShellValueRequest<'_>,
    ) -> Result<Vec<(String, Value)>, ShellRunError> {
        Err(ShellRunError::BeforePreflight)
    }
}

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

    fn for_child_environment(&self, environment: &EnvBaseline) -> Option<Box<dyn ShellRunner>> {
        Some(Box::new(DocumentShellRunner::new(environment.clone())))
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
        execute_set_shell_values(request, Some(&self.environment))
    }

    fn for_child_environment(&self, environment: &EnvBaseline) -> Option<Box<dyn ShellRunner>> {
        Some(Box::new(Self::new(environment.clone())))
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

/// Everything the executor needs to run one lifecycle event.
///
/// Construct one per event with the active [`LifecycleSignal`], the composed
/// frontmatter, the lifecycle globals (`err`/`timing`), the invocation's
/// `current` refresh authority, and the side-effect / shell / emitter routes.
pub struct StackExecutionContext<'a> {
    /// The event being processed.
    pub signal: LifecycleSignal,
    /// The binding scope expressions evaluate in; `None` is the event
    /// `signal` names. A sequence task's `setup:`/`teardown:` stack sets its
    /// own scope ([`Self::in_scope`]) because it reuses the step's context.
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
    /// start. When `Some`, `build_state` reuses it for `ctx.*`/`env.*` instead
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

/// What a single action did.
enum ActionStep {
    /// Continue to the next action / item.
    Continue,
    /// A lifecycle control action fired; terminate the event's stack.
    Control(StackControl),
    /// A side-effect dispatch failure (not suppressed by `no_error`); stop the
    /// stack and report it as an [`LifecycleEventOutcome::action_error`].
    Errored(LifecycleErrorInfo),
    /// An expression-layer evaluation raise; stop the stack and report it as an
    /// [`LifecycleEventOutcome::evaluation_error`]. Never suppressed by
    /// `no_error`.
    EvaluationErrored(LifecycleErrorInfo),
}

/// Which layer an action error originated in.
///
/// The stack loop routes the two to different outcome channels: an
/// [`ActionFailure::Evaluation`] (an expression/binding raise) halts on every
/// phase and ignores `no_error`; an [`ActionFailure::Dispatch`] (a side-effect
/// that failed after a clean evaluation) keeps the existing `no_error` /
/// per-phase policy.
enum ActionFailure {
    /// An expression/binding raise: control-argument evaluation, message/shell
    /// interpolation, or side-effect argument evaluation.
    Evaluation(LifecycleErrorInfo),
    /// A side-effect dispatch failure: shell non-zero, an effect-engine error,
    /// or an invalid resolved effect name.
    Dispatch(LifecycleErrorInfo),
}


impl StackExecutionContext<'_> {
    /// Run the event's stack.
    pub fn execute_event(&self, config: &LifecycleConfig) -> LifecycleEventOutcome {
        self.execute_stack_for_signal(config)
    }

    /// Run the stack for this event's signal.
    pub fn execute_stack_for_signal(&self, config: &LifecycleConfig) -> LifecycleEventOutcome {
        self.execute_stack(config.stack(self.signal))
    }

    /// Run one already-parsed action stack — a task's `setup:` or `teardown:`.
    ///
    /// Mutations reach the live cell exactly as they do for an event stack.
    /// Diagnostics name each action's authored path, which the parser rooted
    /// at the task property (`tasks[0].setup[1].shell`), so a task-stack
    /// failure names the authored task property rather than the synthetic
    /// signal the stack was parsed under.
    pub fn execute_action_stack(&self, nodes: &[LifecycleNode]) -> LifecycleEventOutcome {
        self.execute_stack(nodes)
    }

    /// Dispatch one action as a task's `side_effect:` primary and return the
    /// value the effect produced.
    ///
    /// Only a side-effect action is executable work: the standard grammar also
    /// admits communication and flow-control verbs, and those are rejected here
    /// rather than silently succeeding with no effect.
    ///
    /// `no_error: true` keeps its dispatch-only meaning — a failed effect is
    /// suppressed and yields `Value::Null`, while an expression-layer raise
    /// still surfaces.
    ///
    /// ## Errors
    ///
    /// Returns the error snapshot for an unsuppressed dispatch failure, an
    /// expression-layer raise, or a non-side-effect action.
    /// `property` is the source-rooted semantic path of a runtime `set`
    /// mapping; nested value failures extend it with their key/index suffix.
    pub fn dispatch_task_side_effect(
        &self,
        action: &LifecycleAction,
        property: &str,
    ) -> Result<Value, LifecycleErrorInfo> {
        let mut working: Map<String, Value> = match self.live_frontmatter {
            Some(cell) => cell.lock().expect(LIVE_POISONED).clone(),
            None => self.frontmatter.clone(),
        };
        let result = self.dispatch_task_side_effect_inner(action, property, &mut working);
        if let Some(cell) = self.live_frontmatter {
            *cell.lock().expect(LIVE_POISONED) = working;
        }
        result
    }

    /// The dispatch half of [`Self::dispatch_task_side_effect`], against a
    /// caller-owned working map.
    fn dispatch_task_side_effect_inner(
        &self,
        action: &LifecycleAction,
        property: &str,
        working: &mut Map<String, Value>,
    ) -> Result<Value, LifecycleErrorInfo> {
        let dispatched = match &action.kind {
            LifecycleActionKind::RuntimeSet(set) => self
                .dispatch_runtime_set(set, property, working)
                .map(|prior| Value::Object(prior.into_iter().collect())),
            LifecycleActionKind::SideEffect(effect) => {
                self.dispatch_side_effect(&effect.verb, &effect.args, working)
            }
            LifecycleActionKind::ExpressionFunction(func)
                if is_known_side_effect(&func.function) =>
            {
                self.dispatch_side_effect(&func.function, &func.args, working)
            }
            _ => {
                return Err(LifecycleErrorInfo::from_action_failure(
                    "side_effect",
                    "value is not a side-effect action",
                ));
            }
        };
        match dispatched {
            Ok(value) => Ok(value),
            Err(ActionFailure::Dispatch(info)) if action.no_error => {
                warn!(
                    kind = info.kind,
                    variant = %info.variant,
                    message = %info.msg,
                    "task side effect errored (no_error: suppressed)"
                );
                Ok(Value::Null)
            }
            Err(failure) => Err(match failure {
                ActionFailure::Evaluation(info) | ActionFailure::Dispatch(info) => info,
            }),
        }
    }

    /// Return a copy of this context targeting a different lifecycle signal.
    ///
    /// Used by [`LifecycleRunGuard::execute_event`](super::lifecycle::LifecycleRunGuard::execute_event)
    /// so one constructed context can service every signal in a run.
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
    /// mirrors its write onto the working map that [`Self::execute_stack`]
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

    /// Return a copy of this context that reuses `prepared` as its single
    /// early-binding snapshot.
    ///
    /// Lets a caller that evaluates many leaves against one event (a whole
    /// `proxy.with` overlay) capture the snapshot-less fallback context once
    /// and reuse it, instead of re-capturing per leaf.
    fn with_prepared_context<'b>(
        &'b self,
        prepared: &'b ComposeContext,
    ) -> StackExecutionContext<'b> {
        StackExecutionContext {
            seq: self.seq,
            signal: self.signal,
            scope: self.scope,
            frontmatter: self.frontmatter,
            live_frontmatter: self.live_frontmatter,
            runtime_state: self.runtime_state,
            err: self.err,
            timing: self.timing,
            current: self.current.clone(),
            group: self.group,
            loop_values: self.loop_values,
            base_dir: self.base_dir,
            ctx_base_dir: self.ctx_base_dir,
            prepared_context: Some(prepared),
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

    /// This scope's binding view and a runtime entry for every lifecycle
    /// global it declares, from the catalog.
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

    /// Build resolution state for expressions authored by `source_path`.
    ///
    /// Implicit references resolve source-relative, then against the repository. The
    /// launch-area `ctx_base_dir` is retained as `file_ref_fallback_dir`
    /// diagnostic metadata; it is not a third resolution candidate. Top-level
    /// CLI references are resolved from launch context before this executor is
    /// entered.
    fn resolution_context(
        &self,
    ) -> Result<ResolutionContext, darkmatter::markdown::compose::ContextBuildError> {
        super::super::document_expression_resolution_context(
            self.source_path,
            self.prepared_context,
            self.file_resolution_context,
            self.ctx_base_dir,
        )
    }

    /// The early-binding `ctx.*`/`env.*` snapshot for this event.
    ///
    /// Reuses the single `prepared_context` captured once at composition start
    /// when present — so plain `ctx.*`/`env.*` in a lifecycle string is the
    /// exact snapshot the body composed against and cannot diverge. Falls back
    /// to a demand-driven capture (rooted at the launch area / prompt dir,
    /// scanned against `scan_hint`) only for callers that supply no snapshot.
    fn early_binding_context(&self, scan_hint: &str) -> ComposeContext {
        match self.prepared_context {
            Some(prepared) => prepared.clone(),
            None => {
                let base = self
                    .ctx_base_dir
                    .or(self.base_dir)
                    .unwrap_or_else(|| Path::new("."));
                ComposeContext::capture_for_content(base, scan_hint)
            }
        }
    }

    /// Build the DM2 effective state over the current document `fm`.
    ///
    /// `ctx.*`/`env.*` come from [`Self::early_binding_context`] — the single
    /// composition-start snapshot when available, otherwise a demand-driven
    /// re-capture against `scan_hint`. `err`/`timing` are the event-time
    /// globals (supplied separately by [`Self::bindings`]);
    /// `current.*` is the reserved root this state's refresh authority serves.
    fn build_state(&self, fm: &Map<String, Value>, scan_hint: &str) -> EffectiveState {
        let frontmatter: HashMap<String, Value> =
            fm.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
        let context = self.early_binding_context(scan_hint);
        EffectiveStateBuilder::new()
            .with_frontmatter(frontmatter)
            .with_name_coercion_keys(super::super::sequence::reserved::NAME_COERCION_KEYS.iter().map(|key| key.to_string()).collect())
            .with_context(context)
            // Every event builds its own state, so every event starts fresh
            // evaluation scopes: a `current.<key>` read in a later event
            // observes the fact as it stands then, not as `ctx` froze it.
            .with_current_authority(self.current.clone().unwrap_or_default())
            // A deferred lifecycle subtree never defines `ctx`; downgrade any
            // pathological `ctx` shape to a warning rather than aborting.
            .with_allow_ctx_override(true)
            .build()
            .unwrap_or_else(|_| {
                EffectiveState::new(
                    &fm.iter().map(|(k, v)| (k.clone(), v.clone())).collect(),
                    None,
                    self.early_binding_context(""),
                )
            })
    }

    /// Evaluate a parsed expression at event-time against the live document
    /// state plus the injected globals, through Darkmatter's layered lookup.
    fn eval_expr(&self, expr: &Expr, fm: &Map<String, Value>) -> Result<Value, LifecycleExprError> {
        let state = self.build_state(fm, &expr.to_string());
        let (view, globals) = self.bindings();
        evaluate_expr_in(expr, &state, view, globals, Some(self.resolution_context()?))
    }

    /// Interpolate a string's `{{ … }}` spans at event-time through Darkmatter's
    /// subtree compose (DM2), preserving whole-value typing.
    ///
    /// Subtree compose fails closed (C4): a malformed span, an unknown
    /// function, or a read of a global this scope declares unavailable returns
    /// an error instead of degrading to empty. An absent document property
    /// renders as `null`/empty.
    ///
    /// `s` is authored text and is scanned exactly once. Whatever the spans
    /// insert is data: a frontmatter value or file content that itself holds
    /// `{{ … }}` or `$( … )` is delivered verbatim, never expanded again.
    fn resolve_string_value(
        &self,
        s: &str,
        fm: &Map<String, Value>,
    ) -> Result<Value, LifecycleExprError> {
        let state = self.build_state(fm, s);
        let (view, globals) = self.bindings();
        interpolate_text_in(s, &state, view, globals, Some(self.resolution_context()?))
    }

    /// Evaluate one action operand at event time.
    ///
    /// An authored literal body with `{{ … }}` spans is a template: its spans
    /// are interpolated once, through DM2. Every other expression — a
    /// whole-value span such as `{{ frontmatter('log.md', 'note') }}` — yields
    /// data, returned as is: text it produced is never scanned again, so an
    /// agent-written `{{ … }}` or `$( … )` arrives exactly as written.
    fn evaluate_operand(
        &self,
        expr: &Expr,
        fm: &Map<String, Value>,
    ) -> Result<Value, LifecycleExprError> {
        match expr {
            Expr::StringLiteral(text) if is_template_operand(expr) => {
                self.resolve_string_value(text, fm)
            }
            _ => self.eval_expr(expr, fm),
        }
    }

    /// Validate a resolved sound-effect name against the catalog immediately
    /// before dispatch (C4 / deferred effect validation).
    ///
    /// An `effect: "{{name}}"` field or `{"effect": "{{name}}"}` positional
    /// action cannot be validated at prepare time because the name is
    /// interpolation-dependent; this checks the *resolved* name and reports
    /// [`CompositionError::LifecycleUnknownEffect`] (carried as the action
    /// error's variant) so an invalid name fails closed rather than playing
    /// nothing.
    fn validate_effect_name(&self, name: &str) -> Result<(), LifecycleErrorInfo> {
        if playa::SoundEffect::from_name(name).is_some() {
            return Ok(());
        }
        let err = CompositionError::LifecycleUnknownEffect(
            self.signal.property_name().to_string(),
            name.to_string(),
        );
        Err(LifecycleErrorInfo::from_composition_error(&err))
    }

    /// Process the typed stack top to bottom.
    ///
    /// Holds an evolving in-memory `working` frontmatter, seeded from the shared
    /// cross-event live cell when present (so this event sees frontmatter
    /// mutations made by *earlier* events in the same attempt) and otherwise
    /// from the composed document state. A frontmatter-mutating side effect that
    /// targets the document mirrors its change onto `working`, so a later action
    /// in the same stack resolves `{{ … }}` against the mutated value
    /// (just-in-time resolution — not a single snapshot taken when the event
    /// fired). On every return path the final `working` is written back to the
    /// live cell so a *later* event observes this event's mutations — including
    /// mutations made before an early control/error return (the side effect
    /// already hit disk).
    fn execute_stack(&self, nodes: &[LifecycleNode]) -> LifecycleEventOutcome {
        if nodes.is_empty() {
            return LifecycleEventOutcome::default();
        }
        let mut working: Map<String, Value> = match self.live_frontmatter {
            Some(cell) => cell.lock().expect(LIVE_POISONED).clone(),
            None => self.frontmatter.clone(),
        };
        let outcome = self
            .execute_nodes(nodes, &mut working)
            .unwrap_or_default();
        if let Some(cell) = self.live_frontmatter {
            *cell.lock().expect(LIVE_POISONED) = working;
        }
        outcome
    }

    /// Run one ordered node list against a caller-owned `working` map.
    ///
    /// Returns `Some` when the event must end — a directive fired, or an
    /// action or guard failed — so every enclosing list stops at once: a
    /// directive inside a nested `else` ends the whole event, not just its
    /// branch. `None` means the list ran to completion. Every return path
    /// leaves `working` holding the mutations made up to that point, which
    /// the caller persists.
    ///
    /// A guard is evaluated once, when its item is reached, against the live
    /// state; only the selected body runs. A guard that raises is an
    /// evaluation error and never selects `else`.
    fn execute_nodes(
        &self,
        nodes: &[LifecycleNode],
        working: &mut Map<String, Value>,
    ) -> Option<LifecycleEventOutcome> {
        for node in nodes {
            let ended = match node {
                LifecycleNode::Conditional(conditional) => {
                    match self.when_matches(Some(&conditional.when), working) {
                        Ok(true) => self.execute_nodes(&conditional.then, working),
                        Ok(false) => self.execute_nodes(&conditional.otherwise, working),
                        Err(info) => Some(LifecycleEventOutcome {
                            evaluation_error: Some(
                                info.at_property(conditional.path.field("when").to_string()),
                            ),
                            ..Default::default()
                        }),
                    }
                }
                LifecycleNode::Action(action) => {
                    let location = ActionLocation::new(self.signal, action.path.clone());
                    let property = action.path.to_string();
                    match self.run_action(action, location, &property, working) {
                        ActionStep::Continue => None,
                        ActionStep::Control(control) => Some(LifecycleEventOutcome {
                            control: Some(control),
                            ..Default::default()
                        }),
                        ActionStep::Errored(info) => Some(LifecycleEventOutcome {
                            action_error: Some(info),
                            ..Default::default()
                        }),
                        ActionStep::EvaluationErrored(info) => Some(LifecycleEventOutcome {
                            evaluation_error: Some(info.at_property(property)),
                            ..Default::default()
                        }),
                    }
                }
            };
            if ended.is_some() {
                return ended;
            }
        }
        None
    }

    /// Evaluate an optional `when:` clause, failing closed on an unresolvable
    /// guard. Omitted clauses always match (`Ok(true)`).
    ///
    /// A `when:` guard reads live document state, evaluated just in time
    /// against the live `fm`. An absent property is `null`, so a guard over it
    /// is falsy and skips the item. A malformed expression, an unknown
    /// function, or a read of a global this scope declares unavailable returns
    /// `Err`, so the event fails closed before any side effect dispatches.
    fn when_matches(
        &self,
        when: Option<&Expr>,
        fm: &Map<String, Value>,
    ) -> Result<bool, LifecycleErrorInfo> {
        let Some(expr) = when else {
            return Ok(true);
        };
        self.eval_expr(expr, fm)
            .map(|value| is_truthy(&value))
            .map_err(|error| LifecycleErrorInfo::from_expr_error("when", error))
    }

    /// Run one action, applying the `no_error` escape hatch.
    ///
    /// `no_error` is scoped to side-effect **dispatch** failures only: an
    /// expression-layer **evaluation** raise always halts the stack and
    /// surfaces, because a thrown guard/interpolation is a defect the author
    /// cannot meaningfully "tolerate" the way a flaky channel send can be.
    fn run_action(
        &self,
        action: &LifecycleAction,
        location: ActionLocation,
        property: &str,
        working: &mut Map<String, Value>,
    ) -> ActionStep {
        match self.execute_action_inner(action, location, property, working) {
            Ok(None) => ActionStep::Continue,
            Ok(Some(control)) => ActionStep::Control(control),
            Err(ActionFailure::Evaluation(info)) => {
                warn!(
                    kind = info.kind,
                    variant = %info.variant,
                    message = %info.msg,
                    "lifecycle action evaluation error"
                );
                ActionStep::EvaluationErrored(info)
            }
            Err(ActionFailure::Dispatch(info)) => {
                if action.no_error {
                    warn!(
                        kind = info.kind,
                        variant = %info.variant,
                        message = %info.msg,
                        "lifecycle action errored (no_error: suppressed)"
                    );
                    ActionStep::Continue
                } else {
                    warn!(
                        kind = info.kind,
                        variant = %info.variant,
                        message = %info.msg,
                        "lifecycle action errored"
                    );
                    ActionStep::Errored(info)
                }
            }
        }
    }

    /// Execute one action's body against the evolving `working` frontmatter.
    ///
    /// Returns `Ok(None)` to continue, `Ok(Some(control))` when a lifecycle
    /// control action fired, or `Err(ActionFailure)` on an action error — tagged
    /// [`ActionFailure::Evaluation`] for an expression-layer raise (which always
    /// halts) and [`ActionFailure::Dispatch`] for a side-effect failure (subject
    /// to `no_error` and the per-phase policy).
    ///
    /// `location` is the proxy-provenance identity; `property` is the authored
    /// path the same action reports in diagnostics. The two differ for a task
    /// `setup:`/`teardown:` stack, whose signal is synthetic.
    fn execute_action_inner(
        &self,
        action: &LifecycleAction,
        location: ActionLocation,
        property: &str,
        working: &mut Map<String, Value>,
    ) -> Result<Option<StackControl>, ActionFailure> {
        match &action.kind {
            LifecycleActionKind::LifecycleControl(control) => self
                .resolve_control(control, location, working)
                .map(Some)
                .map_err(ActionFailure::Evaluation),
            LifecycleActionKind::Communication(comm) => {
                let message = self
                    .render_message(&comm.text, working)
                    .map_err(|error| {
                        ActionFailure::Evaluation(LifecycleErrorInfo::from_expr_error(
                            comm.channel.verb(),
                            error,
                        ))
                    })?;
                // Deferred effect validation (C4): an `effect` positional
                // action's name is only known after interpolation, so validate
                // the resolved name before dispatch. The interpolation already
                // succeeded, so an invalid catalog name is a dispatch failure.
                if comm.channel == CommunicationChannel::Effect {
                    self.validate_effect_name(&message)
                        .map_err(ActionFailure::Dispatch)?;
                }
                self.emit_communication(comm.channel, &message);
                Ok(None)
            }
            LifecycleActionKind::Shell(shell) => {
                self.run_shell_action(shell, property, working).map(|()| None)
            }
            LifecycleActionKind::RuntimeSet(set) => {
                // `property` is the `set` verb's own authored path.
                self.dispatch_runtime_set(set, property, working)
                    .map(|_| None)
            }
            LifecycleActionKind::SideEffect(effect) => self
                .dispatch_side_effect(&effect.verb, &effect.args, working)
                .map(|_| None),
            LifecycleActionKind::ExpressionFunction(func) => {
                // A positional action whose verb is a known side effect was
                // parsed as an expression-function action; route it to the
                // side-effect engine here.
                if is_known_side_effect(&func.function) {
                    return self
                        .dispatch_side_effect(&func.function, &func.args, working)
                        .map(|_| None);
                }
                self.invoke_expression_function(&func.function, &func.args, working)
                    .map(|_| None)
                    .map_err(|error| {
                        ActionFailure::Evaluation(LifecycleErrorInfo::from_expr_error(
                            func.function.clone(),
                            error,
                        ))
                    })
            }
        }
    }

    /// Evaluate a message expression to its display string at event-time.
    ///
    /// A literal-default action body is an [`Expr::StringLiteral`] whose
    /// `{{ … }}` spans interpolate once through DM2 against the current document
    /// `fm` plus the injected globals; a whole-value `{{ … }}` span resolves to
    /// a typed expression whose result is data ([`Self::evaluate_operand`]).
    ///
    /// An absent document property renders through ordinary `null` semantics.
    /// A genuine evaluation error fails before dispatch.
    fn render_message(
        &self,
        expr: &Expr,
        fm: &Map<String, Value>,
    ) -> Result<String, LifecycleExprError> {
        self.evaluate_operand(expr, fm).map(|value| scalar_string(&value))
    }

    /// Emit a resolved communication message on the given channel.
    fn emit_communication(&self, channel: CommunicationChannel, message: &str) {
        match channel {
            CommunicationChannel::Say | CommunicationChannel::Speak => {
                let config = tts_config_from_settings(self.settings.tts.as_ref());
                self.emitter.emit_speech(message, config);
            }
            CommunicationChannel::Effect => self.emitter.emit_effect(message),
            CommunicationChannel::Message => {
                self.emitter
                    .emit_message(message, self.source_path, self.repo_root, self.messaging);
            }
            CommunicationChannel::Notify => self.emitter.emit_notification(message),
            CommunicationChannel::Stderr => {
                self.emitter.emit_stderr(self.signal, message, self.term);
            }
            CommunicationChannel::Info => self.emitter.emit_info(message, self.term),
            CommunicationChannel::Warn => self.emitter.emit_warn(message, self.term),
            CommunicationChannel::Success => self.emitter.emit_success(message, self.term),
            CommunicationChannel::Stdout => self.emitter.emit_stdout(message, self.term),
        }
    }

    /// Run a shell action. A failed command-string interpolation is an
    /// evaluation error; a non-zero exit or spawn failure is a dispatch error
    /// (subject to `no_error` upstream). Initialization and preflight prohibitions
    /// are unsuppressible. `on_error` is emitted as a warning status line before
    /// a dispatch error propagates.
    ///
    /// A site approved by its shape runs its template
    /// ([`Self::run_shell_template`]); one approved in neither mode is refused.
    fn run_shell_action(
        &self,
        shell: &super::actions::ShellAction,
        property: &str,
        fm: &Map<String, Value>,
    ) -> Result<(), ActionFailure> {
        if self.signal == LifecycleSignal::Initialize {
            return Err(ActionFailure::Evaluation(LifecycleErrorInfo::from_error_or_action(
                "shell", &ShellRunError::BeforePreflight,
            )));
        }
        if let Some(site) = &shell.template {
            match site.mode() {
                Some(super::shell_template::LifecycleShellMode::ResolvedBytes) => {}
                Some(super::shell_template::LifecycleShellMode::Template { policy_root }) => {
                    return self.run_shell_template(shell, site, policy_root.as_deref(), property, fm);
                }
                None => {
                    self.ensure_shell_permitted("shell")?;
                    return Err(self.not_approved("shell", property, &site.template().authored));
                }
            }
        }
        let command = self.render_shell_text(shell, &shell.command, fm).map_err(|error| {
            ActionFailure::Evaluation(LifecycleErrorInfo::from_expr_error("shell", error))
        })?;
        match self.shell_runner.run(&command) {
            Err(ShellRunError::BeforePreflight) => Err(ActionFailure::Evaluation(
                LifecycleErrorInfo::from_error_or_action("shell", &ShellRunError::BeforePreflight),
            )),
            Ok(0) => Ok(()),
            Ok(code) => {
                if let Some(on_error) = &shell.on_error {
                    if let Ok(text) = self.render_shell_text(shell, on_error, fm) {
                        self.emitter.emit_warn(&text, self.term);
                    }
                }
                Err(ActionFailure::Dispatch(LifecycleErrorInfo::from_action_failure(
                    "shell",
                    format!("command `{command}` exited with code {code}"),
                )))
            }
            // `ShellRunError` owns the "command `…` failed to run: …" prose, so
            // the snapshot projects it rather than the executor rebuilding it
            // around an untyped runner error.
            Err(spawn_err) => Err(ActionFailure::Dispatch(
                LifecycleErrorInfo::from_error_or_action("shell", &spawn_err),
            )),
        }
    }

    /// Run a shell action approved by its shape.
    ///
    /// Every placeholder is read from the state current now, checked against
    /// its declared type, and filled into exactly one argument; the whole
    /// filled command then meets the built-in and user blacklists, and only
    /// then does any segment start. A value that is missing or rejected, a
    /// blacklisted fill, and a program that needs an interpreter are
    /// evaluation errors: `no_error` cannot suppress them, and their message
    /// never shows a value. A non-zero exit is a dispatch error, as for bytes.
    fn run_shell_template(
        &self,
        shell: &super::actions::ShellAction,
        site: &super::shell_template::LifecycleShellTemplate,
        policy_root: Option<&Path>,
        property: &str,
        fm: &Map<String, Value>,
    ) -> Result<(), ActionFailure> {
        self.ensure_shell_permitted("shell")?;
        let template = site.template();
        let authored = template.authored.as_str();
        let chain = self
            .with_hole_evaluator(fm, authored, |holes| fill_template(template, holes, site.late_values()))
            .map_err(|error| ActionFailure::Evaluation(LifecycleErrorInfo::from_expr_error("shell", error)))?
            .map_err(|error| {
                let ctx = self.template_context(authored);
                self.template_refusal(property, authored, error.into_shell_error(&ctx, authored, &template_origin(property)))
            })?;
        let blacklist = crate::harness::shell::user_blacklist(policy_root);
        if let Some(reason) = darkmatter::markdown::compose::shell_expansion::check_filled_chain(&chain, &blacklist) {
            let error = ShellExpansionError::Blacklisted {
                ctx: Box::new(self.template_context(authored)),
                command: authored.to_string(),
                reason,
                origin: template_origin(property),
            };
            return Err(self.template_refusal(property, authored, error));
        }
        match self.shell_runner.run_structured(&chain) {
            Ok(0) => Ok(()),
            Ok(code) => {
                if let Some(on_error) = &shell.on_error {
                    if let Ok(text) = self.render_shell_text(shell, on_error, fm) {
                        self.emitter.emit_warn(&text, self.term);
                    }
                }
                // The authored text: the filled arguments may hold a secret.
                Err(ActionFailure::Dispatch(LifecycleErrorInfo::from_action_failure(
                    "shell",
                    format!("command `{authored}` exited with code {code}"),
                )))
            }
            Err(ShellRunError::BeforePreflight) => Err(ActionFailure::Evaluation(
                LifecycleErrorInfo::from_error_or_action("shell", &ShellRunError::BeforePreflight),
            )),
            Err(ShellRunError::Target { reason, .. }) => {
                let error = ShellExpansionError::TemplateTarget {
                    ctx: Box::new(self.template_context(authored)),
                    command: authored.to_string(),
                    reason,
                    origin: template_origin(property),
                };
                Err(self.template_refusal(property, authored, error))
            }
            Err(spawn_err) => Err(ActionFailure::Dispatch(LifecycleErrorInfo::from_error_or_action(
                "shell", &spawn_err,
            ))),
        }
    }

    /// The runner's prohibition, if any, as the unsuppressible error a shell
    /// prohibition always is.
    fn ensure_shell_permitted(&self, verb: &str) -> Result<(), ActionFailure> {
        self.shell_runner.ensure_permitted().map_err(|error| {
            ActionFailure::Evaluation(LifecycleErrorInfo::from_error_or_action(verb.to_string(), &error))
        })
    }

    /// A site with a placeholder that pre-flight approved in neither mode.
    fn not_approved(&self, verb: &str, property: &str, authored: &str) -> ActionFailure {
        ActionFailure::Evaluation(
            LifecycleErrorInfo::from_action_failure(
                verb.to_string(),
                format!(
                    "`{property}` runs `{authored}`, which pre-flight never approved as its bytes or \
                     its shape, so it cannot run"
                ),
            )
            .at_property(property.to_string()),
        )
    }

    /// A template-mode failure that refused the command before it ran:
    /// unsuppressible, with Darkmatter's typed cause kept for `err.*`.
    fn template_refusal(&self, property: &str, authored: &str, error: ShellExpansionError) -> ActionFailure {
        let message = error.to_string();
        let diagnostic = CompositionError::LifecycleShellTemplateRefused {
            source_path: self.source_path.to_path_buf(),
            property: property.to_string(),
            command: authored.to_string(),
            message,
            source: Box::new(darkmatter::markdown::MarkdownError::ShellExpansion(Box::new(error))),
        };
        ActionFailure::Evaluation(
            LifecycleErrorInfo::from_composition_error(&diagnostic).at_property(property.to_string()),
        )
    }

    /// The authored command as the excerpt a template diagnostic shows.
    fn template_context(&self, authored: &str) -> biscuit_terminal::errors::SourceContext {
        biscuit_terminal::errors::SourceContext::new(
            self.source_path.to_path_buf(),
            self.source_path.to_path_buf(),
            authored.to_string(),
        )
    }

    /// Run `f` with an evaluator that reads a placeholder against `fm` and
    /// this event's globals, exactly as every other action expression reads.
    fn with_hole_evaluator<R>(
        &self,
        fm: &Map<String, Value>,
        scan_hint: &str,
        f: impl FnOnce(&dyn HoleEvaluator) -> R,
    ) -> Result<R, LifecycleExprError> {
        let state = self.build_state(fm, scan_hint);
        let (view, globals) = self.bindings();
        let session = layered_session(&state, globals, Some(view), Some(self.resolution_context()?)).map_err(
            |error| LifecycleExprError::Evaluate(Box::new(ExpressionError::Binding(Box::new(error)))),
        )?;
        Ok(f(&LookupEvaluator::new(&session)))
    }

    /// The text of a shell action's `command` or `on_error`.
    ///
    /// A pre-flight-resolved literal is the approved byte string and runs as it
    /// stands; everything else renders like a message body.
    fn render_shell_text(
        &self,
        shell: &super::actions::ShellAction,
        expr: &Expr,
        fm: &Map<String, Value>,
    ) -> Result<String, LifecycleExprError> {
        match expr {
            Expr::StringLiteral(text) if shell.pre_resolved => Ok(text.clone()),
            _ => self.render_message(expr, fm),
        }
    }

    /// Dispatch a Darkmatter side effect by verb with positional, evaluated
    /// arguments.
    ///
    /// Arguments follow the communication-body rule ([`Self::evaluate_operand`]):
    /// an authored literal interpolates once, and a whole-value span keeps its
    /// typed result, which is data and is never interpolated again. A frontmatter-mutating verb
    /// that targets the document mirrors its change onto `working` so a later
    /// action in the same stack reads the mutated value. Lifecycle `set`
    /// mappings use [`Self::dispatch_runtime_set`] instead of this positional
    /// Darkmatter-effect path.
    fn dispatch_side_effect(
        &self,
        verb: &str,
        args: &[Expr],
        working: &mut Map<String, Value>,
    ) -> Result<Value, ActionFailure> {
        // Argument evaluation/interpolation is the expression layer: a raise
        // here is an evaluation error.
        let values = args
            .iter()
            .map(|expr| self.evaluate_operand(expr, working))
            .collect::<Result<Vec<_>, LifecycleExprError>>()
            .map_err(|error| ActionFailure::Evaluation(LifecycleErrorInfo::from_expr_error(verb, error)))?;
        // From here on, an error is a side-effect dispatch failure (a missing
        // argument, an unknown verb, or an effect-engine error).
        let dispatch_err =
            |msg: String| ActionFailure::Dispatch(LifecycleErrorInfo::from_action_failure(verb, msg));
        let engine = self.effect_engine;
        let s = |idx: usize| -> Result<String, ActionFailure> {
            values
                .get(idx)
                .map(scalar_string)
                .ok_or_else(|| dispatch_err(format!("`{verb}` is missing a required argument")))
        };
        let path = |idx: usize| -> Result<String, ActionFailure> {
            let raw = s(idx)?;
            self.resolve_effect_path(verb, &raw)
        };
        let v = |idx: usize| -> Result<Value, ActionFailure> {
            values
                .get(idx)
                .cloned()
                .ok_or_else(|| dispatch_err(format!("`{verb}` is missing a required argument")))
        };

        // Every written value is the product of one evaluation, so it is data:
        // it is stored so the next preparation reads it back instead of
        // scanning it (the inline closure's gate). `working` keeps it raw.
        let stored = |idx: usize| v(idx).map(|value| crate::composition::closure::persisted_data(&value));
        let result = match verb {
            "set_frontmatter" => engine.set_frontmatter(&s(0)?, &s(1)?, stored(2)?),
            "merge_frontmatter" => engine.merge_frontmatter(&s(0)?, stored(1)?),
            "delete_frontmatter" => engine.delete_frontmatter(&s(0)?, &s(1)?),
            "increment_frontmatter" => engine.increment_frontmatter(&s(0)?, &s(1)?),
            "decrement_frontmatter" => engine.decrement_frontmatter(&s(0)?, &s(1)?),
            "append_frontmatter" => engine.append_frontmatter(&s(0)?, &s(1)?, stored(2)?),
            "prepend_frontmatter" => engine.prepend_frontmatter(&s(0)?, &s(1)?, stored(2)?),
            "ensure_file" => {
                if values.len() >= 2 {
                    engine.ensure_file_with_content(&path(0)?, &s(1)?).map(Value::String)
                } else {
                    engine.ensure_file(&path(0)?).map(Value::String)
                }
            }
            "ensure_dir" => engine.ensure_dir(&path(0)?).map(Value::String),
            "append_line" => engine.append_line(&path(0)?, &s(1)?).map(Value::String),
            "append_jsonl" => engine.append_jsonl(&path(0)?, v(1)?).map(Value::String),
            "http_post" => engine.http_post(&s(0)?, s(1)?.into_bytes()),
            other => return Err(dispatch_err(format!("unknown side effect `{other}`"))),
        };
        // The effect engine's error is the §D9 snapshot boundary's input: hand
        // it over typed so the projection reads it, rather than pre-flattening
        // it here and handing the snapshot prose.
        let out = result.map_err(|error| {
            ActionFailure::Dispatch(LifecycleErrorInfo::from_error_or_action(verb, &error))
        })?;
        self.mirror_frontmatter_mutation(verb, &values, working);
        Ok(out)
    }

    /// Resolve against the pre-write snapshot; an absent destination is a
    /// declared null, so optional values can be copied before being reset.
    ///
    /// Every value reads the same pre-write snapshot, and no destination is
    /// written unless every value succeeds. Expression values evaluate first;
    /// whole-value `$( … )` values run afterwards, together, through the
    /// shell runner, so a failed expression runs no command. A command's
    /// external effects are not undone when a later value fails.
    fn dispatch_runtime_set(
        &self,
        set: &RuntimeSet,
        property: &str,
        working: &mut Map<String, Value>,
    ) -> Result<IndexMap<String, Value>, ActionFailure> {
        let mut snapshot = working.clone();
        for (key, _) in set.iter() {
            snapshot.entry(key.clone()).or_insert(Value::Null);
        }
        let mut resolved_values = HashMap::with_capacity(set.len());
        for (key, value) in set.iter() {
            if matches!(value, WithValue::Shell(_)) {
                continue;
            }
            let resolved = self.resolve_with_value(value, &snapshot).map_err(|(suffix, error)| {
                let value_property = format!("{property}.{key}{suffix}");
                let reason = LifecycleEvaluationReason::Expression;
                let cause = LifecycleCause::new(error);
                let diagnostic = CompositionError::LifecycleEvaluationError {
                    source_path: self.source_path.to_path_buf(),
                    event: self.signal.property_name().to_string(),
                    surface: "set".to_string(),
                    message: cause.to_string(),
                    property: Some(value_property.clone()),
                    reason: reason.clone(),
                    cause: Some(cause),
                };
                let mut info = LifecycleErrorInfo::from_composition_error(&diagnostic)
                    .at_property(value_property);
                info.variant = "set".to_string();
                info.reason = reason;
                ActionFailure::Evaluation(info)
            })?;
            resolved_values.insert(key.clone(), resolved);
        }
        if set.has_shell_values() {
            resolved_values.extend(self.run_set_shell_values(set, property, &snapshot)?);
        }
        let updates: IndexMap<String, Value> = set
            .iter()
            .map(|(key, _)| {
                let value = resolved_values
                    .remove(key)
                    .expect("every set destination resolved to a value");
                (key.clone(), value)
            })
            .collect();

        let fallback;
        let state = match self.runtime_state {
            Some(state) => state,
            None => {
                fallback = super::super::runtime_state::RuntimeState::new();
                &fallback
            }
        };
        let prior = state
            .set_batch(self.effect_engine, &updates, &snapshot)
            .map_err(|error| {
                ActionFailure::Dispatch(LifecycleErrorInfo::from_error_or_action("set", &error))
            })?;
        for (key, value) in updates {
            working.insert(key, value);
        }
        Ok(prior)
    }

    /// Run a `set` action's whole-value `$( … )` values against the pre-write
    /// `snapshot`.
    ///
    /// A value preflight never resolved does not run, nor does one approved in
    /// neither mode. A value approved by its shape runs its template, its
    /// placeholders read from `snapshot` with the event's globals; a refusal of
    /// its value, blacklisted fill, or program is an evaluation error. A runner that prohibits
    /// shell execution, and a user interruption, are evaluation errors that
    /// `no_error` cannot suppress; a command that fails is a dispatch error.
    fn run_set_shell_values(
        &self,
        set: &RuntimeSet,
        property: &str,
        snapshot: &Map<String, Value>,
    ) -> Result<Vec<(String, Value)>, ActionFailure> {
        let mut values: Vec<std::borrow::Cow<'_, ResolvedShellValue>> = Vec::new();
        let mut authored = String::new();
        let mut policy_root = None;
        for (key, shell) in set.shell_values() {
            let Some(resolved) = shell.resolved.as_ref() else {
                return Err(ActionFailure::Evaluation(
                    LifecycleErrorInfo::from_action_failure(
                        "set",
                        format!(
                            "`{property}.{key}` runs a command that pre-flight never resolved or \
                             approved, so it cannot run"
                        ),
                    )
                    .at_property(format!("{property}.{key}")),
                ));
            };
            authored.push_str(&shell.authored);
            // A value approved by its shape runs its template; one approved in
            // neither mode never runs.
            match shell.template.as_ref().map(|site| (site, site.mode())) {
                None | Some((_, Some(super::shell_template::LifecycleShellMode::ResolvedBytes))) => {
                    values.push(std::borrow::Cow::Borrowed(resolved));
                }
                Some((site, Some(super::shell_template::LifecycleShellMode::Template { policy_root: root }))) => {
                    self.ensure_shell_permitted("set")?;
                    policy_root = root.or(policy_root);
                    let value = resolved.clone().with_template(site.template().clone());
                    values.push(std::borrow::Cow::Owned(match site.late_values() {
                        Some(late) => value.with_late_values(late.clone()),
                        None => value,
                    }));
                }
                Some((site, None)) => {
                    self.ensure_shell_permitted("set")?;
                    return Err(self.not_approved("set", &format!("{property}.{key}"), &site.template().authored));
                }
            }
        }
        let values: Vec<&ResolvedShellValue> = values.iter().map(std::convert::AsRef::as_ref).collect();
        let run = |holes: Option<&dyn HoleEvaluator>| {
            let request = ShellValueRequest {
                values: &values,
                state: snapshot,
                source_path: self.source_path,
                context: self.early_binding_context(&authored),
                file_resolution_context: self.file_resolution_context,
                holes,
                policy_root: policy_root.as_deref(),
            };
            self.shell_runner.run_values(&request)
        };
        // Placeholders read the pre-write snapshot, as every other value does.
        let templated = values.iter().any(|value| value.template().is_some());
        let outcome = if templated {
            self.with_hole_evaluator(snapshot, &authored, |holes| run(Some(holes)))
                .map_err(|error| ActionFailure::Evaluation(LifecycleErrorInfo::from_expr_error("set", error)))?
        } else {
            run(None)
        };
        match outcome {
            Ok(results) if !super::cancellation::requested() => Ok(results),
            Ok(_) => Err(ActionFailure::Evaluation(
                LifecycleErrorInfo::from_action_failure(
                    "set",
                    "the run was interrupted while a `set` command ran",
                )
                .at_property(property.to_string()),
            )),
            Err(error @ ShellRunError::BeforePreflight) => Err(ActionFailure::Evaluation(
                LifecycleErrorInfo::from_error_or_action("set", &error).at_property(property.to_string()),
            )),
            // A template value refused before it ran (missing or rejected
            // value, blacklisted fill, interpreter target) is an input error,
            // never a command failure `no_error` may absorb.
            Err(ShellRunError::Value { source }) if templated && template_input_refusal(&source) => {
                let command = source.command().unwrap_or_default().to_string();
                Err(self.template_refusal(property, &command, source))
            }
            Err(error) if super::cancellation::requested() => Err(ActionFailure::Evaluation(
                LifecycleErrorInfo::from_error_or_action("set", &error).at_property(property.to_string()),
            )),
            Err(error) => Err(ActionFailure::Dispatch(
                LifecycleErrorInfo::from_error_or_action("set", &error).at_property(property.to_string()),
            )),
        }
    }

    /// Resolve a document-authored mutation target through the same captured
    /// file-reference context used by composition.
    ///
    /// Existing targets follow ordinary document resolution precedence. For a
    /// missing implicit-relative target, creation stays anchored to the
    /// mutation root, preserving the effect engine's existing relative
    /// mutation policy; other reference kinds use their first document-scoped
    /// candidate. A later transclusion therefore resolves the same identity.
    fn resolve_effect_path(&self, verb: &str, raw: &str) -> Result<String, ActionFailure> {
        let request_context = self.file_resolution_context;
        let reference = FileReference::new(raw).map_err(|error| {
            ActionFailure::Dispatch(LifecycleErrorInfo::from_error_or_action(verb, &error))
        })?;
        let document_context = request_context.for_source(self.source_path);
        let mutation_context = request_context.for_cwd(self.effect_engine.mutation_root());
        let resolved = reference
            .resolve_in_context(&document_context)
            .map_err(|error| {
                ActionFailure::Dispatch(LifecycleErrorInfo::from_error_or_action(verb, &error))
            })?;
        let path = match resolved {
            Some(path) => path,
            None => reference
                .candidate_plan(if reference.class().kind == FileReferenceKind::ImplicitRelative {
                    &mutation_context
                } else {
                    &document_context
                })
                .map_err(|error| {
                    ActionFailure::Dispatch(LifecycleErrorInfo::from_error_or_action(verb, &error))
                })?
                .into_iter()
                .next()
                .map(|candidate| candidate.path().to_path_buf())
                .ok_or_else(|| {
                    ActionFailure::Dispatch(LifecycleErrorInfo::from_action_failure(
                        verb,
                        format!("file reference `{raw}` has no local mutation target"),
                    ))
                })?,
        };
        Ok(path.to_string_lossy().into_owned())
    }

    /// Mirror a successful frontmatter-verb mutation onto the in-memory
    /// `working` document state when it targets the document being processed.
    ///
    /// The side-effect engine writes to disk; this keeps the executor's live
    /// view consistent so a later action in the same stack resolves `{{ … }}`
    /// against the mutated value (just-in-time resolution). Mutations targeting
    /// any other file are not mirrored — they do not change this document.
    fn mirror_frontmatter_mutation(
        &self,
        verb: &str,
        values: &[Value],
        working: &mut Map<String, Value>,
    ) {
        let Some(Value::String(target)) = values.first() else {
            return;
        };
        if !self.targets_document(target) {
            return;
        }
        let prop = || values.get(1).and_then(Value::as_str);
        match verb {
            "set_frontmatter" => {
                if let (Some(p), Some(val)) = (prop(), values.get(2)) {
                    working.insert(p.to_string(), val.clone());
                }
            }
            "merge_frontmatter" => {
                if let Some(Value::Object(obj)) = values.get(1) {
                    for (k, v) in obj {
                        working.insert(k.clone(), v.clone());
                    }
                }
            }
            "delete_frontmatter" => {
                if let Some(p) = prop() {
                    working.remove(p);
                }
            }
            "increment_frontmatter" | "decrement_frontmatter" => {
                if let Some(p) = prop() {
                    let delta = if verb == "increment_frontmatter" { 1 } else { -1 };
                    let next = working.get(p).and_then(Value::as_i64).unwrap_or(0) + delta;
                    working.insert(p.to_string(), Value::from(next));
                }
            }
            "append_frontmatter" | "prepend_frontmatter" => {
                if let (Some(p), Some(val)) = (prop(), values.get(2)) {
                    let entry = working.entry(p.to_string()).or_insert_with(|| Value::Array(Vec::new()));
                    if let Value::Array(arr) = entry {
                        if verb == "append_frontmatter" {
                            arr.push(val.clone());
                        } else {
                            arr.insert(0, val.clone());
                        }
                    }
                }
            }
            _ => {}
        }
    }

    /// Whether a side-effect target path refers to the document being processed.
    ///
    /// Resolves both the target and the document source against the effect
    /// engine's mutation root (relative paths join it; absolute paths win) and
    /// compares them lexically.
    fn targets_document(&self, target: &str) -> bool {
        let root = self.effect_engine.mutation_root();
        let resolve = |raw: &Path| -> PathBuf {
            if raw.is_absolute() {
                lexical_normalize(raw)
            } else {
                lexical_normalize(&root.join(raw))
            }
        };
        let target_clean = target.trim().trim_matches('"').trim_matches('\'');
        resolve(Path::new(target_clean)) == resolve(self.source_path)
    }

    /// Invoke a read-only expression function for its result, logging it in
    /// the lifecycle/status style.
    fn invoke_expression_function(
        &self,
        function: &str,
        args: &[Expr],
        fm: &Map<String, Value>,
    ) -> Result<Value, LifecycleExprError> {
        let call = Expr::FunctionCall {
            name: function.to_string(),
            args: args.to_vec(),
        };
        let value = self.eval_expr(&call, fm)?;
        warn!(
            function,
            result = %scalar_string(&value),
            "lifecycle expression-function action evaluated"
        );
        Ok(value)
    }

    /// Resolve a parse-time [`LifecycleControlAction`] into a runtime
    /// [`StackControl`] by evaluating its expression arguments against `fm`.
    ///
    /// `fm` is the live intra-stack frontmatter, so a preceding
    /// `set_frontmatter` in the same stack is visible to every argument —
    /// including a `proxy.with` value.
    fn resolve_control(
        &self,
        control: &LifecycleControlAction,
        location: ActionLocation,
        fm: &Map<String, Value>,
    ) -> Result<StackControl, LifecycleErrorInfo> {
        use LifecycleControlAction as C;
        let verb = control.verb();
        let classify = |error: LifecycleExprError| LifecycleErrorInfo::from_expr_error(verb, error);
        Ok(match control {
            C::Stop => StackControl::Stop,
            C::Skip => StackControl::Skip,
            C::Error { reason } => StackControl::Error {
                reason: self.eval_opt_string(reason.as_ref(), fm).map_err(classify)?,
            },
            C::Proxy { target, with } => {
                // Atomicity: the target and the *complete* overlay resolve
                // before this returns, and this returns before anything acts on
                // the handoff. A failure in either leaves the source active with
                // no partial overlay installed and the target untouched.
                let target = self.render_message(target, fm).map_err(classify)?;
                let overlay = self.resolve_with_overlay(with, &target, &location, fm)?;
                StackControl::Proxy {
                    target,
                    overlay,
                    location,
                }
            }
            // The overlay resolves here, before the directive is returned, so an
            // evaluation failure leaves no retry for the runtime to dispatch:
            // no transition is taken and no attempt budget is spent.
            C::Retry {
                max_attempts,
                backoff,
                delay,
                with,
            } => StackControl::Retry {
                max_attempts: self
                    .eval_opt_u32(max_attempts.as_ref(), fm)
                    .map_err(classify)?
                    .unwrap_or(1),
                backoff: backoff.unwrap_or(RetryBackoff::Fixed),
                delay: self
                    .eval_opt_string(delay.as_ref(), fm)
                    .map_err(classify)?
                    .unwrap_or_else(|| "0s".to_string()),
                // A retry's target is the fresh re-read of this same document.
                overlay: self.resolve_with_overlay(
                    with,
                    &biscuit_file::to_portable_string(self.source_path),
                    &location,
                    fm,
                )?,
            },
            C::Resume {
                message,
                max_attempts,
            } => StackControl::Resume {
                message: self.render_message(message, fm).map_err(classify)?,
                max_attempts: self
                    .eval_opt_u32(max_attempts.as_ref(), fm)
                    .map_err(classify)?
                    .unwrap_or(1),
            },
            C::Defer { delay, reason } => StackControl::Defer {
                delay: self.render_message(delay, fm).map_err(classify)?,
                reason: self.eval_opt_string(reason.as_ref(), fm).map_err(classify)?,
            },
        })
    }

    /// Evaluate a whole `proxy.with` or `retry.with` mapping into the typed
    /// overlay the target will receive.
    ///
    /// `target` names the document the overlay is built for in a diagnostic:
    /// the proxy target, or the current document for `retry`. `location` is the
    /// authored directive, whose last path segment names the verb.
    ///
    /// Every value goes through the same DM2 subtree composition the rest of
    /// the lifecycle surface uses, so `with:` adds no second interpolation
    /// grammar. The first failing value aborts the whole mapping.
    fn resolve_with_overlay(
        &self,
        with: &WithOverlay,
        target: &str,
        location: &ActionLocation,
        fm: &Map<String, Value>,
    ) -> Result<IndexMap<String, Value>, LifecycleErrorInfo> {
        // Capture the snapshot-less fallback context ONCE for the whole overlay,
        // then walk every leaf against it. Recursive `resolve_with_value`
        // evaluates each scalar leaf independently; without this a
        // `prepared_context: None` caller re-runs a demand-driven capture per
        // leaf. A prepared caller already reuses its single snapshot.
        match self.prepared_context {
            Some(_) => self.walk_with_overlay(self, with, target, location, fm),
            None => {
                let base = self
                    .ctx_base_dir
                    .or(self.base_dir)
                    .unwrap_or_else(|| Path::new("."));
                let content = proxy_with_scan_content(with);
                let fallback = capture_proxy_with_fallback(base, &content);
                let walker = self.with_prepared_context(&fallback);
                self.walk_with_overlay(&walker, with, target, location, fm)
            }
        }
    }

    /// Walk `with`'s leaves through `walker`, rooting a diagnostic at the exact
    /// nested property on failure.
    fn walk_with_overlay(
        &self,
        walker: &StackExecutionContext<'_>,
        with: &WithOverlay,
        target: &str,
        location: &ActionLocation,
        fm: &Map<String, Value>,
    ) -> Result<IndexMap<String, Value>, LifecycleErrorInfo> {
        let mut overlay = IndexMap::with_capacity(with.len());
        for (key, value) in with.iter() {
            let resolved = walker
                .resolve_with_value(value, fm)
                .and_then(|resolved| reject_control_plane_template(key, resolved))
                .map_err(|(suffix, error)| {
                    let cause = LifecycleCause::new(error);
                    let err = CompositionError::LifecycleWithEvaluationFailed {
                        source_path: self.source_path.to_path_buf(),
                        property: location.to_string(),
                        path: format!("with.{key}{suffix}"),
                        target: target.to_string(),
                        message: cause.to_string(),
                        cause: Some(cause),
                    };
                    LifecycleErrorInfo::from_composition_error(&err)
                })?;
            overlay.insert(key.clone(), resolved);
        }
        Ok(overlay)
    }

    /// Resolve one `with:` value tree.
    ///
    /// On failure returns the path suffix *below* the overlay key (e.g.
    /// `.metadata.area`, `[2]`) alongside the reason, so the caller can root a
    /// diagnostic at the exact nested property without the value ever being
    /// echoed.
    fn resolve_with_value(
        &self,
        value: &WithValue,
        fm: &Map<String, Value>,
    ) -> Result<Value, (String, LifecycleExprError)> {
        match value {
            WithValue::Null => Ok(Value::Null),
            // Only a `set` mapping holds one, and `set` runs it separately.
            WithValue::Shell(shell) => Err((
                String::new(),
                LifecycleExprError::prose(format!(
                    "`{}` is a shell value, which only a `set` action runs",
                    shell.authored
                )),
            )),
            WithValue::Scalar(expr) => self
                .resolve_typed_value(expr, fm)
                .map_err(|msg| (String::new(), msg)),
            WithValue::Array(items) => items
                .iter()
                .enumerate()
                .map(|(i, item)| {
                    self.resolve_with_value(item, fm)
                        .map_err(|(suffix, msg)| (format!("[{i}]{suffix}"), msg))
                })
                .collect::<Result<Vec<_>, _>>()
                .map(Value::Array),
            WithValue::Object(map) => map
                .iter()
                .map(|(k, v)| {
                    self.resolve_with_value(v, fm)
                        .map(|resolved| (k.clone(), resolved))
                        .map_err(|(suffix, msg)| (format!(".{k}{suffix}"), msg))
                })
                .collect::<Result<Map<_, _>, _>>()
                .map(Value::Object),
        }
    }

    /// Evaluate one expression at event-time, **preserving its type**.
    ///
    /// The type-collapsing counterpart of [`Self::render_message`], which folds
    /// everything to a display string via `scalar_string` — the path that
    /// reduces `proxy.target` to text. A whole-value span keeps its resolved
    /// `bool`/number/array/object/null; a mixed string interpolates through DM2
    /// and stays a string.
    ///
    /// An absent document property resolves to `null`. The resolved value is
    /// data: a `{{ … }}` inside it — at any depth of a container — reaches a
    /// `set:` or a `proxy.with:` target as text, never as a template.
    fn resolve_typed_value(
        &self,
        expr: &Expr,
        fm: &Map<String, Value>,
    ) -> Result<Value, LifecycleExprError> {
        self.evaluate_operand(expr, fm)
    }

    /// Evaluate an optional expression to a display string.
    fn eval_opt_string(
        &self,
        expr: Option<&Expr>,
        fm: &Map<String, Value>,
    ) -> Result<Option<String>, LifecycleExprError> {
        match expr {
            Some(expr) => Ok(Some(self.render_message(expr, fm)?)),
            None => Ok(None),
        }
    }

    /// Evaluate an optional expression to a non-negative integer count.
    fn eval_opt_u32(
        &self,
        expr: Option<&Expr>,
        fm: &Map<String, Value>,
    ) -> Result<Option<u32>, LifecycleExprError> {
        let Some(expr) = expr else {
            return Ok(None);
        };
        let value = self.eval_expr(expr, fm)?;
        let n = value
            .as_f64()
            .ok_or_else(|| LifecycleExprError::prose(format!("expected a number, got {value}")))?;
        if n < 0.0 || n.fract() != 0.0 {
            return Err(LifecycleExprError::prose(format!(
                "expected a non-negative whole number, got {n}"
            )));
        }
        Ok(Some(n as u32))
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

/// Combined `ctx.*` scan content across a whole `with:` overlay, so a single
/// fallback [`ComposeContext`] capture covers every leaf.
///
/// A mixed-interpolation leaf is an [`Expr::StringLiteral`] whose raw text still
/// holds its `{{ … }}` spans, so that text is scanned directly; a whole-value
/// span is a parsed [`Expr`], scanned as its rendering. The union feeds
/// [`ComposeContext::capture_for_content`], which stays datetime-only when no
/// leaf references `ctx.*`.
fn proxy_with_scan_content(with: &WithOverlay) -> String {
    let mut content = String::new();
    for (_, value) in with.iter() {
        push_proxy_with_scan_content(value, &mut content);
    }
    content
}

/// Recursively append one `with:` value's scan content to `content`.
fn push_proxy_with_scan_content(value: &WithValue, content: &mut String) {
    match value {
        WithValue::Null => {}
        WithValue::Shell(shell) => {
            content.push(' ');
            content.push_str(&shell.authored);
        }
        WithValue::Scalar(Expr::StringLiteral(s)) => {
            content.push(' ');
            content.push_str(s);
        }
        WithValue::Scalar(expr) => {
            content.push(' ');
            content.push_str(&expr.to_string());
        }
        WithValue::Array(items) => {
            for item in items {
                push_proxy_with_scan_content(item, content);
            }
        }
        WithValue::Object(map) => {
            for (_, value) in map {
                push_proxy_with_scan_content(value, content);
            }
        }
    }
}

/// Whether an action operand is a template, interpolated once by
/// [`interpolate_text_in`], rather than an expression evaluated by
/// [`evaluate_expr_in`]: an authored literal body carrying `{{ … }}` spans.
pub(super) fn is_template_operand(expr: &Expr) -> bool {
    matches!(expr, Expr::StringLiteral(text) if text.contains("{{"))
}

/// Evaluate one action operand against an established state, as lifecycle
/// playback does: a template is interpolated, every other expression yields
/// data that is never scanned again.
///
/// Shared by event-time playback and pre-warm's speculative resolution, so
/// both produce the same text from the same inputs.
pub(super) fn evaluate_operand_in(
    expr: &Expr,
    state: &EffectiveState,
    view: Arc<BindingView>,
    globals: HashMap<String, RuntimeBinding<'static>>,
    resolution_context: Option<ResolutionContext>,
) -> Result<Value, LifecycleExprError> {
    match expr {
        Expr::StringLiteral(text) if is_template_operand(expr) => {
            interpolate_text_in(text, state, view, globals, resolution_context)
        }
        _ => evaluate_expr_in(expr, state, view, globals, resolution_context),
    }
}

/// The origin a lifecycle template diagnostic names: the authored property.
fn template_origin(property: &str) -> darkmatter::markdown::compose::ShellCommandOrigin {
    darkmatter::markdown::compose::ShellCommandOrigin::Frontmatter {
        key: property.to_string(),
        line: None,
    }
}

/// Whether a `set` value's failure refused a template command before it ran.
fn template_input_refusal(error: &ShellExpansionError) -> bool {
    matches!(
        error,
        ShellExpansionError::MissingValue { .. }
            | ShellExpansionError::TemplateValueRejected { .. }
            | ShellExpansionError::TemplateGrammar { .. }
            | ShellExpansionError::TemplateTarget { .. }
            | ShellExpansionError::Blacklisted { .. }
    )
}

fn evaluate_expr_in(
    expr: &Expr,
    state: &EffectiveState,
    view: Arc<BindingView>,
    globals: HashMap<String, RuntimeBinding<'static>>,
    resolution_context: Option<ResolutionContext>,
) -> Result<Value, LifecycleExprError> {
    let lookup = layered_session(state, globals, Some(view), resolution_context).map_err(|error| {
        LifecycleExprError::Evaluate(Box::new(ExpressionError::Binding(Box::new(error))))
    })?;
    evaluate(expr, &lookup).map_err(|error| LifecycleExprError::Evaluate(Box::new(error)))
}

fn interpolate_text_in(
    text: &str,
    state: &EffectiveState,
    view: Arc<BindingView>,
    globals: HashMap<String, RuntimeBinding<'static>>,
    resolution_context: Option<ResolutionContext>,
) -> Result<Value, LifecycleExprError> {
    let value = Value::String(text.to_string());
    let compose = SubtreeCompose::new(&value, state)
        .with_globals(globals)
        .with_binding_view(view);
    let compose = match resolution_context {
        Some(context) => compose.with_resolution_context(context),
        None => compose,
    };
    compose
        .compose()
        .map_err(|error| LifecycleExprError::Compose(Box::new(error)))
}

/// Refuse template syntax in a `with:` value that becomes lifecycle
/// configuration in the target.
///
/// Every other overlay key reaches the target as a data layer, so a `{{ … }}`
/// in it is text. A lifecycle key (`success:`, `loop:`, …) is different: the
/// target reparses it as its own stack or loop, and Claudine evaluates those
/// strings at event time, outside Darkmatter compose. Run-time data carrying
/// template syntax there would become an instruction, so it fails closed. On
/// failure returns the path suffix below the key, as
/// [`StackExecutionContext::resolve_with_value`] does.
fn reject_control_plane_template(
    key: &str,
    resolved: Value,
) -> Result<Value, (String, LifecycleExprError)> {
    if !super::LIFECYCLE_EVENT_KEYS.contains(&key) {
        return Ok(resolved);
    }
    match first_template_span(&resolved, &mut String::new()) {
        None => Ok(resolved),
        Some((suffix, span)) => Err((
            suffix,
            LifecycleExprError::prose(format!(
                "the value holds `{span}`, but `{key}` becomes the target's lifecycle \
                 configuration, where template syntax would run as an instruction"
            )),
        )),
    }
}

/// The path suffix and text of the first `{{ … }}` span anywhere in `value`.
fn first_template_span(value: &Value, suffix: &mut String) -> Option<(String, String)> {
    match value {
        Value::String(text) => ExpressionFinder::find_all_plain(text)
            .first()
            .map(|span| (suffix.clone(), text[span.start..span.end].to_string())),
        Value::Array(items) => items.iter().enumerate().find_map(|(index, item)| {
            let mark = suffix.len();
            suffix.push_str(&format!("[{index}]"));
            let found = first_template_span(item, suffix);
            suffix.truncate(mark);
            found
        }),
        Value::Object(map) => map.iter().find_map(|(key, item)| {
            let mark = suffix.len();
            suffix.push_str(&format!(".{key}"));
            let found = first_template_span(item, suffix);
            suffix.truncate(mark);
            found
        }),
        _ => None,
    }
}

/// Lexically normalize a path (resolve `.`/`..` components) without touching the
/// filesystem, so two paths that name the same document compare equal.
fn lexical_normalize(path: &Path) -> PathBuf {
    use std::path::Component;
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            Component::ParentDir => {
                out.pop();
            }
            Component::CurDir => {}
            other => out.push(other.as_os_str()),
        }
    }
    out
}


#[cfg(test)]
mod tests;
