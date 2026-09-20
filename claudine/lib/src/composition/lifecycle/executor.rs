//! Lifecycle stack execution engine.
//!
//! Given a parsed [`LifecycleConfig`] and a [`LifecycleSignal`], this module
//! runs one lifecycle event: it emits the top-level communication properties
//! first, then processes the typed `stack:` top to bottom. Each stack item's
//! `when:` clause is evaluated against the lifecycle execution context; when
//! it matches, the item's actions run in order until a lifecycle control
//! action terminates the event.
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

use biscuit_file::{FileReference, FileReferenceKind};
use biscuit_terminal::terminal::Terminal;
use darkmatter::effects::EffectEngine;
use darkmatter::markdown::compose::expression::{
    Expr, ExpressionFinder, ResolutionContext, evaluate, is_truthy, scalar_string,
};
use darkmatter::markdown::compose::subtree::{InjectedGlobal, LayeredLookup, SubtreeCompose};
use darkmatter::markdown::compose::{
    ComposeContext, CurrentAuthority, EffectiveState, EffectiveStateBuilder,
};
use indexmap::IndexMap;
use serde_json::{Map, Value};
use tracing::warn;

use super::super::error::{CompositionError, LifecycleEvaluationReason};
use super::{
    LifecycleConfig, LifecycleEmitter, LifecycleNotification, LifecycleSignal, audio_phases,
    first_undefined_stack_variable, tts_config_from_settings,
};
use super::actions::{
    CommunicationChannel, LifecycleAction, LifecycleActionKind, LifecycleControlAction, ProxyWith,
    ProxyWithValue, RetryBackoff, RuntimeSet, is_known_side_effect,
};
use crate::composition::coordinator::ActionLocation;
use super::context::{
    LifecycleErrorInfo, LifecycleTiming, lifecycle_injected_globals,
};
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
    /// error in hand: an undefined-variable rejection or a control argument of
    /// the wrong shape.
    #[error("{0}")]
    Prose(String),

    /// The post-DM2 leak guard: resolution finished, but a `{{ … }}` span
    /// survived in the rendered text.
    #[error(
        "the rendered text still contains `{span}` after every interpolation pass; a \
         `{{{{ … }}}}` inside a quoted string literal is text and is never interpolated on \
         this surface, and a frontmatter value that holds template syntax is not \
         re-expanded at event time"
    )]
    SurvivingSpan {
        /// The first surviving span, braces included.
        span: String,
    },
}

impl LifecycleExprError {
    /// Build a [`Self::Prose`] arm from anything string-shaped.
    fn prose(message: impl Into<String>) -> Self {
        LifecycleExprError::Prose(message.into())
    }
}

