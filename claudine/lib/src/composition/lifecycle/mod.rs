//! Lifecycle stacks for composition frontmatter.
//!
//! This module provides support for the eight composition lifecycle events
//! (`initialize`, `start`, `success`, `blocked`, `failure`, `finalize`,
//! `loop`, `cancelled`). Each event is a stack: an ordered action list, or one dictionary
//! of verbs that expands into that list in canonical order (see
//! [`actions::LifecycleNode`]). The `loop:` block additionally carries
//! iteration controls parsed by [`super::looping::resolve_loop_config`]; its
//! lifecycle concerns are loose verb keys or a `gate:` list.

// rustfmt doesn't support let-chains yet, so nested ifs are required
#![allow(clippy::collapsible_if)]

use std::path::Path;

use biscuit_speaks::{SpeedLevel, TtsConfig, TtsFailoverStrategy};
use biscuit_terminal::components::prose::LineBreaks;
use biscuit_terminal::components::status::{Status, StatusState, StatusTheme};
use biscuit_terminal::prelude::{Prose, TerminalRenderable};
use biscuit_terminal::terminal::Terminal;
use darkmatter::markdown::compose::expression::{
    Expr, ExpressionFinder, parse, parse_condition,
};
use tracing::warn;

use super::error::{ActionExprError, CompositionError};
use self::actions::{
    all_lifecycle_verbs, CommunicationAction, CommunicationChannel, ExpressionFunctionAction,
    expression_function_signature, is_known_lifecycle_verb, LifecycleAction, LifecycleActionKind,
    LifecycleControlAction, LifecycleNode, LifecyclePath, WithOverlay, WithOverlayError,
    RuntimeSet, RuntimeSetError, RetryBackoff, rewrite_to_positional, ShellAction,
    SideEffectAction, side_effect_signature,
};
use crate::events::{GlobalSettings, TtsSettings};
use crate::messaging::RuntimeMessagingSettings;

mod action_shape;
pub mod actions;
mod audio;
pub mod bindings;
pub mod context;
pub mod control;
pub mod executor;
pub mod runtime;
mod parse;
pub mod shell_template;
mod structured_shell;
pub mod prewarm;
mod source_map;
mod validate;

use action_shape::*;
pub(crate) use action_shape::action_value_to_expr;
pub(crate) use audio::*;
pub use parse::{LifecycleParseOptions, scan_removed_validation_keys};
pub(crate) use parse::LOOP_ITERATION_KEYS;
pub use validate::{
    collect_lifecycle_shell_commands, collect_lifecycle_shell_commands_for,
    validate_no_err_in_no_error_events, validate_no_nested_spans_in_literals,
};

/// Parses lifecycle configuration from composition frontmatter.
///
/// Every event (`initialize`, `start`, `success`, `blocked`, `failure`,
/// `finalize`) and the `loop:` block's lifecycle concerns parse into an
/// ordered [`LifecycleNode`] stack on [`LifecycleConfig::stacks`]. Literal
/// `effect` names are checked against the embedded Playa catalog here, so an
/// unknown name fails before anything runs; a `{{ … }}` name is checked when
/// the event resolves it.
///
/// ## Errors
///
/// The lifecycle authoring family of [`CompositionError`]: shape violations
/// ([`CompositionError::LifecycleStackInvalidShape`]), removed spellings
/// ([`CompositionError::RemovedGrammar`]), unknown verbs or parameters,
/// directive placement and order, and
/// [`CompositionError::LifecycleUnknownEffect`].
///
/// ## Examples
///
/// ```
/// # use serde_json::json;
/// # use claudine::composition::{parse_lifecycle_config, LifecycleSignal};
/// let frontmatter = json!({
///     "title": "My Composition",
///     "start": [{ "message": "Starting..." }]
/// });
/// let config = parse_lifecycle_config(&frontmatter, std::path::Path::new("test.md")).unwrap();
/// assert_eq!(config.stack(LifecycleSignal::Start).len(), 1);
/// ```
pub fn parse_lifecycle_config(
    frontmatter: &serde_json::Value,
    source_file: &Path,
) -> Result<LifecycleConfig, CompositionError> {
    parse_lifecycle_config_with_orders(frontmatter, source_file, None)
}

/// [`parse_lifecycle_config`] with the authored frontmatter, whose retained
/// mapping order gives a `set:` mapping its authored key order.
pub(crate) fn parse_lifecycle_config_with_orders(
    frontmatter: &serde_json::Value,
    source_file: &Path,
    authored: Option<&darkmatter::markdown::Frontmatter>,
) -> Result<LifecycleConfig, CompositionError> {
    parse::parse_lifecycle_config_with(
        frontmatter,
        source_file,
        &LifecycleParseOptions {
            is_known_effect: &is_known_effect,
            authored,
        },
    )
}

