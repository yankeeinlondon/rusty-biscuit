//! Lifecycle notification types and parsing for composition frontmatter.
//!
//! This module provides support for the seven composition lifecycle events
//! (`initialize`, `start`, `success`, `blocked`, `failure`, `finalize`,
//! `loop`) in composition frontmatter. Each event may carry top-level
//! communication properties (`say`, `say_first`, `effect`, `message`,
//! `stderr`, `notify`, `info`, `warn`) and an ordered `stack:` of
//! conditional actions. The `loop` event additionally carries iteration
//! controls parsed by [`super::looping::resolve_loop_config`].

// rustfmt doesn't support let-chains yet, so nested ifs are required
#![allow(clippy::collapsible_if)]

use std::path::Path;

use biscuit_speaks::{SpeedLevel, TtsConfig, TtsFailoverStrategy};
use biscuit_terminal::components::status::{Status, StatusState, StatusTheme};
use biscuit_terminal::prelude::{Prose, TerminalRenderable};
use biscuit_terminal::terminal::Terminal;
use darkmatter::markdown::compose::expression::{
    Expr, ExpressionFinder, parse, parse_condition,
};
use serde::{Deserialize, Serialize};
use tracing::warn;

use super::error::CompositionError;
use self::actions::{
    all_lifecycle_verbs, CommunicationAction, CommunicationChannel, ExpressionFunctionAction,
    expression_function_signature, is_known_lifecycle_verb, LifecycleAction, LifecycleActionKind,
    LifecycleControlAction, LifecycleStackItem, ProxyWith, ProxyWithError,
    RetryBackoff, rewrite_to_positional, ShellAction,
    SideEffectAction, side_effect_signature,
};
use crate::events::{GlobalSettings, TtsSettings};
use crate::messaging::RuntimeMessagingSettings;

mod action_shape;
pub mod actions;
mod audio;
pub mod context;
pub mod control;
pub mod executor;
pub mod runtime;
mod parse;
mod validate;

use action_shape::*;
pub use audio::*;
#[cfg(test)]
use audio::run_blocking_with_timeout;
pub use parse::{parse_lifecycle_config, scan_removed_validation_keys};
pub use validate::{
    collect_lifecycle_shell_commands, collect_lifecycle_shell_commands_for,
    validate_no_err_in_no_error_events,
    validate_no_interpolation_leaks, validate_no_undefined_lifecycle_variables,
};
pub(crate) use validate::first_undefined_stack_variable;
#[cfg(test)]
use validate::undefined_bare_variable;

/// The canonical communication-field names for [`LifecycleNotification`],
/// in the deterministic iteration order used by every validator that walks
/// lifecycle event surfaces.
///
/// Excludes `stack` because `stack` is a structured (list) field rather
/// than a string. Use [`LIFECYCLE_CONCERN_KEYS`] when the full set of
/// lifecycle concern keys (including `stack`) is needed.
const LIFECYCLE_COMM_FIELDS: &[&str] = &[
    "say",
    "say_first",
    "message",
    "stderr",
    "notify",
    "info",
    "warn",
    "success",
    "stdout",
];

/// The notification's nine communication-field name/value pairs, in the
/// [`LIFECYCLE_COMM_FIELDS`] iteration order.
///
/// Shared by the lifecycle string guards that walk top-level communication
/// surfaces (the leak scan and the `err`-availability scan) so they agree on
/// the field set and iteration order.
fn notification_comm_fields(
    n: &LifecycleNotification,
) -> [(&'static str, Option<&String>); 9] {
    [
        ("say", n.say.as_ref()),
        ("say_first", n.say_first.as_ref()),
        ("message", n.message.as_ref()),
        ("stderr", n.stderr.as_ref()),
        ("notify", n.notify.as_ref()),
        ("info", n.info.as_ref()),
        ("warn", n.warn.as_ref()),
        ("success", n.success.as_ref()),
        ("stdout", n.stdout.as_ref()),
    ]
}

/// The seven top-level frontmatter keys that hold lifecycle event subtrees.
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
];

/// The lifecycle late-binding global roots — values that exist only at
/// event-time: `err` (active failure), `timing` (observed durations), and
/// `current` (event-time `ctx`/`env` snapshots).
///
/// Shared authority for the pre-flight shell resolution (C3), which rejects
/// any late-binding reference inside a `shell` command because shell commands
/// are resolved at pre-flight — before any event fires — so only early-binding
/// values (`doc.*`, `ctx.*`, `env.*`, read-side functions) are available there.
pub use super::reserved::LATE_BINDING_ROOTS;