/// A shell command refused by the lifecycle boundary or unable to start.
/// A command that ran and exited nonzero is reported through
/// [`ShellRunner::run`]'s `Ok` arm. Spawn failures retain their I/O source.
#[derive(Debug, thiserror::Error)]
pub enum ShellRunError {
    /// The lifecycle has not crossed the preflight boundary.
    #[error("shell commands are forbidden during initialize and before preflight completes; move the command to start or a later event")]
    BeforePreflight,
    /// The shell process could not be spawned or waited on.
    #[error("command `{command}` failed to run: {source}")]
    Spawn {
        /// The approved command that was attempted.
        command: String,
        /// The underlying spawn/wait failure.
        #[source]
        source: std::io::Error,
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
}

/// Runner for initialization and its early catch handlers. Approvals cannot
/// enable shell execution on this route.
#[derive(Debug, Default, Clone, Copy)]
pub struct DisabledShellRunner;

impl ShellRunner for DisabledShellRunner {
    fn run(&self, _command: &str) -> Result<i32, ShellRunError> {
        Err(ShellRunError::BeforePreflight)
    }
}

/// Production [`ShellRunner`] that runs commands through the system shell.
#[derive(Debug, Default, Clone, Copy)]
pub struct SystemShellRunner;

impl ShellRunner for SystemShellRunner {
    fn run(&self, command: &str) -> Result<i32, ShellRunError> {
        let mut cmd = system_shell_command(command).map_err(|error| ShellRunError::Spawn {
            command: command.to_string(),
            source: std::io::Error::other(error),
        })?;
        let status = cmd.status().map_err(|source| ShellRunError::Spawn {
            command: command.to_string(),
            source,
        })?;
        Ok(status.code().unwrap_or(-1))
    }
}

/// Build the platform `Command` that runs `command` through the system shell.
#[cfg(windows)]
fn system_shell_command(
    command: &str,
) -> Result<std::process::Command, crate::child_environment::ChildEnvironmentError> {
    use std::os::windows::process::CommandExt;

    let mut cmd = crate::child_environment::command("cmd")?;
    // `cmd.exe` owns the command-tail grammar. Passing it through the Windows
    // argv encoder changes nested quotes before the shell can parse them.
    cmd.arg("/D").arg("/C").raw_arg(command);
    Ok(cmd)
}

/// Build the platform `Command` that runs `command` through the system shell.
#[cfg(not(windows))]
fn system_shell_command(
    command: &str,
) -> Result<std::process::Command, crate::child_environment::ChildEnvironmentError> {
    let mut cmd = crate::child_environment::command("sh")?;
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
    /// The invocation's refresh authority for Darkmatter's lazy `current` /
    /// `current_env` roots.
    ///
    /// `None` fails closed: every `current.<key>` renders `null` and records a
    /// `PartialRuntimeCapture` diagnostic rather than probing the host. Only a
    /// caller holding launch evidence supplies one.
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
    /// Immutable request snapshot used by document-authored file expressions.
    pub file_resolution_context: Option<&'a biscuit_file::FileResolutionContext>,
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

/// Which authored container a running stack came from, for the property paths
/// its diagnostics report.
///
/// An event's items are reached through `{signal}.stack[i]`; a task's
/// `setup:`/`teardown:` value *is* the list, so its items are `{root}[i]` with
/// no `stack` segment. That spelling is the only difference between the two, so
/// they share one loop and this is where it lives. Carried as a call argument
/// rather than on [`StackExecutionContext`] because one context runs both kinds
/// of stack, and as a borrowed root rather than on [`ActionLocation`] because
/// that type is the owned, `Copy` proxy-provenance identity.
#[derive(Debug, Clone, Copy)]
enum StackRoot<'a> {
    /// A lifecycle event block's `stack:`.
    Event,
    /// A task's `setup:`/`teardown:` list, at its source-rooted property.
    Task(&'a str),
}

impl StackRoot<'_> {
    /// The property of the `when:` guard on the `index`-th item.
    fn when_property(&self, signal: LifecycleSignal, index: usize) -> String {
        match self {
            Self::Event => format!("{}.stack[{index}].when", signal.property_name()),
            Self::Task(root) => format!("{root}[{index}].when"),
        }
    }

    /// The property of the action `location` names.
    fn action_property(&self, location: ActionLocation) -> String {
        match self {
            Self::Event => location.to_string(),
            Self::Task(root) => format!(
                "{root}[{}].action[{}]",
                location.stack_index(),
                location.action_index()
            ),
        }
    }
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

impl ActionFailure {
    /// Borrow the underlying error snapshot regardless of layer.
    fn info(&self) -> &LifecycleErrorInfo {
        match self {
            ActionFailure::Evaluation(info) | ActionFailure::Dispatch(info) => info,
        }
    }
}

impl StackExecutionContext<'_> {
    /// Run the event: top-level communication first, then the stack.
    pub fn execute_event(&self, config: &LifecycleConfig) -> LifecycleEventOutcome {
        if let Some(notification) = config.get(self.signal) {
            // Top-level emission fails closed: a resolution failure halts the
            // event before its stack runs, mirroring a stack-action failure
            // (C4). An interpolation raise is an evaluation error; an invalid
            // resolved effect name is a dispatch failure.
            if let Err(failure) = self.emit_top_level(notification) {
                let info = failure.info();
                warn!(
                    signal = ?self.signal,
                    kind = info.kind,
                    variant = %info.variant,
                    message = %info.msg,
                    "lifecycle top-level emission failed closed"
                );
                return match failure {
                    ActionFailure::Evaluation(info) => LifecycleEventOutcome {
                        evaluation_error: Some(info),
                        ..Default::default()
                    },
                    ActionFailure::Dispatch(info) => LifecycleEventOutcome {
                        action_error: Some(info),
                        ..Default::default()
                    },
                };
            }
        }
        self.execute_stack_for_signal(config)
    }

    /// Run only the stack for this event's signal, without emitting the
    /// top-level communication properties.
    ///
    /// Used by [`LifecycleRunGuard`] when it owns top-level emission and
    /// only needs the stack actions evaluated.
    pub fn execute_stack_for_signal(&self, config: &LifecycleConfig) -> LifecycleEventOutcome {
        match config.stack(self.signal) {
            Some(items) if !items.is_empty() => self.execute_stack(items, StackRoot::Event),
            _ => LifecycleEventOutcome::default(),
        }
    }

    /// Emit only the top-level communication properties for this event's
    /// signal, without running the stack.
    ///
    /// The counterpart to [`Self::execute_stack_for_signal`]: a caller that has
    /// already run a stack once (e.g. to inspect its [`StackControl`] before
    /// committing to a terminal signal) uses this to fire the communication
    /// surface without re-running the stack's side effects.
    ///
    /// Returns the late-binding **evaluation** error (a raised top-level
    /// interpolation) when one occurs, so a terminal-phase caller can halt the
    /// run on it. A side-effect **dispatch** failure (an invalid resolved
    /// `effect` name) is logged and skipped — the terminal slot is already
    /// taken, so it follows the terminal-phase log-and-continue policy and is
    /// not returned.
    pub fn emit_top_level_for_signal(
        &self,
        config: &LifecycleConfig,
    ) -> Option<LifecycleErrorInfo> {
        if let Some(notification) = config.get(self.signal) {
            if let Err(failure) = self.emit_top_level(notification) {
                let info = failure.info();
                warn!(
                    signal = ?self.signal,
                    kind = info.kind,
                    variant = %info.variant,
                    message = %info.msg,
                    "lifecycle top-level emission failed; field skipped"
                );
                return match failure {
                    ActionFailure::Evaluation(info) => Some(info),
                    ActionFailure::Dispatch(_) => None,
                };
            }
        }
        None
    }

    /// Run one already-parsed action stack — a task's `setup:` or `teardown:`.
    ///
    /// A task stack has no event block of its own, so it never emits top-level
    /// communication; only the items run. Mutations reach the live cell exactly
    /// as they do for an event stack.
    ///
    /// `property` is the source-rooted path of the stack value itself
    /// (`tasks[0].setup`). Item `n`'s diagnostics are rooted at `{property}[n]`,
    /// so a task-stack failure names the authored task property rather than the
    /// synthetic signal the stack was parsed under.
    pub fn execute_action_stack(
        &self,
        items: &[super::actions::LifecycleStackItem],
        property: &str,
    ) -> LifecycleEventOutcome {
        self.execute_stack(items, StackRoot::Task(property))
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
    pub fn with_signal(&self, signal: LifecycleSignal) -> StackExecutionContext<'_> {
        StackExecutionContext {
            signal,
            frontmatter: self.frontmatter,
            live_frontmatter: self.live_frontmatter,
            runtime_state: self.runtime_state,
            err: self.err,
            timing: self.timing,
            current: self.current.clone(),
            group: self.group,
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
            signal: self.signal,
            frontmatter: self.frontmatter,
            live_frontmatter: self.live_frontmatter,
            runtime_state: self.runtime_state,
            err: Some(err),
            timing: self.timing,
            current: self.current.clone(),
            group: self.group,
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

    /// Build the event-time injected-globals layer (`err`/`timing`, plus
    /// `group` inside a sequence group) handed to Darkmatter's subtree compose
    /// and layered lookup.
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
            signal: self.signal,
            frontmatter: self.frontmatter,
            live_frontmatter: self.live_frontmatter,
            runtime_state: self.runtime_state,
            err: self.err,
            timing: self.timing,
            current: self.current.clone(),
            group: self.group,
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

    /// Build the event-time injected-globals layer (`err`/`timing`) handed to
    /// Darkmatter's subtree compose and layered lookup.
    fn injected_globals(&self) -> HashMap<String, InjectedGlobal> {
        let mut globals = lifecycle_injected_globals(self.err, self.timing);
        if let Some(variables) = self.group {
            globals.insert(
                "group".to_string(),
                InjectedGlobal::eager(Value::Object(variables.clone())),
            );
        }
        globals
    }

    /// Build resolution state for expressions authored by `source_path`.
    ///
    /// Implicit references resolve source-relative, then against the repository. The
    /// launch-area `ctx_base_dir` is retained as `file_ref_fallback_dir`
    /// diagnostic metadata; it is not a third resolution candidate. Top-level
    /// CLI references are resolved from launch context before this executor is
    /// entered.
    fn resolution_context(&self) -> ResolutionContext {
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
    /// globals (injected separately via [`Self::injected_globals`]);
    /// `current.*`/`current_env.*` are the reserved roots this state's refresh
    /// authority serves.
    fn build_state(&self, fm: &Map<String, Value>, scan_hint: &str) -> EffectiveState {
        let frontmatter: HashMap<String, Value> =
            fm.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
        let context = self.early_binding_context(scan_hint);
        EffectiveStateBuilder::new()
            .with_frontmatter(frontmatter)
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
        let hint = ctx_scan_hint(expr);
        let state = self.build_state(fm, &hint);
        let globals = self.injected_globals();
        let lookup = LayeredLookup::new(&state, &globals, Some(self.resolution_context()));
        evaluate(expr, &lookup).map_err(|error| LifecycleExprError::Evaluate(Box::new(error)))
    }

    /// Interpolate a string's `{{ … }}` spans at event-time through Darkmatter's
    /// subtree compose (DM2) in **strict** mode, preserving whole-value typing.
    ///
    /// Strict mode fails closed (C4): a malformed span, unknown function, or
    /// unknown root (a typo / genuinely-undefined variable) returns an error
    /// instead of degrading to empty, so a lifecycle side effect never renders
    /// silently-empty operational text. A reference whose root is *known* — a
    /// declared frontmatter key, `ctx`/`env`/`doc`, or an in-scope late-binding
    /// global — that resolves to `null`/empty still renders empty.
    ///
    /// After resolution, the post-DM2 leak guard rejects any recognized
    /// `{{ … }}` span surviving in the result (e.g. a frontmatter value that is
    /// itself raw template text), so no raw span reaches a dispatched side effect.
    fn resolve_string_value(
        &self,
        s: &str,
        fm: &Map<String, Value>,
    ) -> Result<Value, LifecycleExprError> {
        let state = self.build_state(fm, s);
        let globals = self.injected_globals();
        let value = Value::String(s.to_string());
        let resolved = SubtreeCompose::new(&value, &state)
            .with_globals(globals)
            .with_resolution_context(self.resolution_context())
            .strict()
            .compose()
            .map_err(|error| LifecycleExprError::Compose(Box::new(error)))?;
        reject_surviving_spans(resolved)
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

    /// Emit the top-level communication properties for one notification.
    ///
    /// Order: stdout, stderr, info, warn, success, message, notify, then the
    /// audio phases (`say`/`say_first` and `effect`, in their deterministic
    /// order). These strings are deferred lifecycle keys (raw `{{ … }}` through
    /// main compose), so each is interpolated **at event-time** against the live
    /// document state plus the late-binding roots (`err`/`timing`/`current`/
    /// `current_env`).
    ///
    /// Fails closed (C4): a resolution raise (malformed/unknown root) becomes an
    /// [`ActionFailure::Evaluation`] and an unknown resolved `effect` name an
    /// [`ActionFailure::Dispatch`], aborting emission before the offending field
    /// is sent, so no side effect dispatches silently-empty or raw operational
    /// text.
    fn emit_top_level(&self, n: &LifecycleNotification) -> Result<(), ActionFailure> {
        if let Some(text) = self.resolve_emit("stdout", n.stdout.as_deref())? {
            self.emitter.emit_stdout(&text, self.term);
        }
        if let Some(text) = self.resolve_emit("stderr", n.stderr.as_deref())? {
            self.emitter.emit_stderr(self.signal, &text, self.term);
        }
        if let Some(text) = self.resolve_emit("info", n.info.as_deref())? {
            self.emitter.emit_info(&text, self.term);
        }
        if let Some(text) = self.resolve_emit("warn", n.warn.as_deref())? {
            self.emitter.emit_warn(&text, self.term);
        }
        if let Some(text) = self.resolve_emit("success", n.success.as_deref())? {
            self.emitter.emit_success(&text, self.term);
        }
        if let Some(text) = self.resolve_emit("message", n.message.as_deref())? {
            self.emitter
                .emit_message(&text, self.source_path, self.repo_root, self.messaging);
        }
        if let Some(title) = self.resolve_emit("notify", n.notify.as_deref())? {
            self.emitter.emit_notification(&title);
        }
        for phase in audio_phases(n) {
            match phase {
                super::AudioPhase::Speak(text) => {
                    let field = if n.say.is_some() { "say" } else { "say_first" };
                    if let Some(text) = self.resolve_emit(field, Some(&text))? {
                        let config = tts_config_from_settings(self.settings.tts.as_ref());
                        self.emitter.emit_speech(&text, config);
                    }
                }
                super::AudioPhase::Effect(name) => {
                    if let Some(name) = self.resolve_emit("effect", Some(&name))? {
                        self.validate_effect_name(&name)
                            .map_err(ActionFailure::Dispatch)?;
                        self.emitter.emit_effect(&name);
                    }
                }
            }
        }
        Ok(())
    }

    /// Resolve a top-level communication field at event-time.
    ///
    /// `None` input (absent field) yields `Ok(None)`. A field with no `{{ … }}`
    /// span is emitted verbatim. A field carrying interpolation is resolved
    /// through DM2 (strict) against [`Self::frontmatter`]; a resolution raise is
    /// returned as an [`ActionFailure::Evaluation`] so the caller fails the
    /// event closed rather than dispatching silently-empty or raw template text.
    fn resolve_emit(
        &self,
        field: &str,
        text: Option<&str>,
    ) -> Result<Option<String>, ActionFailure> {
        let Some(text) = text else { return Ok(None) };
        if !text.contains("{{") {
            return Ok(Some(text.to_string()));
        }
        // Top-level fields read the live cross-event document state when present
        // so they observe frontmatter mutations made by *earlier* events in the
        // same attempt; otherwise the composed base frontmatter is used.
        let borrowed = self
            .live_frontmatter
            .map(|cell| cell.lock().expect(LIVE_POISONED));
        let fm = borrowed.as_deref().unwrap_or(self.frontmatter);
        self.resolve_string_value(text, fm)
            .map(|value| Some(scalar_string(&value)))
            .map_err(|error| {
                ActionFailure::Evaluation(
                    LifecycleErrorInfo::from_error_or_action("interpolation", &error)
                        .at_property(format!("{}.{field}", self.signal.property_name())),
                )
            })
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
    fn execute_stack(
        &self,
        items: &[super::actions::LifecycleStackItem],
        root: StackRoot<'_>,
    ) -> LifecycleEventOutcome {
        let mut working: Map<String, Value> = match self.live_frontmatter {
            Some(cell) => cell.lock().expect(LIVE_POISONED).clone(),
            None => self.frontmatter.clone(),
        };
        let outcome = self.execute_stack_inner(items, root, &mut working);
        if let Some(cell) = self.live_frontmatter {
            *cell.lock().expect(LIVE_POISONED) = working;
        }
        outcome
    }

    /// Run the stack loop against a caller-owned `working` map.
    ///
    /// Split from [`Self::execute_stack`] so the live-cell seed (before) and
    /// write-back (after) wrap a single inner pass that owns no cross-event
    /// concern: every return path here leaves `working` holding the mutations
    /// made up to that point, which the caller persists.
    fn execute_stack_inner(
        &self,
        items: &[super::actions::LifecycleStackItem],
        root: StackRoot<'_>,
        working: &mut Map<String, Value>,
    ) -> LifecycleEventOutcome {
        for (stack_index, item) in items.iter().enumerate() {
            match self.when_matches(item.when.as_ref(), working) {
                Ok(true) => {}
                Ok(false) => continue,
                // A `when:` guard that raised is an expression-layer evaluation
                // error, not a side-effect dispatch failure.
                Err(info) => {
                    let property = root.when_property(self.signal, stack_index);
                    return LifecycleEventOutcome {
                        evaluation_error: Some(info.at_property(property)),
                        ..Default::default()
                    };
                }
            }
            for (action_index, action) in item.actions.iter().enumerate() {
                let location = ActionLocation::new(self.signal, stack_index, action_index);
                let property = root.action_property(location);
                match self.run_action(action, location, &property, working) {
                    ActionStep::Continue => {}
                    ActionStep::Control(control) => {
                        return LifecycleEventOutcome {
                            control: Some(control),
                            ..Default::default()
                        };
                    }
                    ActionStep::Errored(info) => {
                        return LifecycleEventOutcome {
                            action_error: Some(info),
                            ..Default::default()
                        };
                    }
                    ActionStep::EvaluationErrored(info) => {
                        return LifecycleEventOutcome {
                            evaluation_error: Some(info.at_property(property)),
                            ..Default::default()
                        };
                    }
                }
            }
        }
        LifecycleEventOutcome::default()
    }

    /// Evaluate an optional `when:` clause, failing closed on an unresolvable
    /// guard. Omitted clauses always match (`Ok(true)`).
    ///
    /// A `when:` guard reacts to live document state, so it cannot be statically
    /// validated at prepare time (its referenced keys may be set by an earlier
    /// stack action). Instead it is checked just-in-time here against the live
    /// `fm`: an unknown root (a typo such as `spec_fil`) or a malformed/illegal
    /// expression returns `Err`, so the event fails closed before any side
    /// effect dispatches rather than silently skipping a recovery/messaging/
    /// file-mutating action. The walk shares the lifecycle-stack tolerance, so a
    /// guarded optional fallback (`maybe_missing || false`) is allowed. A guard
    /// that legitimately evaluates falsy returns `Ok(false)` and skips the item.
    fn when_matches(
        &self,
        when: Option<&Expr>,
        fm: &Map<String, Value>,
    ) -> Result<bool, LifecycleErrorInfo> {
        let Some(expr) = when else {
            return Ok(true);
        };
        if let Some(variable) = first_undefined_stack_variable(expr, Some(fm)) {
            return Err(LifecycleErrorInfo::from_action_failure(
                "when",
                format!("`when:` references undefined variable `{variable}`"),
            ));
        }
        self.eval_expr(expr, fm)
            .map(|value| is_truthy(&value))
            .map_err(|error| LifecycleErrorInfo::from_error_or_action("when", &error))
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
                    .render_message(&comm.message, working)
                    .map_err(|error| {
                        ActionFailure::Evaluation(LifecycleErrorInfo::from_error_or_action(
                            comm.channel.verb(),
                            &error,
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
                self.run_shell_action(shell, working).map(|()| None)
            }
            LifecycleActionKind::RuntimeSet(set) => {
                self.dispatch_runtime_set(set, &format!("{property}.set"), working)
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
                        ActionFailure::Evaluation(LifecycleErrorInfo::from_error_or_action(
                            func.function.clone(),
                            &error,
                        ))
                    })
            }
        }
    }

    /// Evaluate a message expression to its display string at event-time.
    ///
    /// A literal-default action body is an [`Expr::StringLiteral`]; a whole-value
    /// `{{ … }}` span resolves to a typed expression. We evaluate the expression
    /// (resolving multi-argument expression verbs), then interpolate any
    /// `{{ … }}` spans surviving inside a literal through DM2 against the current
    /// document `fm` plus the injected globals.
    ///
    /// Fails closed (C4): a whole-value span (or a function argument) referencing
    /// a genuinely-unknown frontmatter root — a typo — errors before dispatch
    /// rather than evaluating leniently to `null`/empty, matching the `when:`
    /// guard. A *known* root resolving to `null`/empty still renders empty.
    fn render_message(
        &self,
        expr: &Expr,
        fm: &Map<String, Value>,
    ) -> Result<String, LifecycleExprError> {
        if let Some(variable) = first_undefined_stack_variable(expr, Some(fm)) {
            return Err(LifecycleExprError::prose(format!(
                "references undefined variable `{variable}`"
            )));
        }
        let value = self.eval_expr(expr, fm)?;
        let rendered = scalar_string(&value);
        if rendered.contains("{{") {
            self.resolve_string_value(&rendered, fm)
                .map(|v| scalar_string(&v))
        } else {
            Ok(rendered)
        }
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
    fn run_shell_action(
        &self,
        shell: &super::actions::ShellAction,
        fm: &Map<String, Value>,
    ) -> Result<(), ActionFailure> {
        if self.signal == LifecycleSignal::Initialize {
            return Err(ActionFailure::Evaluation(LifecycleErrorInfo::from_error_or_action(
                "shell", &ShellRunError::BeforePreflight,
            )));
        }
        let command = self.render_message(&shell.command, fm).map_err(|error| {
            ActionFailure::Evaluation(LifecycleErrorInfo::from_error_or_action("shell", &error))
        })?;
        match self.shell_runner.run(&command) {
            Err(ShellRunError::BeforePreflight) => Err(ActionFailure::Evaluation(
                LifecycleErrorInfo::from_error_or_action("shell", &ShellRunError::BeforePreflight),
            )),
            Ok(0) => Ok(()),
            Ok(code) => {
                if let Some(on_error) = &shell.on_error {
                    if let Ok(text) = self.render_message(on_error, fm) {
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

    /// Dispatch a Darkmatter side effect by verb with positional, evaluated
    /// arguments.
    ///
    /// String arguments carrying `{{ … }}` are interpolated at event-time
    /// through DM2 (preserving whole-value typing), matching the literal-with-
    /// interpolation rule of communication bodies. A frontmatter-mutating verb
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
            .map(|expr| {
                let value = self.eval_expr(expr, working)?;
                if let Value::String(s) = &value {
                    if s.contains("{{") {
                        return self.resolve_string_value(s, working);
                    }
                }
                Ok(value)
            })
            .collect::<Result<Vec<_>, LifecycleExprError>>()
            .map_err(|error| {
                ActionFailure::Evaluation(LifecycleErrorInfo::from_error_or_action(verb, &error))
            })?;
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

        let result = match verb {
            "set_frontmatter" => engine.set_frontmatter(&s(0)?, &s(1)?, v(2)?),
            "merge_frontmatter" => engine.merge_frontmatter(&s(0)?, v(1)?),
            "delete_frontmatter" => engine.delete_frontmatter(&s(0)?, &s(1)?),
            "increment_frontmatter" => engine.increment_frontmatter(&s(0)?, &s(1)?),
            "decrement_frontmatter" => engine.decrement_frontmatter(&s(0)?, &s(1)?),
            "append_frontmatter" => engine.append_frontmatter(&s(0)?, &s(1)?, v(2)?),
            "prepend_frontmatter" => engine.prepend_frontmatter(&s(0)?, &s(1)?, v(2)?),
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
        let mut updates = IndexMap::with_capacity(set.len());
        for (key, value) in set.iter() {
            let resolved = self.resolve_with_value(value, &snapshot).map_err(|(suffix, error)| {
                let value_property = format!("{property}.{key}{suffix}");
                let reason = match &error {
                    LifecycleExprError::SurvivingSpan { span } => {
                        LifecycleEvaluationReason::SurvivingSpan { span: span.clone() }
                    }
                    LifecycleExprError::Evaluate(_)
                    | LifecycleExprError::Compose(_)
                    | LifecycleExprError::Prose(_) => {
                        LifecycleEvaluationReason::Expression
                    }
                };
                let diagnostic = CompositionError::LifecycleEvaluationError {
                    source_path: self.source_path.to_path_buf(),
                    event: self.signal.property_name().to_string(),
                    surface: "set".to_string(),
                    message: error.to_string(),
                    property: Some(value_property.clone()),
                    reason: Box::new(reason.clone()),
                };
                let mut info = LifecycleErrorInfo::from_composition_error(&diagnostic)
                    .at_property(value_property);
                info.variant = "set".to_string();
                info.reason = reason;
                ActionFailure::Evaluation(info)
            })?;
            updates.insert(key.clone(), resolved);
        }

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

    /// Resolve a document-authored mutation target through the same captured
    /// file-reference context used by composition.
    ///
    /// Existing targets follow ordinary document resolution precedence. For a
    /// missing implicit-relative target, creation stays anchored to the
    /// mutation root, preserving the effect engine's existing relative
    /// mutation policy; other reference kinds use their first document-scoped
    /// candidate. A later transclusion therefore resolves the same identity.
    /// Callers without a request snapshot retain the legacy behavior.
    fn resolve_effect_path(&self, verb: &str, raw: &str) -> Result<String, ActionFailure> {
        let Some(request_context) = self.file_resolution_context else {
            return Ok(raw.to_string());
        };
        let reference = FileReference::new(raw).map_err(|error| {
            ActionFailure::Dispatch(LifecycleErrorInfo::from_error_or_action(verb, &error))
        })?;
        let document_context = request_context.for_source(self.source_path);
        let mutation_context = request_context.for_base(self.effect_engine.mutation_root());
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
        let classify = |error: LifecycleExprError| {
            LifecycleErrorInfo::from_error_or_action(verb, &error)
        };
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
                let overlay = self.resolve_proxy_with(with, &target, location, fm)?;
                StackControl::Proxy {
                    target,
                    overlay,
                    location,
                }
            }
            C::Retry {
                max_attempts,
                backoff,
                delay,
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

    /// Evaluate a whole `proxy.with` mapping into the typed overlay the target
    /// will receive.
    ///
    /// Every value goes through the same DM2 subtree composition the rest of
    /// the lifecycle surface uses, so `with:` adds no second interpolation
    /// grammar. The first failing value aborts the whole mapping.
    fn resolve_proxy_with(
        &self,
        with: &ProxyWith,
        target: &str,
        location: ActionLocation,
        fm: &Map<String, Value>,
    ) -> Result<IndexMap<String, Value>, LifecycleErrorInfo> {
        // Capture the snapshot-less fallback context ONCE for the whole overlay,
        // then walk every leaf against it. Recursive `resolve_with_value`
        // evaluates each scalar leaf independently; without this a
        // `prepared_context: None` caller re-runs a demand-driven capture per
        // leaf. A prepared caller already reuses its single snapshot.
        match self.prepared_context {
            Some(_) => self.walk_proxy_with(self, with, target, location, fm),
            None => {
                let base = self
                    .ctx_base_dir
                    .or(self.base_dir)
                    .unwrap_or_else(|| Path::new("."));
                let content = proxy_with_scan_content(with);
                let fallback = capture_proxy_with_fallback(base, &content);
                let walker = self.with_prepared_context(&fallback);
                self.walk_proxy_with(&walker, with, target, location, fm)
            }
        }
    }

    /// Walk `with`'s leaves through `walker`, rooting a diagnostic at the exact
    /// nested property on failure.
    fn walk_proxy_with(
        &self,
        walker: &StackExecutionContext<'_>,
        with: &ProxyWith,
        target: &str,
        location: ActionLocation,
        fm: &Map<String, Value>,
    ) -> Result<IndexMap<String, Value>, LifecycleErrorInfo> {
        let mut overlay = IndexMap::with_capacity(with.len());
        for (key, value) in with.iter() {
            let resolved = walker
                .resolve_with_value(value, fm)
                .map_err(|(suffix, error)| {
                    let err = CompositionError::LifecycleProxyWithEvaluationFailed {
                        source_path: self.source_path.to_path_buf(),
                        property: format!("{}.stack[{}]", location.signal().property_name(), location.stack_index()),
                        path: format!("action[{}].with.{key}{suffix}", location.action_index()),
                        target: target.to_string(),
                        message: error.to_string(),
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
        value: &ProxyWithValue,
        fm: &Map<String, Value>,
    ) -> Result<Value, (String, LifecycleExprError)> {
        match value {
            ProxyWithValue::Null => Ok(Value::Null),
            ProxyWithValue::Scalar(expr) => self
                .resolve_typed_value(expr, fm)
                .map_err(|msg| (String::new(), msg)),
            ProxyWithValue::Array(items) => items
                .iter()
                .enumerate()
                .map(|(i, item)| {
                    self.resolve_with_value(item, fm)
                        .map_err(|(suffix, msg)| (format!("[{i}]{suffix}"), msg))
                })
                .collect::<Result<Vec<_>, _>>()
                .map(Value::Array),
            ProxyWithValue::Object(map) => map
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
    /// Fails closed the same way `render_message` does, and additionally
    /// rejects a raw span surviving *anywhere* inside a resolved container — a
    /// `{{ … }}` must never be deferred into target-time evaluation.
    fn resolve_typed_value(
        &self,
        expr: &Expr,
        fm: &Map<String, Value>,
    ) -> Result<Value, LifecycleExprError> {
        if let Some(variable) = first_undefined_stack_variable(expr, Some(fm)) {
            return Err(LifecycleExprError::prose(format!(
                "references undefined variable `{variable}`"
            )));
        }
        let value = self.eval_expr(expr, fm)?;
        let value = match &value {
            // A literal-default string may still carry `{{ … }}` spans (mixed
            // interpolation); DM2 resolves them and rejects a surviving span.
            Value::String(s) if s.contains("{{") => self.resolve_string_value(s, fm)?,
            _ => value,
        };
        reject_surviving_spans_deep(value)
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
/// span is a parsed [`Expr`], so its variable paths are collected. The union
/// feeds [`ComposeContext::capture_for_content`], which stays datetime-only when
/// no leaf references `ctx.*`.
fn proxy_with_scan_content(with: &ProxyWith) -> String {
    let mut content = String::new();
    for (_, value) in with.iter() {
        push_proxy_with_scan_content(value, &mut content);
    }
    content
}

/// Recursively append one `with:` value's scan content to `content`.
fn push_proxy_with_scan_content(value: &ProxyWithValue, content: &mut String) {
    match value {
        ProxyWithValue::Null => {}
        ProxyWithValue::Scalar(Expr::StringLiteral(s)) => {
            content.push(' ');
            content.push_str(s);
        }
        ProxyWithValue::Scalar(expr) => {
            let mut paths = Vec::new();
            collect_variable_paths(expr, &mut paths);
            for path in paths {
                content.push(' ');
                content.push_str(&path);
            }
        }
        ProxyWithValue::Array(items) => {
            for item in items {
                push_proxy_with_scan_content(item, content);
            }
        }
        ProxyWithValue::Object(map) => {
            for (_, value) in map {
                push_proxy_with_scan_content(value, content);
            }
        }
    }
}

/// Build a demand-driven `ctx.*` capture hint from an expression's variable
/// references, so [`ComposeContext::capture_for_content`] only captures the
/// sniff groups the expression actually reads (most read none).
fn ctx_scan_hint(expr: &Expr) -> String {
    let mut paths = Vec::new();
    collect_variable_paths(expr, &mut paths);
    paths
        .into_iter()
        .filter(|p| p.starts_with("ctx."))
        .collect::<Vec<_>>()
        .join(" ")
}

/// Walk an [`Expr`] pushing every [`Expr::Variable`] dotted path onto `paths`.
fn collect_variable_paths(expr: &Expr, paths: &mut Vec<String>) {
    match expr {
        Expr::Variable(path) => paths.push(path.clone()),
        Expr::StringLiteral(_) | Expr::NumberLiteral(_) | Expr::BoolLiteral(_) => {}
        Expr::UnaryNot(inner)
        | Expr::UnaryMinus(inner)
        | Expr::Paren(inner)
        | Expr::MemberAccess { base: inner, .. } => collect_variable_paths(inner, paths),
        Expr::Fallback { primary, fallback } => {
            collect_variable_paths(primary, paths);
            collect_variable_paths(fallback, paths);
        }
        Expr::Ternary {
            condition,
            then_branch,
            else_branch,
        } => {
            collect_variable_paths(condition, paths);
            collect_variable_paths(then_branch, paths);
            collect_variable_paths(else_branch, paths);
        }
        Expr::Comparison { left, right, .. } | Expr::Binary { left, right, .. } => {
            collect_variable_paths(left, paths);
            collect_variable_paths(right, paths);
        }
        Expr::Index { base, index } => {
            collect_variable_paths(base, paths);
            collect_variable_paths(index, paths);
        }
        Expr::FunctionCall { args, .. } => {
            for arg in args {
                collect_variable_paths(arg, paths);
            }
        }
        // A container literal reads every path its elements read, so
        // `[ctx.area, ctx.package]` must contribute both capture hints.
        // Object keys are authored text, not variable references.
        Expr::ArrayLiteral(elements) => {
            for element in elements {
                collect_variable_paths(element, paths);
            }
        }
        Expr::ObjectLiteral(entries) => {
            for (_, value) in entries {
                collect_variable_paths(value, paths);
            }
        }
    }
}

/// Post-DM2 dispatch-time leak guard (C4): reject a resolved value that still
/// contains a recognized `{{ … }}` span.
///
/// Strict subtree compose fails on a malformed/unknown *template* span, but a
/// resolved value can still carry raw braces when a referenced frontmatter key
/// holds literal template text (e.g. `set_frontmatter` stored `"{{x}}"`). This
/// catches that surviving span before the string reaches a side effect, so no
/// messenger/TTS/sound/stderr/stdout/notify dispatch ever sends raw syntax.
fn reject_surviving_spans(value: Value) -> Result<Value, LifecycleExprError> {
    if let Value::String(s) = &value {
        if let Some(span) = ExpressionFinder::find_all_plain(s).first() {
            return Err(LifecycleExprError::SurvivingSpan {
                span: s[span.start..span.end].to_string(),
            });
        }
    }
    Ok(value)
}

/// [`reject_surviving_spans`] extended through arrays and objects.
///
/// A whole-value span such as `{{ payload }}` resolves to a container the
/// scalar guard never looks inside, so a nested raw span would otherwise ride
/// into the overlay and become a second, target-time evaluation of source
/// syntax.
fn reject_surviving_spans_deep(value: Value) -> Result<Value, LifecycleExprError> {
    match value {
        Value::Array(items) => items
            .into_iter()
            .map(reject_surviving_spans_deep)
            .collect::<Result<Vec<_>, _>>()
            .map(Value::Array),
        Value::Object(map) => map
            .into_iter()
            .map(|(k, v)| reject_surviving_spans_deep(v).map(|v| (k, v)))
            .collect::<Result<Map<_, _>, _>>()
            .map(Value::Object),
        scalar => reject_surviving_spans(scalar),
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