/// Parse a task's `setup:` / `teardown:` value into a typed action stack.
///
/// The value is a stack like any event: a list, or one dictionary of verbs.
/// `property` is the source-rooted path of the value itself
/// (`tasks[0].setup`), and item `n` is `{property}[n]`.
///
/// ## Errors
///
/// The same [`CompositionError`] family [`parse_lifecycle_config`] raises.
pub fn parse_task_action_stack(
    signal: LifecycleSignal,
    raw: &serde_json::Value,
    source_file: &Path,
    property: &str,
) -> Result<Vec<LifecycleNode>, CompositionError> {
    parse_task_action_stack_with_order(
        signal,
        raw,
        source_file,
        property,
        &super::authored_order::AuthoredOrder::default(),
    )
}

/// [`parse_task_action_stack`] against the stack's authored key order, which
/// is positioned at the `setup:`/`teardown:` value itself.
pub(crate) fn parse_task_action_stack_with_order(
    signal: LifecycleSignal,
    raw: &serde_json::Value,
    source_file: &Path,
    property: &str,
    authored: &super::authored_order::AuthoredOrder,
) -> Result<Vec<LifecycleNode>, CompositionError> {
    parse::parse_task_action_stack_with(
        signal,
        raw,
        source_file,
        property,
        authored,
        &LifecycleParseOptions {
            is_known_effect: &is_known_effect,
            authored: None,
        },
    )
}

/// Parse a task's single `side_effect:` action.
///
/// `property` is the source-rooted path of the action value itself
/// (`tasks[0].side_effect`); the action's path is `{property}.{verb}`.
///
/// ## Errors
///
/// [`CompositionError::LifecycleStackInvalidShape`] when the value is not a
/// one-verb dictionary, and the standard action rejections otherwise.
pub fn parse_single_action(
    signal: LifecycleSignal,
    raw: &serde_json::Value,
    source_file: &Path,
    property: &str,
) -> Result<LifecycleAction, CompositionError> {
    parse_single_action_with_order(signal, raw, source_file, property, None)
}

/// [`parse_single_action`] with a `set:` mapping's authored key order.
pub(crate) fn parse_single_action_with_order(
    signal: LifecycleSignal,
    raw: &serde_json::Value,
    source_file: &Path,
    property: &str,
    authored_set_order: Option<&[String]>,
) -> Result<LifecycleAction, CompositionError> {
    parse::parse_single_action_with(
        signal,
        raw,
        source_file,
        property,
        authored_set_order,
        &LifecycleParseOptions {
            is_known_effect: &is_known_effect,
            authored: None,
        },
    )
}

/// The eight top-level frontmatter keys that hold lifecycle event subtrees.
///
/// Claudine defers these from Darkmatter's compose-time value-resolution
/// passes (via `ComposeOptions::with_exclude_keys`) so their authored
/// `{{ }}` spans survive raw in `effective_frontmatter` for event-time
/// interpolation. Non-lifecycle keys compose as today; the iteration
/// controls inside `loop:` (`while`/`until`/`actions`/`max`/`fail_fast`)
/// are unaffected because they are parsed from raw frontmatter by
/// [`super::looping::resolve_loop_config`] and evaluated by the loop
/// engine, not by compose-time interpolation.
///
/// Order matches [`LifecycleSignal::ALL`] for readable diffs; the set is
/// what callers consume.
pub const LIFECYCLE_EVENT_KEYS: &[&str] = &[
    "initialize",
    "start",
    "success",
    "blocked",
    "failure",
    "finalize",
    "loop",
    "cancelled",
];


/// Complete lifecycle configuration for a composition.
///
/// Parsed from frontmatter properties `initialize`, `start`, `success`,
/// `blocked`, `failure`, `cancelled`, `finalize`, and the lifecycle concerns of `loop:`.
///
/// ## Examples
///
/// ```yaml
/// start:
///   - message: "Starting..."
/// success:
///   effect: "crowd-applause"
///   say: "Composition complete"
/// ```
#[derive(Debug, Clone, Default, PartialEq)]
pub struct LifecycleConfig {
    /// Typed, validated stacks keyed by event.
    pub stacks: LifecycleStacks,
    /// Whether `cancelled` was authored, including an empty stack. Sibling
    /// cancellation opts in by presence, independently of cleanup actions.
    pub cancelled_declared: bool,
}

/// Per-event typed stacks.
///
/// An omitted event, `[]`, and `{}` contain no actions. The configuration
/// separately retains cancellation opt-in presence. `cancelled: null` is
/// invalid; other events retain their null-as-empty behavior.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct LifecycleStacks {
    #[allow(missing_docs)]
    pub initialize: Vec<LifecycleNode>,
    #[allow(missing_docs)]
    pub start: Vec<LifecycleNode>,
    #[allow(missing_docs)]
    pub success: Vec<LifecycleNode>,
    #[allow(missing_docs)]
    pub blocked: Vec<LifecycleNode>,
    #[allow(missing_docs)]
    pub failure: Vec<LifecycleNode>,
    #[allow(missing_docs)]
    pub finalize: Vec<LifecycleNode>,
    #[allow(missing_docs)]
    pub cancelled: Vec<LifecycleNode>,
    /// The `loop:` block's lifecycle concerns: its loose verb keys or `gate:`.
    pub loop_gate: Vec<LifecycleNode>,
}