/// A single lifecycle notification configuration.
///
/// Carries the top-level communication properties for one lifecycle event
/// block. The optional `stack` field captures the raw stack items; typed
/// parsing into [`LifecycleStackItem`] values happens in
/// [`parse_lifecycle_config`], which has the event-name context needed for
/// the per-event "Where valid" matrix.
///
/// ## Examples
///
/// ```yaml
/// start:
///   say: "Starting composition workflow"
///   effect: "confirmation"
/// ```
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct LifecycleNotification {
    /// Text to say using TTS (mutually exclusive with `say_first`).
    pub say: Option<String>,

    /// Text to say before other actions (mutually exclusive with `say`).
    pub say_first: Option<String>,

    /// Sound effect to play (kebab-case name like "confirmation").
    pub effect: Option<String>,

    /// Message to display in the terminal.
    pub message: Option<String>,

    /// Message to write to stderr.
    pub stderr: Option<String>,

    /// Title for a local desktop notification.
    pub notify: Option<String>,

    /// Status line rendered with `Status::Info` style.
    pub info: Option<String>,

    /// Status line rendered with `Status::Warn` style.
    pub warn: Option<String>,

    /// Status line rendered with `Status::Success` style.
    pub success: Option<String>,

    /// Plain text written to stdout (no status glyph).
    pub stdout: Option<String>,

    /// Raw stack items, captured at deserialize time and parsed into typed
    /// [`LifecycleStackItem`] values by [`parse_lifecycle_config`] (which
    /// knows the owning event and can enforce the per-event "Where valid"
    /// matrix). After `parse_lifecycle_config` returns, the typed form
    /// lives on [`LifecycleConfig::stacks`] and this field is cleared.
    #[serde(default)]
    pub stack: Option<Vec<serde_json::Value>>,
}

/// Complete lifecycle configuration for a composition.
///
/// Parsed from frontmatter properties: `initialize`, `start`, `success`,
/// `blocked`, `failure`, `finalize`. Lifecycle concerns authored inside the
/// `loop:` block (alongside iteration controls) are extracted into
/// [`Self::loop_concerns`] and surfaced through
/// [`Self::get`](`LifecycleSignal::Loop`).
///
/// Each event's typed stack (post-validation) lives in [`Self::stacks`].
///
/// ## Examples
///
/// ```yaml
/// start:
///   message: "Starting..."
/// success:
///   say: "Composition complete"
///   effect: "crowd-applause"
/// ```
#[derive(Debug, Clone, Default, PartialEq)]
pub struct LifecycleConfig {
    /// Notification emitted at the `initialize` event.
    pub initialize: Option<LifecycleNotification>,

    /// Notification emitted when composition begins.
    pub start: Option<LifecycleNotification>,

    /// Notification emitted when composition succeeds.
    pub success: Option<LifecycleNotification>,

    /// Notification emitted when composition is blocked.
    pub blocked: Option<LifecycleNotification>,

    /// Notification emitted when composition fails.
    pub failure: Option<LifecycleNotification>,

    /// Notification emitted at the `finalize` event (once per iteration).
    pub finalize: Option<LifecycleNotification>,

    /// Lifecycle concerns authored inside the `loop:` block. The iteration
    /// controls themselves live on `LoopConfig`.
    pub loop_concerns: Option<LifecycleNotification>,

    /// Typed, validated stacks keyed by event.
    pub stacks: LifecycleStacks,
}

/// Per-event typed stacks, populated by [`parse_lifecycle_config`].
///
/// Each field is `None` when the event block has no `stack:` key. An empty
/// array never appears: a `stack: []` is normalized to `None`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct LifecycleStacks {
    /// Stack for the `initialize` event.
    pub initialize: Option<Vec<LifecycleStackItem>>,
    /// Stack for the `start` event.
    pub start: Option<Vec<LifecycleStackItem>>,
    /// Stack for the `success` event.
    pub success: Option<Vec<LifecycleStackItem>>,
    /// Stack for the `blocked` event.
    pub blocked: Option<Vec<LifecycleStackItem>>,
    /// Stack for the `failure` event.
    pub failure: Option<Vec<LifecycleStackItem>>,
    /// Stack for the `finalize` event.
    pub finalize: Option<Vec<LifecycleStackItem>>,
    /// Stack for the `loop` gate's lifecycle concerns.
    pub loop_gate: Option<Vec<LifecycleStackItem>>,
}

