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
//! An action that errors (and is not marked `no_error: true`) stops the stack
//! and is logged. At a setup-phase event (`initialize`/`start`/`blocked`) the
//! error routes the run to `failure`; at a terminal-phase event
//! (`success`/`failure`/`finalize`/`loop`) the composition outcome is
//! unchanged. The explicit `Error` lifecycle action is distinct — it is a
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

use super::error::CompositionError;
use super::lifecycle::{
    LifecycleConfig, LifecycleEmitter, LifecycleNotification, LifecycleSignal, audio_phases,
    first_undefined_stack_variable, tts_config_from_settings,
};
use super::lifecycle_actions::{
    CommunicationChannel, LifecycleAction, LifecycleActionKind, LifecycleControlAction,
    RetryBackoff, is_known_side_effect,
};
use super::lifecycle_context::{
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
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LifecycleEventOutcome {
    /// The lifecycle control action that terminated stack processing, if any.
    /// `None` means the stack ran to completion (or there was no stack).
    pub control: Option<StackControl>,

    /// Set when an unintentional (non-`no_error`) action error stopped the
    /// stack. Carries the error snapshot so a routed-to `failure` event can
    /// expose `err.kind`/`err.variant`/`err.msg`.
    pub action_error: Option<LifecycleErrorInfo>,
}

impl LifecycleEventOutcome {
    /// Whether the unintentional action error recorded by this outcome must
    /// route the run to `failure`.
    ///
    /// True only when an action error occurred **and** `signal` is a
    /// setup-phase event ([`LifecycleSignal::routes_action_error_to_failure`]).
    pub fn routes_to_failure(&self, signal: LifecycleSignal) -> bool {
        self.action_error.is_some() && signal.routes_action_error_to_failure()
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
    /// An action errored (not suppressed by `no_error`); stop the stack.
    Errored(LifecycleErrorInfo),
}

impl StackExecutionContext<'_> {
    /// Run the event: top-level communication first, then the stack.
    pub fn execute_event(&self, config: &LifecycleConfig) -> LifecycleEventOutcome {
        if let Some(notification) = config.get(self.signal) {
            // Top-level emission fails closed: a resolution error becomes an
            // action error so the event halts before its stack runs, mirroring
            // a stack-action failure (C4).
            if let Err(info) = self.emit_top_level(notification) {
                warn!(
                    signal = ?self.signal,
                    kind = info.kind,
                    variant = %info.variant,
                    message = %info.msg,
                    "lifecycle top-level emission failed closed"
                );
                return LifecycleEventOutcome {
                    control: None,
                    action_error: Some(info),
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
    pub fn emit_top_level_for_signal(&self, config: &LifecycleConfig) {
        if let Some(notification) = config.get(self.signal) {
            // This path is used only for a terminal slot the caller has already
            // recorded (success/blocked), so there is no event left to fail; a
            // resolution error is logged rather than dispatched as raw text.
            if let Err(info) = self.emit_top_level(notification) {
                warn!(
                    signal = ?self.signal,
                    kind = info.kind,
                    variant = %info.variant,
                    message = %info.msg,
                    "lifecycle top-level emission failed; field skipped"
                );
            }
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
            err: self.err,
            timing: self.timing,
            current: self.current,
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

    /// Resolution context for read-side expression functions, rooted at the
    /// prompt's parent directory (or the current directory when unknown).
    fn resolution_context(&self) -> ResolutionContext {
        let dir = self
            .base_dir
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."));
        ResolutionContext::new(dir)
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
    /// Fails closed (C4): a resolution error (malformed/unknown root) or an
    /// unknown resolved `effect` name aborts emission with a
    /// [`LifecycleErrorInfo`] before the offending field is sent, so no side
    /// effect dispatches silently-empty or raw operational text.
    fn emit_top_level(&self, n: &LifecycleNotification) -> Result<(), LifecycleErrorInfo> {
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
                super::lifecycle::AudioPhase::Speak(text) => {
                    if let Some(text) = self.resolve_emit(Some(&text))? {
                        let config = tts_config_from_settings(self.settings.tts.as_ref());
                        self.emitter.emit_speech(&text, config);
                    }
                }
                super::lifecycle::AudioPhase::Effect(name) => {
                    if let Some(name) = self.resolve_emit(Some(&name))? {
                        self.validate_effect_name(&name)?;
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
    /// through DM2 (strict) against [`Self::frontmatter`]; a resolution failure
    /// is returned as a [`LifecycleErrorInfo`] so the caller fails the event
    /// closed rather than dispatching silently-empty or raw template text.
    fn resolve_emit(&self, text: Option<&str>) -> Result<Option<String>, LifecycleErrorInfo> {
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
            .map_err(|e| LifecycleErrorInfo::from_action_failure("interpolation", e))
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
    fn execute_stack(&self, items: &[super::lifecycle_actions::LifecycleStackItem]) -> LifecycleEventOutcome {
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
        items: &[super::lifecycle_actions::LifecycleStackItem],
        working: &mut Map<String, Value>,
    ) -> LifecycleEventOutcome {
        for item in items {
            match self.when_matches(item.when.as_ref(), working) {
                Ok(true) => {}
                Ok(false) => continue,
                Err(info) => {
                    return LifecycleEventOutcome {
                        control: None,
                        action_error: Some(info),
                    };
                }
            }
            for action in &item.actions {
                match self.run_action(action, working) {
                    ActionStep::Continue => {}
                    ActionStep::Control(control) => {
                        return LifecycleEventOutcome {
                            control: Some(control),
                            action_error: None,
                        };
                    }
                    ActionStep::Errored(info) => {
                        return LifecycleEventOutcome {
                            control: None,
                            action_error: Some(info),
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
    fn run_action(&self, action: &LifecycleAction, working: &mut Map<String, Value>) -> ActionStep {
        match self.execute_action_inner(action, working) {
            Ok(None) => ActionStep::Continue,
            Ok(Some(control)) => ActionStep::Control(control),
            Err(info) => {
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
    /// control action fired, or `Err(info)` on an action error.
    fn execute_action_inner(
        &self,
        action: &LifecycleAction,
        working: &mut Map<String, Value>,
    ) -> Result<Option<StackControl>, LifecycleErrorInfo> {
        match &action.kind {
            LifecycleActionKind::LifecycleControl(control) => self
                .resolve_control(control, working)
                .map(Some)
                .map_err(|msg| LifecycleErrorInfo::from_action_failure(control.verb(), msg)),
            LifecycleActionKind::Communication(comm) => {
                let message = self
                    .render_message(&comm.message, working)
                    .map_err(|msg| LifecycleErrorInfo::from_action_failure(comm.channel.verb(), msg))?;
                // Deferred effect validation (C4): an `effect` positional
                // action's name is only known after interpolation, so validate
                // the resolved name before dispatch.
                if comm.channel == CommunicationChannel::Effect {
                    self.validate_effect_name(&message)?;
                }
                self.emit_communication(comm.channel, &message);
                Ok(None)
            }
            LifecycleActionKind::Shell(shell) => {
                self.run_shell_action(shell, working).map(|()| None)
            }
            LifecycleActionKind::SideEffect(effect) => self
                .dispatch_side_effect(&effect.verb, &effect.args, working)
                .map(|_| None)
                .map_err(|msg| LifecycleErrorInfo::from_action_failure(effect.verb.clone(), msg)),
            LifecycleActionKind::ExpressionFunction(func) => {
                // A positional action whose verb is a known side effect was
                // parsed as an expression-function action; route it to the
                // side-effect engine here.
                if is_known_side_effect(&func.function) {
                    return self
                        .dispatch_side_effect(&func.function, &func.args, working)
                        .map(|_| None)
                        .map_err(|msg| {
                            LifecycleErrorInfo::from_action_failure(func.function.clone(), msg)
                        });
                }
                self.invoke_expression_function(&func.function, &func.args, working)
                    .map(|_| None)
                    .map_err(|msg| {
                        LifecycleErrorInfo::from_action_failure(func.function.clone(), msg)
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

    /// Run a shell action. A non-zero exit is an action error (unless
    /// `no_error` suppresses it upstream); `on_error` is emitted as a warning
    /// status line before the error propagates.
    fn run_shell_action(
        &self,
        shell: &super::lifecycle_actions::ShellAction,
        fm: &Map<String, Value>,
    ) -> Result<(), LifecycleErrorInfo> {
        let command = self
            .render_message(&shell.command, fm)
            .map_err(|msg| LifecycleErrorInfo::from_action_failure("shell", msg))?;
        match self.shell_runner.run(&command) {
            Ok(0) => Ok(()),
            Ok(code) => {
                if let Some(on_error) = &shell.on_error {
                    if let Ok(text) = self.render_message(on_error, fm) {
                        self.emitter.emit_warn(&text, self.term);
                    }
                }
                Err(LifecycleErrorInfo::from_action_failure(
                    "shell",
                    format!("command `{command}` exited with code {code}"),
                ))
            }
            Err(spawn_err) => Err(LifecycleErrorInfo::from_action_failure(
                "shell",
                format!("command `{command}` failed to run: {spawn_err}"),
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
    ) -> Result<Value, String> {
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
            .collect::<Result<Vec<_>, _>>()?;
        let engine = self.effect_engine;
        let s = |idx: usize| -> Result<String, String> {
            values
                .get(idx)
                .map(scalar_string)
                .ok_or_else(|| format!("`{verb}` is missing a required argument"))
        };
        let v = |idx: usize| -> Result<Value, String> {
            values
                .get(idx)
                .cloned()
                .ok_or_else(|| format!("`{verb}` is missing a required argument"))
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
            other => return Err(format!("unknown side effect `{other}`")),
        };
        let out = result.map_err(|e| e.to_string())?;
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
mod tests {
    use super::*;
    use std::sync::Mutex;

    use serde_json::json;

    use super::super::lifecycle::parse_lifecycle_config;

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
            outcome.action_error.is_some(),
            "unknown `when:` root must fail closed"
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
        assert!(outcome.action_error.is_some(), "typo must fail closed");
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
        assert!(outcome.action_error.is_some());
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
        assert!(outcome.action_error.is_some(), "surviving span must fail");
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
}