/// Lifecycle event signal types.
///
/// The eight composition lifecycle signals, in deterministic iteration
/// order: `Initialize`, `Start`, `Success`, `Blocked`, `Failure`,
/// `Finalize`, `Loop`, `Cancelled`. This order is used by validators that walk every
/// event surface (nested-span scan, `err` availability check) and is exposed
/// via [`Self::all`].
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum LifecycleSignal {
    /// Prompt file has been identified and frontmatter has parsed, before
    /// user `$schema` validation and shell pre-flight checks.
    Initialize,

    /// Pre-flight checks have all passed; about to invoke the agent.
    Start,

    /// Agentic loop completed without error.
    Success,

    /// Pre-flight checks failed; agent will not be invoked.
    Blocked,

    /// Agentic loop returned an error.
    Failure,

    /// Once per iteration, immediately after the terminal `success`/
    /// `failure`, `blocked`, or `cancelled` completes.
    Finalize,

    /// Post-`finalize` "iterate again?" gate; evaluates the document's
    /// `loop` while/until condition.
    Loop,

    /// Execution was interrupted or a running leg opted into sibling cancellation.
    Cancelled,
}

impl LifecycleSignal {
    /// All signals in deterministic iteration order.
    ///
    /// Used by validators that walk every event surface. New variants
    /// should be appended to the end so existing diagnostic order is
    /// preserved.
    pub const ALL: [Self; 8] = [
        Self::Initialize,
        Self::Start,
        Self::Success,
        Self::Blocked,
        Self::Failure,
        Self::Finalize,
        Self::Loop,
        Self::Cancelled,
    ];
}

/// Runtime state tracking for lifecycle events.
///
/// Superseded by [`LifecycleRunGuard`] which enforces transitions mechanically.
/// Retained for backward compatibility.
#[deprecated(
    since = "0.1.0",
    note = "use LifecycleRunGuard instead, which enforces transitions mechanically"
)]
#[derive(Debug, Clone, Default)]
pub struct LifecycleRuntimeState {
    /// Whether the `start` signal has been emitted.
    pub start_emitted: bool,

    /// Whether the provider launch has started (used for start signal timing).
    pub provider_launch_started: bool,
}

// ---------------------------------------------------------------------------
// Emitter trait + default implementation
// ---------------------------------------------------------------------------

/// Trait for emitting lifecycle communication side effects.
///
/// Injectable to allow test doubles that capture emissions without hitting
/// real stderr, messaging, TTS, or sound playback. The text channels keep the
/// rows of their text: a newline (authored, or inside an interpolated value
/// such as a captured output) starts a new line.
pub trait LifecycleEmitter: Sync {
    /// Write a plain prose line (no status glyph) to stderr.
    ///
    /// `stderr` is the statusless channel: rich text and links are honored, but
    /// no `Status` decoration is applied. Status decoration is reserved for
    /// [`emit_info`](Self::emit_info) and [`emit_warn`](Self::emit_warn).
    fn emit_stderr(&self, signal: LifecycleSignal, text: &str, term: &Terminal);

    /// Dispatch a message via the configured messaging route.
    fn emit_message(
        &self,
        text: &str,
        source_path: &Path,
        repo_root: Option<&Path>,
        messaging: &RuntimeMessagingSettings,
    );

    /// Speak text via TTS.
    fn emit_speech(&self, text: &str, tts_config: TtsConfig);

    /// Play a named sound effect.
    fn emit_effect(&self, name: &str);

    /// Fire a local desktop notification.
    fn emit_notification(&self, title: &str);

    /// Write a styled `Info` status line to stderr.
    ///
    /// Unlike [`emit_stderr`](Self::emit_stderr), the status state is fixed to
    /// [`StatusState::Info`] regardless of the owning lifecycle event. The
    /// default renders the same way [`DefaultLifecycleEmitter`] would.
    fn emit_info(&self, text: &str, term: &Terminal) {
        let rendered = Status::from_prose(text)
            .with_line_breaks(LineBreaks::Hard)
            .state(StatusState::Info)
            .theme(StatusTheme::Circular)
            .render(term);
        crate::render::console::write_stderr_line(&rendered);
    }

    /// Write a styled `Warning` status line to stderr.
    ///
    /// The status state is fixed to [`StatusState::Warning`] regardless of the
    /// owning lifecycle event.
    fn emit_warn(&self, text: &str, term: &Terminal) {
        let rendered = Status::from_prose(text)
            .with_line_breaks(LineBreaks::Hard)
            .state(StatusState::Warning)
            .theme(StatusTheme::Circular)
            .render(term);
        crate::render::console::write_stderr_line(&rendered);
    }