/// Lifecycle event signal types.
///
/// The seven composition lifecycle signals, in deterministic iteration
/// order: `Initialize`, `Start`, `Success`, `Blocked`, `Failure`,
/// `Finalize`, `Loop`. This order is used by validators that walk every
/// event surface (interpolation-leak scan, undefined-variable scan, `err`
/// scan) and is exposed via [`Self::all`].
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
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
    /// `failure` (or `blocked`) completes.
    Finalize,

    /// Post-`finalize` "iterate again?" gate; evaluates the document's
    /// `loop` while/until condition.
    Loop,
}

impl LifecycleSignal {
    /// All signals in deterministic iteration order.
    ///
    /// Used by validators that walk every event surface. New variants
    /// should be appended to the end so existing diagnostic order is
    /// preserved.
    pub const ALL: [Self; 7] = [
        Self::Initialize,
        Self::Start,
        Self::Success,
        Self::Blocked,
        Self::Failure,
        Self::Finalize,
        Self::Loop,
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

/// Trait for emitting lifecycle notification side effects.
///
/// Injectable to allow test doubles that capture emissions without hitting
/// real stderr, messaging, TTS, or sound playback.
pub trait LifecycleEmitter {
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
            .state(StatusState::Info)
            .theme(StatusTheme::Circular)
            .render(term);
        eprintln!("{rendered}");
    }

    /// Write a styled `Warning` status line to stderr.
    ///
    /// The status state is fixed to [`StatusState::Warning`] regardless of the
    /// owning lifecycle event.
    fn emit_warn(&self, text: &str, term: &Terminal) {
        let rendered = Status::from_prose(text)
            .state(StatusState::Warning)
            .theme(StatusTheme::Circular)
            .render(term);
        eprintln!("{rendered}");
    }

    /// Write a styled `Success` status line to stderr.
    ///
    /// The status state is fixed to [`StatusState::Success`] regardless of the
    /// owning lifecycle event.
    fn emit_success(&self, text: &str, term: &Terminal) {
        let rendered = Status::from_prose(text)
            .state(StatusState::Success)
            .theme(StatusTheme::Circular)
            .render(term);
        eprintln!("{rendered}");
    }

    /// Write a plain prose line (no status glyph) to **stdout**.
    ///
    /// The only lifecycle channel that targets stdout. Inline styling and links
    /// are honored, but no status decoration is applied. Authors opt into this
    /// knowing stdout is otherwise reserved for pipeable command data.
    fn emit_stdout(&self, text: &str, term: &Terminal) {
        let rendered = Prose::new(text).render(term);
        println!("{rendered}");
    }
}

/// Production emitter that performs real side effects.
pub struct DefaultLifecycleEmitter;

impl LifecycleEmitter for DefaultLifecycleEmitter {
    fn emit_stderr(&self, _signal: LifecycleSignal, text: &str, term: &Terminal) {
        // `stderr` carries no status: it is plain prose (rich text/links honored)
        // routed to STDERR. Status glyphs belong to `info`/`warn` only.
        let rendered = Prose::new(text).render(term);
        eprintln!("{rendered}");
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
        say_blocking(text, tts_config);
    }

