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

use biscuit_terminal::terminal::Terminal;
use darkmatter::effects::EffectEngine;
use darkmatter::markdown::compose::expression::{
    Expr, ExpressionFinder, ResolutionContext, evaluate, is_truthy, scalar_string,
};
use darkmatter::markdown::compose::subtree::{InjectedGlobal, LayeredLookup, SubtreeCompose};
use darkmatter::markdown::compose::{ComposeContext, EffectiveState, EffectiveStateBuilder};
use serde_json::{Map, Value};
use tracing::warn;

use super::super::error::CompositionError;
use super::{
    LifecycleConfig, LifecycleEmitter, LifecycleNotification, LifecycleSignal, audio_phases,
    first_undefined_stack_variable, tts_config_from_settings,
};
use super::actions::{
    CommunicationChannel, LifecycleAction, LifecycleActionKind, LifecycleControlAction,
    RetryBackoff, is_known_side_effect,
};
use super::context::{
    LifecycleCurrent, LifecycleErrorInfo, LifecycleTiming, lifecycle_injected_globals,
};
use crate::events::GlobalSettings;
use crate::messaging::RuntimeMessagingSettings;

/// A resolved lifecycle control action — the runtime-flow effect a stack item
/// requested, with every expression argument already evaluated.
///
/// The parse-time [`LifecycleControlAction`] carries unevaluated [`Expr`]
/// arguments; this is its post-evaluation form, suitable for the composition
/// runtime to act on directly.
#[derive(Debug, Clone, PartialEq, Eq)]
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
    Proxy {
        /// Evaluated target prompt reference (e.g. `@prompts/foo.md`).
        target: String,
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

/// Abstraction over running an approved shell command.
///
/// Lifecycle shell actions are audited against Claudine's command whitelist
/// during pre-flight; this trait runs an already-approved command. Injectable
/// so tests can assert command dispatch without spawning real processes.
pub trait ShellRunner {
    /// Run `command`. Returns the process exit code (or an error string when
    /// the process could not be spawned at all).
    fn run(&self, command: &str) -> Result<i32, String>;
}

/// Production [`ShellRunner`] that runs commands through the system shell.
#[derive(Debug, Default, Clone, Copy)]
pub struct SystemShellRunner;

impl ShellRunner for SystemShellRunner {
    fn run(&self, command: &str) -> Result<i32, String> {
        let mut cmd = system_shell_command(command);
        let status = cmd.status().map_err(|e| e.to_string())?;
        Ok(status.code().unwrap_or(-1))
    }
}

/// Build the platform `Command` that runs `command` through the system shell.
#[cfg(windows)]
fn system_shell_command(command: &str) -> std::process::Command {
    let mut cmd = std::process::Command::new("cmd");
    cmd.arg("/C").arg(command);
    cmd
}

/// Build the platform `Command` that runs `command` through the system shell.
#[cfg(not(windows))]
fn system_shell_command(command: &str) -> std::process::Command {
    let mut cmd = std::process::Command::new("sh");
    cmd.arg("-c").arg(command);
    cmd
}

/// Everything the executor needs to run one lifecycle event.
///
/// Construct one per event with the active [`LifecycleSignal`], the composed
/// frontmatter, the lifecycle globals (`err`/`timing`/`current`), and the
/// side-effect / shell / emitter routes.
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
    pub live_frontmatter: Option<&'a std::cell::RefCell<Map<String, Value>>>,
    /// The `err` global snapshot (only meaningful for error-carrying events).
    pub err: Option<&'a LifecycleErrorInfo>,
    /// The `timing` global snapshot.
    pub timing: Option<&'a LifecycleTiming>,
    /// The `current` global snapshot (lazy `ctx`/`env` capture).
    pub current: Option<&'a LifecycleCurrent>,
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
            Some(items) if !items.is_empty() => self.execute_stack(items),
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

    /// Return a copy of this context targeting a different lifecycle signal.
    ///
    /// Used by [`LifecycleRunGuard::execute_event`](super::lifecycle::LifecycleRunGuard::execute_event)
    /// so one constructed context can service every signal in a run.
    pub fn with_signal(&self, signal: LifecycleSignal) -> StackExecutionContext<'_> {
        StackExecutionContext {
            signal,
            frontmatter: self.frontmatter,
            live_frontmatter: self.live_frontmatter,
            err: self.err,
            timing: self.timing,
            current: self.current,
            group: self.group,
            base_dir: self.base_dir,
            ctx_base_dir: self.ctx_base_dir,
            prepared_context: self.prepared_context,
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
            err: Some(err),
            timing: self.timing,
            current: self.current,
            group: self.group,
            base_dir: self.base_dir,
            ctx_base_dir: self.ctx_base_dir,
            prepared_context: self.prepared_context,
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

    /// Build the event-time injected-globals layer (`err`/`timing`/`current`)
    /// handed to Darkmatter's subtree compose and layered lookup.
    fn injected_globals(&self) -> HashMap<String, InjectedGlobal> {
        lifecycle_injected_globals(self.err, self.timing, self.current)
    }

    /// Resolution context for read-side expression functions.
    ///
    /// Resolution order: `base_dir` (the prompt document's parent, or the
    /// current directory when unknown) is the primary anchor for
    /// document-authored references; the captured launch area
    /// (`ctx_base_dir`) is the explicit fallback for caller-supplied file
    /// references so resolution is independent of the mutated ambient
    /// process CWD after the wrapper's `chdir`.
    fn resolution_context(&self) -> ResolutionContext {
        let dir = self
            .base_dir
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."));
        match self.ctx_base_dir {
            Some(launch_area) => {
                ResolutionContext::new(dir).with_file_ref_fallback_dir(launch_area.to_path_buf())
            }
            None => ResolutionContext::new(dir),
        }
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
    /// re-capture against `scan_hint`. Only `current.*`/`err`/`timing` are
    /// event-time globals (injected separately via [`Self::injected_globals`]).
    fn build_state(&self, fm: &Map<String, Value>, scan_hint: &str) -> EffectiveState {
        let frontmatter: HashMap<String, Value> =
            fm.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
        let context = self.early_binding_context(scan_hint);
        EffectiveStateBuilder::new()
            .with_frontmatter(frontmatter)
            .with_context(context)
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
    fn eval_expr(&self, expr: &Expr, fm: &Map<String, Value>) -> Result<Value, String> {
        let hint = ctx_scan_hint(expr);
        let state = self.build_state(fm, &hint);
        let globals = self.injected_globals();
        let lookup = LayeredLookup::new(&state, &globals, Some(self.resolution_context()));
        evaluate(expr, &lookup).map_err(|e| e.to_string())
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
    fn resolve_string_value(&self, s: &str, fm: &Map<String, Value>) -> Result<Value, String> {
        let state = self.build_state(fm, s);
        let globals = self.injected_globals();
        let value = Value::String(s.to_string());
        let resolved = SubtreeCompose::new(&value, &state)
            .with_globals(globals)
            .with_resolution_context(self.resolution_context())
            .strict()
            .compose()
            .map_err(|e| e.to_string())?;
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
    /// document state plus the late-binding globals (`err`/`timing`/`current`).
    ///
    /// Fails closed (C4): a resolution raise (malformed/unknown root) becomes an
    /// [`ActionFailure::Evaluation`] and an unknown resolved `effect` name an
    /// [`ActionFailure::Dispatch`], aborting emission before the offending field
    /// is sent, so no side effect dispatches silently-empty or raw operational
    /// text.
    fn emit_top_level(&self, n: &LifecycleNotification) -> Result<(), ActionFailure> {
        if let Some(text) = self.resolve_emit(n.stdout.as_deref())? {
            self.emitter.emit_stdout(&text, self.term);
        }
        if let Some(text) = self.resolve_emit(n.stderr.as_deref())? {
            self.emitter.emit_stderr(self.signal, &text, self.term);
        }
        if let Some(text) = self.resolve_emit(n.info.as_deref())? {
            self.emitter.emit_info(&text, self.term);
        }
        if let Some(text) = self.resolve_emit(n.warn.as_deref())? {
            self.emitter.emit_warn(&text, self.term);
        }
        if let Some(text) = self.resolve_emit(n.success.as_deref())? {
            self.emitter.emit_success(&text, self.term);
        }
        if let Some(text) = self.resolve_emit(n.message.as_deref())? {
            self.emitter
                .emit_message(&text, self.source_path, self.repo_root, self.messaging);
        }
        if let Some(title) = self.resolve_emit(n.notify.as_deref())? {
            self.emitter.emit_notification(&title);
        }
        for phase in audio_phases(n) {
            match phase {
                super::AudioPhase::Speak(text) => {
                    if let Some(text) = self.resolve_emit(Some(&text))? {
                        let config = tts_config_from_settings(self.settings.tts.as_ref());
                        self.emitter.emit_speech(&text, config);
                    }
                }
                super::AudioPhase::Effect(name) => {
                    if let Some(name) = self.resolve_emit(Some(&name))? {
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
    fn resolve_emit(&self, text: Option<&str>) -> Result<Option<String>, ActionFailure> {
        let Some(text) = text else { return Ok(None) };
        if !text.contains("{{") {
            return Ok(Some(text.to_string()));
        }
        // Top-level fields read the live cross-event document state when present
        // so they observe frontmatter mutations made by *earlier* events in the
        // same attempt; otherwise the composed base frontmatter is used.
        let borrowed = self.live_frontmatter.map(std::cell::RefCell::borrow);
        let fm = borrowed.as_deref().unwrap_or(self.frontmatter);
        self.resolve_string_value(text, fm)
            .map(|value| Some(scalar_string(&value)))
            .map_err(|e| {
                ActionFailure::Evaluation(LifecycleErrorInfo::from_action_failure(
                    "interpolation",
                    e,
                ))
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
    fn execute_stack(&self, items: &[super::actions::LifecycleStackItem]) -> LifecycleEventOutcome {
        let mut working: Map<String, Value> = match self.live_frontmatter {
            Some(cell) => cell.borrow().clone(),
            None => self.frontmatter.clone(),
        };
        let outcome = self.execute_stack_inner(items, &mut working);
        if let Some(cell) = self.live_frontmatter {
            *cell.borrow_mut() = working;
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
        working: &mut Map<String, Value>,
    ) -> LifecycleEventOutcome {
        for item in items {
            match self.when_matches(item.when.as_ref(), working) {
                Ok(true) => {}
                Ok(false) => continue,
                // A `when:` guard that raised is an expression-layer evaluation
                // error, not a side-effect dispatch failure.
                Err(info) => {
                    return LifecycleEventOutcome {
                        evaluation_error: Some(info),
                        ..Default::default()
                    };
                }
            }
            for action in &item.actions {
                match self.run_action(action, working) {
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
                            evaluation_error: Some(info),
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
            .map_err(|e| LifecycleErrorInfo::from_action_failure("when", e))
    }

    /// Run one action, applying the `no_error` escape hatch.
    ///
    /// `no_error` is scoped to side-effect **dispatch** failures only: an
    /// expression-layer **evaluation** raise always halts the stack and
    /// surfaces, because a thrown guard/interpolation is a defect the author
    /// cannot meaningfully "tolerate" the way a flaky channel send can be.
    fn run_action(&self, action: &LifecycleAction, working: &mut Map<String, Value>) -> ActionStep {
        match self.execute_action_inner(action, working) {
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
    fn execute_action_inner(
        &self,
        action: &LifecycleAction,
        working: &mut Map<String, Value>,
    ) -> Result<Option<StackControl>, ActionFailure> {
        match &action.kind {
            LifecycleActionKind::LifecycleControl(control) => self
                .resolve_control(control, working)
                .map(Some)
                .map_err(|msg| {
                    ActionFailure::Evaluation(LifecycleErrorInfo::from_action_failure(
                        control.verb(),
                        msg,
                    ))
                }),
            LifecycleActionKind::Communication(comm) => {
                let message = self.render_message(&comm.message, working).map_err(|msg| {
                    ActionFailure::Evaluation(LifecycleErrorInfo::from_action_failure(
                        comm.channel.verb(),
                        msg,
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
                    .map_err(|msg| {
                        ActionFailure::Evaluation(LifecycleErrorInfo::from_action_failure(
                            func.function.clone(),
                            msg,
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
    fn render_message(&self, expr: &Expr, fm: &Map<String, Value>) -> Result<String, String> {
        if let Some(variable) = first_undefined_stack_variable(expr, Some(fm)) {
            return Err(format!("references undefined variable `{variable}`"));
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
    /// (subject to `no_error` upstream). `on_error` is emitted as a warning
    /// status line before a dispatch error propagates.
    fn run_shell_action(
        &self,
        shell: &super::actions::ShellAction,
        fm: &Map<String, Value>,
    ) -> Result<(), ActionFailure> {
        let command = self.render_message(&shell.command, fm).map_err(|msg| {
            ActionFailure::Evaluation(LifecycleErrorInfo::from_action_failure("shell", msg))
        })?;
        match self.shell_runner.run(&command) {
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
            Err(spawn_err) => Err(ActionFailure::Dispatch(
                LifecycleErrorInfo::from_action_failure(
                    "shell",
                    format!("command `{command}` failed to run: {spawn_err}"),
                ),
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
    /// action in the same stack reads the mutated value.
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
            .collect::<Result<Vec<_>, String>>()
            .map_err(|msg| {
                ActionFailure::Evaluation(LifecycleErrorInfo::from_action_failure(verb, msg))
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
                    engine.ensure_file_with_content(&s(0)?, &s(1)?).map(Value::String)
                } else {
                    engine.ensure_file(&s(0)?).map(Value::String)
                }
            }
            "ensure_dir" => engine.ensure_dir(&s(0)?).map(Value::String),
            "append_line" => engine.append_line(&s(0)?, &s(1)?).map(Value::String),
            "append_jsonl" => engine.append_jsonl(&s(0)?, v(1)?).map(Value::String),
            "http_post" => engine.http_post(&s(0)?, s(1)?.into_bytes()),
            other => return Err(dispatch_err(format!("unknown side effect `{other}`"))),
        };
        let out = result.map_err(|e| dispatch_err(e.to_string()))?;
        self.mirror_frontmatter_mutation(verb, &values, working);
        Ok(out)
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
    ) -> Result<Value, String> {
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
    fn resolve_control(
        &self,
        control: &LifecycleControlAction,
        fm: &Map<String, Value>,
    ) -> Result<StackControl, String> {
        use LifecycleControlAction as C;
        Ok(match control {
            C::Stop => StackControl::Stop,
            C::Skip => StackControl::Skip,
            C::Error { reason } => StackControl::Error {
                reason: self.eval_opt_string(reason.as_ref(), fm)?,
            },
            C::Proxy { target } => StackControl::Proxy {
                target: self.render_message(target, fm)?,
            },
            C::Retry {
                max_attempts,
                backoff,
                delay,
            } => StackControl::Retry {
                max_attempts: self.eval_opt_u32(max_attempts.as_ref(), fm)?.unwrap_or(1),
                backoff: backoff.unwrap_or(RetryBackoff::Fixed),
                delay: self
                    .eval_opt_string(delay.as_ref(), fm)?
                    .unwrap_or_else(|| "0s".to_string()),
            },
            C::Resume {
                message,
                max_attempts,
            } => StackControl::Resume {
                message: self.render_message(message, fm)?,
                max_attempts: self.eval_opt_u32(max_attempts.as_ref(), fm)?.unwrap_or(1),
            },
            C::Defer { delay, reason } => StackControl::Defer {
                delay: self.render_message(delay, fm)?,
                reason: self.eval_opt_string(reason.as_ref(), fm)?,
            },
        })
    }

    /// Evaluate an optional expression to a display string.
    fn eval_opt_string(
        &self,
        expr: Option<&Expr>,
        fm: &Map<String, Value>,
    ) -> Result<Option<String>, String> {
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
    ) -> Result<Option<u32>, String> {
        let Some(expr) = expr else {
            return Ok(None);
        };
        let value = self.eval_expr(expr, fm)?;
        let n = value
            .as_f64()
            .ok_or_else(|| format!("expected a number, got {value}"))?;
        if n < 0.0 || n.fract() != 0.0 {
            return Err(format!("expected a non-negative whole number, got {n}"));
        }
        Ok(Some(n as u32))
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
fn reject_surviving_spans(value: Value) -> Result<Value, String> {
    if let Value::String(s) = &value {
        if !ExpressionFinder::find_all_plain(s).is_empty() {
            return Err(format!(
                "unresolved interpolation survived event-time resolution: `{s}`"
            ));
        }
    }
    Ok(value)
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