    /// Write a styled `Success` status line to stderr.
    ///
    /// The status state is fixed to [`StatusState::Success`] regardless of the
    /// owning lifecycle event.
    fn emit_success(&self, text: &str, term: &Terminal) {
        let rendered = Status::from_prose(text)
            .with_line_breaks(LineBreaks::Hard)
            .state(StatusState::Success)
            .theme(StatusTheme::Circular)
            .render(term);
        crate::render::console::write_stderr_line(&rendered);
    }

    /// Write a plain prose line (no status glyph) to **stdout**.
    ///
    /// The only lifecycle channel that targets stdout. Inline styling and links
    /// are honored, but no status decoration is applied. Authors opt into this
    /// knowing stdout is otherwise reserved for pipeable command data.
    fn emit_stdout(&self, text: &str, term: &Terminal) {
        let rendered = Prose::new(text).with_line_breaks(LineBreaks::Hard).render(term);
        crate::render::console::write_stdout_line(&rendered);
    }
}

/// Production emitter that performs real side effects.
pub struct DefaultLifecycleEmitter;

impl LifecycleEmitter for DefaultLifecycleEmitter {
    fn emit_stderr(&self, _signal: LifecycleSignal, text: &str, term: &Terminal) {
        // `stderr` carries no status: it is plain prose (rich text/links honored)
        // routed to STDERR. Status glyphs belong to `info`/`warn` only.
        let rendered = Prose::new(text).with_line_breaks(LineBreaks::Hard).render(term);
        crate::render::console::write_stderr_line(&rendered);
    }

    fn emit_message(
        &self,
        text: &str,
        source_path: &Path,
        repo_root: Option<&Path>,
        messaging: &RuntimeMessagingSettings,
    ) {
        crate::messaging::execute_resolved_message(
            text,
            None,
            Some(source_path),
            repo_root,
            messaging,
        );
    }

    fn emit_speech(&self, text: &str, tts_config: TtsConfig) {
        enqueue_speech(text, tts_config);
    }

    fn emit_effect(&self, name: &str) {
        enqueue_effect(name);
    }

    fn emit_notification(&self, title: &str) {
        crate::messaging::execute_notification(title, None);
    }
}

// ---------------------------------------------------------------------------
// LifecycleRunGuard
// ---------------------------------------------------------------------------

/// RAII guard that centralizes lifecycle state transitions and guarantees a
/// terminal signal (`blocked` or `failure`) is emitted when a run exits
/// after `start` without an explicit terminal signal.
///
/// The guard tracks terminal exclusivity and cleanup:
/// `initialize` → `start` → (`success`|`blocked`|`failure`|`cancelled`) → `finalize`.
/// Each non-terminal signal is idempotent; terminal signals are exclusive
/// (only one may fire per iteration); `finalize` may fire once after the
/// terminal signal.
///
/// ## Drop behaviour
///
/// | `start_emitted` | `provider_launched` | Drop emits |
/// |-----------------|---------------------|------------|
/// | `false`         | —                   | nothing    |
/// | `true`          | `false`             | `Blocked`  |
/// | `true`          | `true`              | `Failure`  |
///
/// `finalize` is never emitted by Drop. Explicit calls to
/// [`emit_terminal`](Self::emit_terminal),
/// [`emit_blocked_or_failure`](Self::emit_blocked_or_failure), or
/// [`defuse`](Self::defuse) suppress the Drop emission.
pub struct LifecycleRunGuard<'a> {
    // `Cow` so a `proxy` hand-off can repoint the guard at the target
    // document's lifecycle (an owned, re-parsed config) without the borrowed
    // original's lifetime. The common case stays a zero-cost borrow.
    config: std::borrow::Cow<'a, LifecycleConfig>,
    ctx: &'a LifecycleRuntimeContext<'a>,
    emitter: &'a dyn LifecycleEmitter,
    initialize_emitted: bool,
    start_emitted: bool,
    provider_launched: bool,
    terminal_emitted: bool,
    finalize_emitted: bool,
    terminal_signal: Option<LifecycleSignal>,
    // Set once a `proxy` hand-off adopts a target document. The composition
    // -start `ctx.*` snapshot in `ctx.context` was captured demand-driven for
    // the *original* document, so it omits groups only the proxied target
    // references — it must be dropped for the target's events.
    proxied: bool,
    // The current composition run's early-binding context, when the run is not
    // the one the guard was built for: a proxied target's rebuilt context
    // (its own provider/model identity, recomputed from its frontmatter), or a
    // retried or resumed attempt's fresh capture. Owned by the guard so it can
    // outlive the transient loop borrows that build it, and preferred over both
    // `ctx.context` (the first run's snapshot) and the demand-driven fallback,
    // so the run's lifecycle `ctx.*`/`env.*` reads that run's observation.
    run_prepared_context: Option<darkmatter::markdown::compose::ComposeContext>,
}