    fn emit_effect(&self, name: &str) {
        play_effect_blocking(name);
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
/// The guard now tracks the full seven-event lifecycle:
/// `initialize` → `start` → (`success`|`blocked`|`failure`) → `finalize`.
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
    // The target document's rebuilt early-binding context, installed by the
    // R6 target launch rebuild once a proxied target's launch state is
    // recomputed from its own frontmatter (provider/model identity). Owned by
    // the guard so it can outlive the transient loop borrows that build it, and
    // preferred over both `ctx.context` (the source's snapshot) and the
    // demand-driven fallback so a proxied target's lifecycle `ctx.*`/`env.*`
    // (e.g. `env.MODEL`) resolves to the target's own resolved identity, exactly
    // as it does when the target is invoked directly.
    proxy_prepared_context: Option<darkmatter::markdown::compose::ComposeContext>,
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
            proxy_prepared_context: None,
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
            | LifecycleSignal::Failure => {
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
    /// downgrades the run to `failure`. The success/blocked top-level
    /// communication has **already** fired (top-level fires before the stack, per
    /// the spec) and that emission must remain — so the terminal slot was already
    /// taken by `Success`/`Blocked`. This overwrites only [`Self::terminal_signal`]
    /// to `Failure` while keeping [`Self::terminal_emitted`] true, letting the
    /// caller run the `failure` event's top-level + stack directly (without
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

    /// Run the top-level notification + typed stack for `signal` using the
    /// provided context.
    ///
    /// Callers must first call [`Self::record_event_emission`] and only run
    /// the stack when it returns `true`.
    pub fn run_event_stack(
        &self,
        signal: LifecycleSignal,
        stack_ctx: &super::lifecycle_executor::StackExecutionContext<'_>,
    ) -> super::lifecycle_executor::LifecycleEventOutcome {
        stack_ctx.with_signal(signal).execute_event(&self.config)
    }

    /// Execute the full lifecycle event (top-level notification + stack) for
    /// `signal`, tracking the guard's state machine.
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
        self.proxy_prepared_context = None;
    }

    /// Install the proxied target's rebuilt early-binding context.
    ///
    /// The R6 target launch rebuild recomputes a proxied target's launch
    /// identity (provider/model → `env.AGENT`/`env.MODEL`) from the target's own
    /// frontmatter, captures a `ComposeContext` carrying that identity, and hands
    /// it here. From this point [`Self::effective_prepared_context`] returns it,
    /// so the target's lifecycle `ctx.*`/`env.*` resolve to the same values a
    /// direct invocation of the target would produce, rather than the source's
    /// snapshot or an identity-less demand capture.
    pub fn set_proxy_prepared_context(
        &mut self,
        context: darkmatter::markdown::compose::ComposeContext,
    ) {
        self.proxy_prepared_context = Some(context);
    }

    /// The early-binding `ctx.*`/`env.*` snapshot lifecycle events should use,
    /// or `None` to force a demand-driven per-expression re-capture.
    ///
    /// Returns the composition-start snapshot for a normally-run document.
    /// After a `proxy` hand-off, once the R6 launch rebuild has installed the
    /// target's context via [`Self::set_proxy_prepared_context`], that rebuilt
    /// context wins — it carries the target's own resolved provider/model
    /// identity, so `env.MODEL`/`ctx.model` in the target's stacks match a direct
    /// invocation. Before the rebuild installs it (or if none is installed) a
    /// proxied guard returns `None`: the *original* document's snapshot omits any
    /// `ctx.*` group only the target references, so it is dropped and the
    /// executor re-captures at the launch area, exactly as the target's own body
    /// composition already does.
    pub fn effective_prepared_context(
        &self,
    ) -> Option<&darkmatter::markdown::compose::ComposeContext> {
        // A rebuilt target context wins: it carries the proxied target's own
        // resolved provider/model identity (R6), so it is preferred over both
        // the source's snapshot and the demand-driven fallback.
        if let Some(context) = self.proxy_prepared_context.as_ref() {
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

    /// Emit a single lifecycle signal through the injected emitter.
    ///
    /// Short-circuits on `crate::interrupt::interrupted()` so a Ctrl+C
    /// during execution skips all blocking post-execute side effects
    /// (messenger sends, desktop notifications, TTS playback, sound
    /// effects). The cheap stderr line still emits so the user sees the
    /// terminal status before the process exits.
    fn emit_signal(&self, signal: LifecycleSignal) {
        let Some(notification) = self.config.get(signal) else {
            return;
        };

        let interrupted = crate::interrupt::interrupted();

        // --- Non-audio fan-out (immediate) ---
        if let Some(stderr_text) = &notification.stderr {
            self.emitter.emit_stderr(signal, stderr_text, self.ctx.term);
        }
        if interrupted {
            return;
        }
        if let Some(message_text) = &notification.message {
            self.emitter.emit_message(
                message_text,
                self.ctx.source_path,
                self.ctx.repo_root,
                self.ctx.messaging,
            );
        }
        if let Some(notify_title) = &notification.notify {
            self.emitter.emit_notification(notify_title);
        }

        // --- Audio phases (sequential, blocking, lazy TTS config) ---
        let phases = audio_phases(notification);
        let mut tts_config: Option<TtsConfig> = None;

        for phase in phases {
            if crate::interrupt::interrupted() {
                return;
            }
            match phase {
                AudioPhase::Speak(text) => {
                    let config = tts_config
                        .get_or_insert_with(|| {
                            tts_config_from_settings(self.ctx.settings.tts.as_ref())
                        })
                        .clone();
                    self.emitter.emit_speech(&text, config);
                }
                AudioPhase::Effect(name) => self.emitter.emit_effect(&name),
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

/// Runtime context required for emitting lifecycle notifications.
///
/// Holds references to settings, messaging configuration, terminal, and paths
/// needed to resolve and emit lifecycle notifications.
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

/// A single audio playback phase.
impl LifecycleConfig {
    /// Returns the notification for a given signal, if configured.
    ///
    /// For [`LifecycleSignal::Loop`], this returns the lifecycle concerns
    /// authored inside the `loop:` block (alongside the iteration controls
    /// that live on `LoopConfig`).
    ///
    /// ## Examples
    ///
    /// ```
    /// # use claudine::composition::{LifecycleConfig, LifecycleSignal};
    /// let config = LifecycleConfig::default();
    /// assert!(config.get(LifecycleSignal::Start).is_none());
    /// ```
    pub fn get(&self, signal: LifecycleSignal) -> Option<&LifecycleNotification> {
        match signal {
            LifecycleSignal::Initialize => self.initialize.as_ref(),
            LifecycleSignal::Start => self.start.as_ref(),
            LifecycleSignal::Success => self.success.as_ref(),
            LifecycleSignal::Blocked => self.blocked.as_ref(),
            LifecycleSignal::Failure => self.failure.as_ref(),
            LifecycleSignal::Finalize => self.finalize.as_ref(),
            LifecycleSignal::Loop => self.loop_concerns.as_ref(),
        }
    }

    /// Returns the typed stack for a given signal, if any.
    ///
    /// `None` means the event block had no `stack:` key (or the event is
    /// absent entirely). An empty stack is normalized to `None`.
    pub fn stack(&self, signal: LifecycleSignal) -> Option<&[LifecycleStackItem]> {
        let stack = match signal {
            LifecycleSignal::Initialize => self.stacks.initialize.as_ref(),
            LifecycleSignal::Start => self.stacks.start.as_ref(),
            LifecycleSignal::Success => self.stacks.success.as_ref(),
            LifecycleSignal::Blocked => self.stacks.blocked.as_ref(),
            LifecycleSignal::Failure => self.stacks.failure.as_ref(),
            LifecycleSignal::Finalize => self.stacks.finalize.as_ref(),
            LifecycleSignal::Loop => self.stacks.loop_gate.as_ref(),
        };
        stack.map(Vec::as_slice)
    }

    /// Returns a mutable reference to the typed stack for a given signal, if
    /// any.
    ///
    /// Used by the pre-flight shell resolution pass (C3) to stamp resolved
    /// command bytes back into `ShellAction::command` / `on_error` so the
    /// approved command equals the executed command.
    pub fn stack_mut(
        &mut self,
        signal: LifecycleSignal,
    ) -> Option<&mut Vec<LifecycleStackItem>> {
        let stack = match signal {
            LifecycleSignal::Initialize => &mut self.stacks.initialize,
            LifecycleSignal::Start => &mut self.stacks.start,
            LifecycleSignal::Success => &mut self.stacks.success,
            LifecycleSignal::Blocked => &mut self.stacks.blocked,
            LifecycleSignal::Failure => &mut self.stacks.failure,
            LifecycleSignal::Finalize => &mut self.stacks.finalize,
            LifecycleSignal::Loop => &mut self.stacks.loop_gate,
        };
        stack.as_mut()
    }

    /// Returns `true` if no lifecycle notifications and no stacks are
    /// configured across any event.
    ///
    /// ## Examples
    ///
    /// ```
    /// # use claudine::composition::LifecycleConfig;
    /// let config = LifecycleConfig::default();
    /// assert!(config.is_empty());
    /// ```
    pub fn is_empty(&self) -> bool {
        self.initialize.is_none()
            && self.start.is_none()
            && self.success.is_none()
            && self.blocked.is_none()
            && self.failure.is_none()
            && self.finalize.is_none()
            && self.loop_concerns.is_none()
            && self.stacks.initialize.is_none()
            && self.stacks.start.is_none()
            && self.stacks.success.is_none()
            && self.stacks.blocked.is_none()
            && self.stacks.failure.is_none()
            && self.stacks.finalize.is_none()
            && self.stacks.loop_gate.is_none()
    }
}

/// Removed harness validation/handler DSL keys and their lifecycle
/// replacements.
///
/// These top-level frontmatter keys were retired when the lifecycle stack
/// model replaced the harness validation and handler execution layers.
#[cfg(test)]
mod tests;