impl<'a> LifecycleRunGuard<'a> {
    /// Create a new guard.
    pub fn new(
        config: &'a LifecycleConfig,
        ctx: &'a LifecycleRuntimeContext<'a>,
        emitter: &'a dyn LifecycleEmitter,
    ) -> Self {
        Self {
            config: std::borrow::Cow::Borrowed(config),
            ctx,
            emitter,
            initialize_emitted: false,
            start_emitted: false,
            provider_launched: false,
            terminal_emitted: false,
            finalize_emitted: false,
            terminal_signal: None,
            proxied: false,
            run_prepared_context: None,
        }
    }

    /// Emit the `Initialize` signal (idempotent).
    pub fn emit_initialize_once(&mut self) {
        if !self.initialize_emitted {
            self.emit_signal(LifecycleSignal::Initialize);
            self.initialize_emitted = true;
        }
    }

    /// Emit the `Start` signal (idempotent — only emits on the first call).
    pub fn emit_start_once(&mut self) {
        if !self.start_emitted {
            self.emit_signal(LifecycleSignal::Start);
            self.start_emitted = true;
        }
    }

    /// Record that the provider child process has actually been spawned.
    ///
    /// Call this **after** `execute_harness_attempt` returns `Ok`, not before.
    pub fn mark_provider_launched(&mut self) {
        self.provider_launched = true;
    }

    /// Emit a specific terminal signal and suppress the Drop safety-net.
    ///
    /// Terminal signals (`Success`, `Blocked`, `Failure`) are mutually
    /// exclusive; the first one wins.
    pub fn emit_terminal(&mut self, signal: LifecycleSignal) {
        if self.terminal_emitted {
            return;
        }
        self.emit_signal(signal);
        self.terminal_emitted = true;
        self.terminal_signal = Some(signal);
    }

    /// Emit `Blocked` (pre-launch) or `Failure` (post-launch) and return
    /// the original error unchanged. Convenient for wrapping `?` returns.
    pub fn emit_blocked_or_err<E>(&mut self, err: E) -> E {
        self.emit_blocked_or_failure();
        err
    }

    /// Emit `Blocked` (pre-launch) or `Failure` (post-launch) and suppress
    /// the Drop safety-net.
    pub fn emit_blocked_or_failure(&mut self) {
        let signal = if self.provider_launched {
            LifecycleSignal::Failure
        } else {
            LifecycleSignal::Blocked
        };
        self.emit_terminal(signal);
    }

    /// Record that `signal` is about to be emitted and return whether the
    /// caller should actually run the event.
    ///
    /// This is split from [`Self::run_event_stack`] so the runtime can build
    /// a [`StackExecutionContext`] that borrows the guard's emitter without
    /// conflicting with the mutable state update.
    pub fn record_event_emission(&mut self, signal: LifecycleSignal) -> bool {
        match signal {
            LifecycleSignal::Initialize => {
                if self.initialize_emitted {
                    return false;
                }
                self.initialize_emitted = true;
            }
            LifecycleSignal::Start => {
                if self.start_emitted {
                    return false;
                }
                self.start_emitted = true;
            }
            LifecycleSignal::Success
            | LifecycleSignal::Blocked
            | LifecycleSignal::Failure
            | LifecycleSignal::Cancelled => {
                if self.terminal_emitted {
                    return false;
                }
                self.terminal_emitted = true;
                self.terminal_signal = Some(signal);
            }
            LifecycleSignal::Finalize => {
                if self.finalize_emitted || !self.terminal_emitted {
                    return false;
                }
                self.finalize_emitted = true;
            }
            LifecycleSignal::Loop => {}
        }
        true
    }

    /// Re-designate an already-recorded `Success`/`Blocked` terminal signal to
    /// `Failure` so a routed-to `failure` event (and the subsequent `finalize`)
    /// can still fire.
    ///
    /// A `success`/`blocked` stack that ends in an explicit `error(...)` action
    /// downgrades the run to `failure`. The actions before the directive in the
    /// success/blocked stack have **already** run and their emissions must
    /// remain — so the terminal slot was already taken by `Success`/`Blocked`.
    /// This overwrites only [`Self::terminal_signal`] to `Failure` while keeping
    /// [`Self::terminal_emitted`] true, letting the caller run the `failure`
    /// event's stack directly (without
    /// [`Self::record_event_emission`], which would refuse the taken slot) and
    /// keeping [`Self::emit_finalize_once`] enabled.
    ///
    /// No-op (returns `false`) unless the currently recorded terminal signal is
    /// `Success` or `Blocked`; the existing slot is otherwise authoritative.
    pub fn redesignate_terminal_to_failure(&mut self) -> bool {
        if !matches!(
            self.terminal_signal,
            Some(LifecycleSignal::Success) | Some(LifecycleSignal::Blocked)
        ) {
            return false;
        }
        self.terminal_signal = Some(LifecycleSignal::Failure);
        true
    }

    /// Run the typed stack for `signal` using the provided context.
    ///
    /// Callers must first call [`Self::record_event_emission`] and only run
    /// the stack when it returns `true`.
    pub fn run_event_stack(
        &self,
        signal: LifecycleSignal,
        stack_ctx: &super::lifecycle_executor::StackExecutionContext<'_>,
    ) -> super::lifecycle_executor::LifecycleEventOutcome {
        // A repeat Ctrl+C gives an in-flight terminal event a short grace
        // window rather than force-exiting through its side effects.
        let _terminal_scope = matches!(
            signal,
            LifecycleSignal::Success
                | LifecycleSignal::Blocked
                | LifecycleSignal::Failure
                | LifecycleSignal::Cancelled
                | LifecycleSignal::Finalize
        )
        .then(crate::interrupt::TerminalLifecycleScope::enter);
        let mut context = stack_ctx.with_signal(signal);
        // `start` follows successful preflight; proxy adoption resets it. Catch
        // events reached before that boundary must not inherit a shell runner.
        if !self.start_emitted {
            context.shell_runner = &super::lifecycle_executor::DisabledShellRunner;
        }
        context.execute_event(&self.config)
    }

    /// Execute the lifecycle event's stack for `signal`, tracking the guard's
    /// state machine.
    ///
    /// Convenience wrapper around [`Self::record_event_emission`] +
    /// [`Self::run_event_stack`]. Prefer the split methods when the context
    /// borrows from the guard to avoid borrow-checker conflicts.
    pub fn execute_event(
        &mut self,
        signal: LifecycleSignal,
        stack_ctx: &super::lifecycle_executor::StackExecutionContext<'_>,
    ) -> super::lifecycle_executor::LifecycleEventOutcome {
        if !self.record_event_emission(signal) {
            return super::lifecycle_executor::LifecycleEventOutcome::default();
        }
        self.run_event_stack(signal, stack_ctx)
    }


    /// Emit the `Finalize` signal once, after a terminal signal has fired.
    ///
    /// If no terminal signal has been emitted yet, this is a no-op (the
    /// caller should resolve the terminal state first).
    pub fn emit_finalize_once(&mut self) {
        if self.finalize_emitted {
            return;
        }
        if !self.terminal_emitted {
            return;
        }
        self.emit_signal(LifecycleSignal::Finalize);
        self.finalize_emitted = true;
    }

    /// Reset per-iteration state so the guard can drive the next loop
    /// iteration without double-emitting terminal or finalize events.
    ///
    /// Preserves `initialize_emitted` so `initialize` fires exactly once
    /// across the whole loop run.
    pub fn reset_for_next_iteration(&mut self) {
        self.start_emitted = false;
        self.provider_launched = false;
        self.terminal_emitted = false;
        self.finalize_emitted = false;
        self.terminal_signal = None;
    }

    /// Reset **all** per-run state, including `initialize_emitted`, so the
    /// guard can drive a fresh prompt run from `initialize`.
    ///
    /// Unlike [`Self::reset_for_next_iteration`] (which preserves
    /// `initialize_emitted` because loop iterations re-enter at `start`), a
    /// `Proxy` hand-off replaces the running document with another prompt
    /// that must run its own `initialize`/pre-flight. The proxied document is
    /// a distinct run, so every signal — including `initialize` — fires again.
    pub fn reset_for_proxy(&mut self) {
        self.initialize_emitted = false;
        self.start_emitted = false;
        self.provider_launched = false;
        self.terminal_emitted = false;
        self.finalize_emitted = false;
        self.terminal_signal = None;
        self.proxied = true;
        // A previous hand-off's rebuilt context belongs to the document being
        // replaced. Drop it so the newly adopted target's events fall back to
        // the demand-driven capture until its own launch rebuild installs a
        // fresh one.
        self.run_prepared_context = None;
    }

    /// Install the early-binding context of the composition run now active.
    ///
    /// Every run observes its own `ctx.*`, so a run that replaces the one this
    /// guard was built for hands its snapshot here:
    ///
    /// - a proxied target, whose launch rebuild recomputes its identity
    ///   (provider/model → `env.AGENT`/`env.MODEL`) from its own frontmatter,
    ///   so its lifecycle reads what a direct invocation would;
    /// - a retried or resumed attempt, whose fresh capture observes Git
    ///   working state as the previous attempt left it.
    ///
    /// From this point [`Self::effective_prepared_context`] returns it.
    pub fn set_run_prepared_context(
        &mut self,
        context: darkmatter::markdown::compose::ComposeContext,
    ) {
        self.run_prepared_context = Some(context);
    }

    /// The early-binding `ctx.*`/`env.*` snapshot lifecycle events should use,
    /// or `None` to force a demand-driven per-expression re-capture.
    ///
    /// Returns the composition-start snapshot for a normally-run document.
    /// Once a later run has installed its own snapshot through
    /// [`Self::set_run_prepared_context`] — a proxied target's rebuild, which
    /// carries the target's own resolved provider/model identity, or a retried
    /// or resumed attempt's fresh capture — that snapshot wins. Before a proxied
    /// target's rebuild installs one (or if none is installed) a
    /// proxied guard returns `None`: the *original* document's snapshot omits any
    /// `ctx.*` group only the target references, so it is dropped and the
    /// executor re-captures at the launch area, exactly as the target's own body
    /// composition already does.
    pub fn effective_prepared_context(
        &self,
    ) -> Option<&darkmatter::markdown::compose::ComposeContext> {
        // The active run's installed context wins: a proxied target's carries
        // its own resolved identity, a retry's or resume's its fresh capture.
        if let Some(context) = self.run_prepared_context.as_ref() {
            return Some(context);
        }
        if self.proxied {
            None
        } else {
            self.ctx.context
        }
    }

    /// Suppress the Drop emission without emitting any signal.
    ///
    /// Use when transferring lifecycle responsibility elsewhere.
    pub fn defuse(&mut self) {
        self.terminal_emitted = true;
    }

    pub fn initialize_emitted(&self) -> bool {
        self.initialize_emitted
    }

    pub fn start_emitted(&self) -> bool {
        self.start_emitted
    }

    pub fn provider_launched(&self) -> bool {
        self.provider_launched
    }

    pub fn terminal_signal(&self) -> Option<LifecycleSignal> {
        self.terminal_signal
    }

    pub fn finalize_emitted(&self) -> bool {
        self.finalize_emitted
    }

    /// The runtime context this guard was constructed with.
    pub fn context(&self) -> &LifecycleRuntimeContext<'_> {
        self.ctx
    }

    /// The emitter this guard delegates communication to.
    pub fn emitter(&self) -> &dyn LifecycleEmitter {
        self.emitter
    }

    /// The lifecycle configuration this guard is driving.
    pub fn config(&self) -> &LifecycleConfig {
        &self.config
    }

    /// Repoint the guard at a different lifecycle config (owned).
    ///
    /// A `proxy` hand-off replaces the running document with a different
    /// prompt. The target document is a distinct run with its own lifecycle
    /// blocks, so the guard must drive **its** events, not the proxying
    /// document's. Pair this with [`Self::reset_for_proxy`] so every signal —
    /// including `initialize` — fires again for the target.
    pub fn set_config(&mut self, config: LifecycleConfig) {
        self.config = std::borrow::Cow::Owned(config);
    }

    /// Emit a signal's unguarded, literal root communication without an
    /// execution context.
    ///
    /// This is the drop safety net's path (and the `emit_*_once` helpers'):
    /// with no [`StackExecutionContext`] it cannot evaluate a guard,
    /// interpolate, or run anything but communication. It emits, in stack
    /// order (a root dictionary's canonical order), each root-level `stderr`,
    /// `message`, `notify`, `say`/`speak`, and `effect` whose text contains no
    /// `{{ … }}` span. Guarded items, templated text, and every other verb are
    /// skipped rather than emitted unevaluated. Ordinary runs go through
    /// [`Self::run_event_stack`] instead.
    ///
    /// Short-circuits on `crate::interrupt::interrupted()` so a Ctrl+C
    /// during execution skips messenger sends, desktop notifications, and
    /// durable audio publication. The cheap stderr line still emits so the
    /// user sees the terminal status before the process exits.
    ///
    /// [`StackExecutionContext`]: super::lifecycle_executor::StackExecutionContext
    fn emit_signal(&self, signal: LifecycleSignal) {
        let mut tts_config: Option<TtsConfig> = None;
        for node in self.config.stack(signal) {
            let LifecycleNode::Action(LifecycleAction {
                kind: LifecycleActionKind::Communication(comm),
                ..
            }) = node
            else {
                continue;
            };
            let Expr::StringLiteral(text) = &comm.text else {
                continue;
            };
            // Mixed text such as `done: {{ fm.title }}` is still a string
            // literal; without a context its spans cannot render, and emitting
            // them raw would speak or send template source.
            if !ExpressionFinder::find_all_plain(text).is_empty() {
                continue;
            }
            if comm.channel == CommunicationChannel::Stderr {
                self.emitter.emit_stderr(signal, text, self.ctx.term);
                continue;
            }
            if crate::interrupt::interrupted() {
                continue;
            }
            match comm.channel {
                CommunicationChannel::Message => self.emitter.emit_message(
                    text,
                    self.ctx.source_path,
                    self.ctx.repo_root,
                    self.ctx.messaging,
                ),
                CommunicationChannel::Notify => self.emitter.emit_notification(text),
                CommunicationChannel::Say | CommunicationChannel::Speak => {
                    let config = tts_config
                        .get_or_insert_with(|| {
                            tts_config_from_settings(self.ctx.settings.tts.as_ref())
                        })
                        .clone();
                    self.emitter.emit_speech(text, config);
                }
                CommunicationChannel::Effect => self.emitter.emit_effect(text),
                _ => {}
            }
        }
    }
}

impl Drop for LifecycleRunGuard<'_> {
    fn drop(&mut self) {
        if self.start_emitted && !self.terminal_emitted {
            let signal = if self.provider_launched {
                LifecycleSignal::Failure
            } else {
                LifecycleSignal::Blocked
            };
            self.emit_signal(signal);
        }
    }
}

/// Runtime context required for emitting lifecycle communication.
///
/// Holds references to settings, messaging configuration, terminal, and paths
/// needed to resolve and emit lifecycle communication actions.
#[derive(Debug)]
pub struct LifecycleRuntimeContext<'a> {
    /// Global settings (includes TTS configuration).
    pub settings: &'a GlobalSettings,

    /// Runtime messaging settings.
    pub messaging: &'a RuntimeMessagingSettings,

    /// Terminal for output rendering.
    pub term: &'a Terminal,

    /// Path to the composition source file.
    pub source_path: &'a Path,

    /// Repository root (if in a git repository).
    pub repo_root: Option<&'a Path>,

    /// Directory the caller launched from (the package area), used as the base
    /// for `ctx.*` capture in lifecycle events; falls back to the prompt/source
    /// directory when `None`.
    pub launch_area: Option<&'a Path>,

    /// The single early-binding context snapshot captured once at composition
    /// start (against the launch area) and shared with body compose and
    /// preflight. When `Some`, lifecycle events reuse this exact `ctx.*`/`env.*`
    /// snapshot so they can never diverge from the body; when `None`, the
    /// executor falls back to a demand-driven re-capture rooted at the launch
    /// area. `current.*` stays event-time regardless.
    pub context: Option<&'a darkmatter::markdown::compose::ComposeContext>,
}

impl LifecycleConfig {
    /// Returns the typed stack for a given signal; empty when the event is
    /// absent.
    ///
    /// For [`LifecycleSignal::Loop`] this is the `loop:` block's lifecycle
    /// concerns.
    ///
    /// ## Examples
    ///
    /// ```
    /// # use claudine::composition::{LifecycleConfig, LifecycleSignal};
    /// let config = LifecycleConfig::default();
    /// assert!(config.stack(LifecycleSignal::Start).is_empty());
    /// ```
    pub fn stack(&self, signal: LifecycleSignal) -> &[LifecycleNode] {
        match signal {
            LifecycleSignal::Initialize => &self.stacks.initialize,
            LifecycleSignal::Start => &self.stacks.start,
            LifecycleSignal::Success => &self.stacks.success,
            LifecycleSignal::Blocked => &self.stacks.blocked,
            LifecycleSignal::Failure => &self.stacks.failure,
            LifecycleSignal::Finalize => &self.stacks.finalize,
            LifecycleSignal::Loop => &self.stacks.loop_gate,
            LifecycleSignal::Cancelled => &self.stacks.cancelled,
        }
    }

    /// Mutable access to the typed stack for a given signal.
    ///
    /// Used by the pre-flight shell resolution pass (C3) to stamp resolved
    /// command bytes back into `ShellAction::command` / `on_error` so the
    /// approved command equals the executed command.
    pub fn stack_mut(&mut self, signal: LifecycleSignal) -> &mut Vec<LifecycleNode> {
        match signal {
            LifecycleSignal::Initialize => &mut self.stacks.initialize,
            LifecycleSignal::Start => &mut self.stacks.start,
            LifecycleSignal::Success => &mut self.stacks.success,
            LifecycleSignal::Blocked => &mut self.stacks.blocked,
            LifecycleSignal::Failure => &mut self.stacks.failure,
            LifecycleSignal::Finalize => &mut self.stacks.finalize,
            LifecycleSignal::Loop => &mut self.stacks.loop_gate,
            LifecycleSignal::Cancelled => &mut self.stacks.cancelled,
        }
    }

    /// Returns `true` when there are no actions or explicit cancellation opt-in.
    ///
    /// ## Examples
    ///
    /// ```
    /// # use claudine::composition::LifecycleConfig;
    /// let config = LifecycleConfig::default();
    /// assert!(config.is_empty());
    /// ```
    pub fn is_empty(&self) -> bool {
        !self.cancelled_declared && LifecycleSignal::ALL
            .iter()
            .all(|signal| self.stack(*signal).is_empty())
    }
}

/// Removed harness validation/handler DSL keys and their lifecycle
/// replacements.
///
/// These top-level frontmatter keys were retired when the lifecycle stack
/// model replaced the harness validation and handler execution layers.
#[cfg(test)]
mod tests;
