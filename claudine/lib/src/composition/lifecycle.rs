//! Lifecycle notification types and parsing for composition frontmatter.
//!
//! This module provides support for the seven composition lifecycle events
//! (`initialize`, `start`, `success`, `blocked`, `failure`, `finalize`,
//! `loop`) in composition frontmatter. Each event may carry top-level
//! communication properties (`say`, `say_first`, `effect`, `message`,
//! `stderr`, `notify`, `info`, `warn`) and an ordered `stack:` of
//! conditional actions. The `loop` event additionally carries iteration
//! controls parsed by [`super::loop_config::resolve_loop_config`].

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
use super::lifecycle_actions::{
    all_lifecycle_verbs, CommunicationAction, CommunicationChannel, ExpressionFunctionAction,
    expression_function_signature, is_known_lifecycle_verb, LifecycleAction, LifecycleActionKind,
    LifecycleControlAction, LifecycleStackItem, RetryBackoff, rewrite_to_positional, ShellAction,
    SideEffectAction, side_effect_signature,
};
use crate::events::{GlobalSettings, TtsSettings};
use crate::messaging::RuntimeMessagingSettings;

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
/// [`super::loop_config::resolve_loop_config`] and evaluated by the loop
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
pub const LATE_BINDING_ROOTS: &[&str] = &["err", "timing", "current"];

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
}

/// A single audio playback phase.
#[derive(Debug)]
#[cfg_attr(test, derive(PartialEq))]
pub(crate) enum AudioPhase {
    Speak(String),
    Effect(String),
}

/// Compute the ordered audio phases for a notification.
///
/// When both speech and effect are present:
/// - `say` + `effect` → effect first, then speech
/// - `say_first` + `effect` → speech first, then effect
///
/// When only one audio output is present, it is the sole phase.
pub(crate) fn audio_phases(n: &LifecycleNotification) -> Vec<AudioPhase> {
    let speech_text = n
        .say
        .as_deref()
        .or(n.say_first.as_deref())
        .filter(|s| !s.is_empty());
    let effect_name = n.effect.as_deref().filter(|s| !s.is_empty());
    let speech_first = n.say_first.is_some();

    match (speech_text, effect_name) {
        (Some(text), Some(effect)) if speech_first => {
            vec![
                AudioPhase::Speak(text.to_string()),
                AudioPhase::Effect(effect.to_string()),
            ]
        }
        (Some(text), Some(effect)) => {
            vec![
                AudioPhase::Effect(effect.to_string()),
                AudioPhase::Speak(text.to_string()),
            ]
        }
        (Some(text), None) => vec![AudioPhase::Speak(text.to_string())],
        (None, Some(effect)) => vec![AudioPhase::Effect(effect.to_string())],
        (None, None) => vec![],
    }
}

impl LifecycleSignal {
    /// Returns the frontmatter property name for this signal.
    ///
    /// ## Examples
    ///
    /// ```
    /// # use claudine::composition::LifecycleSignal;
    /// assert_eq!(LifecycleSignal::Start.property_name(), "start");
    /// assert_eq!(LifecycleSignal::Loop.property_name(), "loop");
    /// ```
    pub fn property_name(&self) -> &'static str {
        match self {
            Self::Initialize => "initialize",
            Self::Start => "start",
            Self::Success => "success",
            Self::Blocked => "blocked",
            Self::Failure => "failure",
            Self::Finalize => "finalize",
            Self::Loop => "loop",
        }
    }

    /// Returns the status state for this signal.
    ///
    /// ## Examples
    ///
    /// ```
    /// # use claudine::composition::LifecycleSignal;
    /// # use biscuit_terminal::components::status::StatusState;
    /// assert_eq!(LifecycleSignal::Start.status_state(), StatusState::Info);
    /// assert_eq!(LifecycleSignal::Success.status_state(), StatusState::Success);
    /// assert_eq!(LifecycleSignal::Blocked.status_state(), StatusState::Error);
    /// assert_eq!(LifecycleSignal::Failure.status_state(), StatusState::Error);
    /// ```
    pub fn status_state(&self) -> StatusState {
        match self {
            Self::Initialize => StatusState::Info,
            Self::Start => StatusState::Info,
            Self::Success => StatusState::Success,
            Self::Blocked | Self::Failure => StatusState::Error,
            Self::Finalize => StatusState::Info,
            Self::Loop => StatusState::Info,
        }
    }

    /// Whether this event can ever observe an error and therefore may
    /// legitimately reference the lifecycle-stack-only `err` global.
    ///
    /// Per the spec's `err` static-scan rule:
    /// - `blocked` and `failure` always carry an error.
    /// - `finalize` *optionally* carries an error (success path: no error;
    ///   failure path: error present).
    /// - `initialize`, `start`, `success`, `loop` never carry an error.
    pub const fn can_carry_error(self) -> bool {
        matches!(self, Self::Blocked | Self::Failure | Self::Finalize)
    }

    /// Whether an *unintentional* action error during this event's stack
    /// must route the run to the `failure` event.
    ///
    /// Per the spec's action error-propagation table, setup-phase events
    /// (`initialize`, `start`, `blocked`) propagate an errored action to
    /// `failure` so the agent is never invoked with a broken environment.
    /// Terminal-phase events (`success`, `failure`, `finalize`, `loop`) log
    /// the error but leave the composition outcome unchanged.
    ///
    /// This governs only unintentional action errors. The explicit `Error`
    /// lifecycle action is a deliberate author choice and follows the
    /// separate "Where valid" transition table.
    pub const fn routes_action_error_to_failure(self) -> bool {
        matches!(self, Self::Initialize | Self::Start | Self::Blocked)
    }
}

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
const REMOVED_VALIDATION_KEYS: &[(&str, &str)] = &[
    (
        "pre_checks",
        "use the `initialize` or `start` lifecycle stack instead",
    ),
    (
        "post_checks",
        "use the `success` or `finalize` lifecycle stack instead",
    ),
    (
        "handle",
        "use a lifecycle `shell` action or other lifecycle action instead",
    ),
    (
        "deviate",
        "use a lifecycle `shell` action plus a recovery action (`retry`, `resume`, etc.) instead",
    ),
];

/// Prefix used for subject-specific handler keys that are also removed.
const HANDLE_PREFIX: &str = "handle_";

/// Scan frontmatter top-level keys for removed validation/handler DSL keys.
///
/// Returns the lexicographically first removed key found together with its
/// replacement guidance. This is called from composition preparation before
/// lifecycle event blocks are parsed so the diagnostic names the removed DSL
/// key rather than falling through to generic unknown-field handling.
pub(crate) fn scan_removed_validation_keys(
    frontmatter: &serde_json::Value,
) -> Option<(String, &'static str)> {
    let obj = frontmatter.as_object()?;
    let mut keys: Vec<&str> = obj.keys().map(String::as_str).collect();
    keys.sort_unstable();
    for key in keys {
        if let Some((_, replacement)) = REMOVED_VALIDATION_KEYS.iter().find(|(k, _)| *k == key) {
            return Some((key.to_string(), *replacement));
        }
        if let Some(suffix) = key.strip_prefix(HANDLE_PREFIX) {
            if !suffix.is_empty() {
                return Some((
                    key.to_string(),
                    "use the `blocked` or `failure` lifecycle recovery actions instead",
                ));
            }
        }
    }
    None
}

/// Parses lifecycle configuration from composition frontmatter.
///
/// Walks every event block (`initialize`, `start`, `success`, `blocked`,
/// `failure`, `finalize`) and extracts the top-level communication
/// properties into [`LifecycleConfig`]. Lifecycle concerns authored inside
/// the `loop:` block (alongside the iteration controls parsed by
/// [`super::loop_config::resolve_loop_config`]) are extracted into
/// [`LifecycleConfig::loop_concerns`].
///
/// For each event the raw `stack:` is parsed into typed
/// [`LifecycleStackItem`] values stored on [`LifecycleConfig::stacks`].
/// Stack parsing enforces the spec's cardinality rule (at most one
/// lifecycle control action per item; it must be last) and the per-event
/// "Where valid" matrix for control actions.
///
/// Validates mutual exclusivity of `say` and `say_first` and validates sound
/// effect names against the embedded `playa` catalog.
///
/// ## Returns
///
/// Returns `Ok(LifecycleConfig)` on success, or a `CompositionError` if
/// validation fails.
///
/// ## Errors
///
/// - [`CompositionError::LifecycleSayConflict`]: Both `say` and `say_first`
///   are present in the same event.
/// - [`CompositionError::LifecycleUnknownEffect`]: An unknown sound effect
///   name is referenced.
/// - [`CompositionError::LifecycleInvalid`]: A property failed to
///   deserialize (typically an unknown field).
/// - [`CompositionError::LifecycleStackInvalidShape`]: A stack item is
///   malformed (not an object, missing `action`, unknown key).
/// - [`CompositionError::LifecycleActionInvalidShortForm`] /
///   [`CompositionError::LifecycleActionInvalidLongForm`]: An action could
///   not be parsed.
/// - [`CompositionError::LifecycleActionPlacement`]: A control action
///   appears in an event where the spec's "Where valid" matrix forbids it.
/// - [`CompositionError::LifecycleMultipleLifecycleActions`] /
///   [`CompositionError::LifecycleActionOrder`]: Cardinality violation.
///
/// ## Examples
///
/// ```
/// # use serde_json::json;
/// # use claudine::composition::parse_lifecycle_config;
/// let frontmatter = json!({
///     "title": "My Composition",
///     "start": {
///         "message": "Starting..."
///     }
/// });
/// let config = parse_lifecycle_config(&frontmatter, std::path::Path::new("test.md")).unwrap();
/// assert!(config.start.is_some());
/// ```
pub fn parse_lifecycle_config(
    frontmatter: &serde_json::Value,
    source_file: &Path,
) -> Result<LifecycleConfig, CompositionError> {
    // Non-object frontmatter returns default
    let Some(fm_obj) = frontmatter.as_object() else {
        return Ok(LifecycleConfig::default());
    };

    let mut config = LifecycleConfig::default();

    // Top-level event blocks. Loop concerns are handled separately below
    // because they share the `loop:` block with iteration controls.
    for signal in [
        LifecycleSignal::Initialize,
        LifecycleSignal::Start,
        LifecycleSignal::Success,
        LifecycleSignal::Blocked,
        LifecycleSignal::Failure,
        LifecycleSignal::Finalize,
    ] {
        let property_name = signal.property_name();
        let Some(value) = fm_obj.get(property_name) else {
            continue;
        };

        // Skip null values
        if value.is_null() {
            continue;
        }

        let (notification, stack) =
            parse_event_block(signal, value, source_file, property_name)?;

        *event_notification_field_mut(signal, &mut config) = Some(notification);
        *event_stack_field_mut(signal, &mut config) = stack.filter(|s| !s.is_empty());
    }

    // Loop concerns: extract lifecycle concern keys from inside `loop:`.
    if let Some(loop_value) = fm_obj.get("loop")
        && !loop_value.is_null()
    {
        let property_name = LifecycleSignal::Loop.property_name();
        let Some(loop_obj) = loop_value.as_object() else {
            return Err(CompositionError::LifecycleInvalid {
                property: property_name.to_string(),
                message: format!(
                    "`loop` must be an object, got {}",
                    json_type_name(loop_value)
                ),
                source_file: source_file.to_path_buf(),
                unknown_field: None,
                expected_fields: LIFECYCLE_NOTIFICATION_FIELDS
                    .iter()
                    .map(|s| (*s).to_string())
                    .collect(),
            });
        };

        let mut concerns_obj = serde_json::Map::new();
        for key in LIFECYCLE_CONCERN_KEYS {
            if let Some(value) = loop_obj.get(*key) {
                concerns_obj.insert((*key).to_string(), value.clone());
            }
        }
        if !concerns_obj.is_empty() {
            let concerns_value = serde_json::Value::Object(concerns_obj);
            let (notification, stack) = parse_event_block(
                LifecycleSignal::Loop,
                &concerns_value,
                source_file,
                property_name,
            )?;
            config.loop_concerns = Some(notification);
            config.stacks.loop_gate = stack.filter(|s| !s.is_empty());
        }
    }

    Ok(config)
}

/// The lifecycle-concern keys accepted inside the `loop:` block (and on
/// every top-level event block).
///
/// Kept distinct from [`LIFECYCLE_NOTIFICATION_FIELDS`] (the comm-field
/// list used by serde-driven diagnostics) because this list also includes
/// `stack`, which is a structured field rather than a string.
pub(crate) const LIFECYCLE_CONCERN_KEYS: &[&str] = &[
    "say",
    "say_first",
    "effect",
    "message",
    "stderr",
    "notify",
    "info",
    "warn",
    "success",
    "stdout",
    "stack",
];

/// Parse one event block into a (notification, stack) pair.
///
/// `signal` carries the event name needed for stack parsing (cardinality
/// and "Where valid" enforcement). `property_name` is the frontmatter key
/// used in diagnostics (it always matches `signal.property_name()` but is
/// passed in to avoid repeated lookups).
fn parse_event_block(
    signal: LifecycleSignal,
    value: &serde_json::Value,
    source_file: &Path,
    property_name: &str,
) -> Result<(LifecycleNotification, Option<Vec<LifecycleStackItem>>), CompositionError> {
    let mut notification: LifecycleNotification = serde_json::from_value(value.clone()).map_err(
        |e| {
            let (unknown_field, expected_fields) = parse_serde_unknown_field(&e);
            CompositionError::LifecycleInvalid {
                property: property_name.to_string(),
                message: e.to_string(),
                source_file: source_file.to_path_buf(),
                unknown_field,
                expected_fields,
            }
        },
    )?;

    // Normalize empty strings to None for every string field.
    normalize_empty_string(&mut notification.say);
    normalize_empty_string(&mut notification.say_first);
    normalize_empty_string(&mut notification.effect);
    normalize_empty_string(&mut notification.message);
    normalize_empty_string(&mut notification.stderr);
    normalize_empty_string(&mut notification.notify);
    normalize_empty_string(&mut notification.info);
    normalize_empty_string(&mut notification.warn);
    normalize_empty_string(&mut notification.success);
    normalize_empty_string(&mut notification.stdout);

    // Validate mutual exclusivity of say and say_first.
    if notification.say.is_some() && notification.say_first.is_some() {
        return Err(CompositionError::LifecycleSayConflict(
            property_name.to_string(),
        ));
    }

    // Validate effect name if present and free of interpolation. An
    // `effect: "{{name}}"` is deferred: its real name is only known after
    // event-time interpolation, so it is validated then (see
    // [`super::lifecycle_executor`]'s deferred effect validation), not here.
    if let Some(effect_name) = &notification.effect
        && !effect_name.contains("{{")
        && playa::SoundEffect::from_name(effect_name).is_none()
    {
        return Err(CompositionError::LifecycleUnknownEffect(
            property_name.to_string(),
            effect_name.clone(),
        ));
    }

    // Parse the raw stack into typed form, enforcing cardinality + "Where
    // valid" matrix for this event.
    let typed_stack = match notification.stack.take() {
        Some(raw_stack) if !raw_stack.is_empty() => Some(parse_lifecycle_stack(
            signal,
            &raw_stack,
            source_file,
        )?),
        _ => None,
    };

    Ok((notification, typed_stack))
}

/// Parse a raw stack (`Vec<Value>`) into typed form for the given event.
fn parse_lifecycle_stack(
    signal: LifecycleSignal,
    raw_stack: &[serde_json::Value],
    source_file: &Path,
) -> Result<Vec<LifecycleStackItem>, CompositionError> {
    let property_name = signal.property_name();
    let mut items = Vec::with_capacity(raw_stack.len());
    for (idx, raw_item) in raw_stack.iter().enumerate() {
        let item = parse_lifecycle_stack_item(signal, raw_item, source_file)
            .map_err(|e| annotate_stack_error(e, property_name, idx))?;
        items.push(item);
    }
    Ok(items)
}

/// Attach the stack-item index to a parse error so the diagnostic can name
/// `start.stack[2]` rather than just `start`.
fn annotate_stack_error(err: CompositionError, property: &str, idx: usize) -> CompositionError {
    let dotted = format!("{property}.stack[{idx}]");
    match err {
        CompositionError::LifecycleStackInvalidShape {
            source_path,
            property: _,
            message,
        } => CompositionError::LifecycleStackInvalidShape {
            source_path,
            property: dotted,
            message,
        },
        CompositionError::LifecycleActionInvalidShortForm {
            source_path,
            property: _,
            raw,
            message,
        } => CompositionError::LifecycleActionInvalidShortForm {
            source_path,
            property: dotted,
            raw,
            message,
        },
        CompositionError::LifecycleActionInvalidLongForm {
            source_path,
            property: _,
            action,
            message,
        } => CompositionError::LifecycleActionInvalidLongForm {
            source_path,
            property: dotted,
            action,
            message,
        },
        CompositionError::LifecycleActionPlacement {
            source_path,
            property: _,
            action,
            event,
        } => CompositionError::LifecycleActionPlacement {
            source_path,
            property: dotted,
            action,
            event,
        },
        CompositionError::LifecycleMultipleLifecycleActions {
            source_path,
            property: _,
        } => CompositionError::LifecycleMultipleLifecycleActions {
            source_path,
            property: dotted,
        },
        CompositionError::LifecycleActionOrder {
            source_path,
            property: _,
        } => CompositionError::LifecycleActionOrder {
            source_path,
            property: dotted,
        },
        CompositionError::LifecycleInvalidArgs {
            source_path,
            property: _,
            action,
            message,
        } => CompositionError::LifecycleInvalidArgs {
            source_path,
            property: dotted,
            action,
            message,
        },
        other => other,
    }
}

/// Parse a single stack item.
///
/// Stack item schema (per the lifecycle spec):
///
/// ```yaml
/// - when: <optional condition>
///   action: <scalar string | array of object>
///   no_error: <optional boolean>
///   # A scalar `action` value must be a bare verb name with zero arguments
///   # (e.g. `stop`, `skip`). The universal `no_error` flag may appear as a
///   # sibling key. Key/value parameters must live inside an explicit object:
///   # `{ action: verb, ... }`. Array elements are self-contained positional
///   # single-key objects (`{verb: value}`) or key/value objects.
/// ```
fn parse_lifecycle_stack_item(
    signal: LifecycleSignal,
    raw_item: &serde_json::Value,
    source_file: &Path,
) -> Result<LifecycleStackItem, CompositionError> {
    let property_name = signal.property_name();

    let obj = raw_item.as_object().ok_or_else(|| {
        CompositionError::LifecycleStackInvalidShape {
            source_path: source_file.to_path_buf(),
            property: property_name.to_string(),
            message: format!(
                "stack item must be an object, got {}",
                json_type_name(raw_item)
            ),
        }
    })?;

    // Parse `when:` as a Darkmatter condition expression.
    //
    // This is the intentional exception to the literal-default rule: `when`,
    // `until`, and `while` are always boolean expressions and must never be
    // routed through `action_value_to_expr`.
    let when = match obj.get("when") {
        Some(serde_json::Value::String(s)) => {
            Some(parse_condition(s).map_err(|e| CompositionError::LifecycleStackInvalidShape {
                source_path: source_file.to_path_buf(),
                property: property_name.to_string(),
                message: format!("`when` is not a valid expression: {e}"),
            })?)
        }
        Some(other) => {
            return Err(CompositionError::LifecycleStackInvalidShape {
                source_path: source_file.to_path_buf(),
                property: property_name.to_string(),
                message: format!("`when` must be a string, got {}", json_type_name(other)),
            });
        }
        None => None,
    };

    let raw_action = obj.get("action").ok_or_else(|| {
        CompositionError::LifecycleStackInvalidShape {
            source_path: source_file.to_path_buf(),
            property: property_name.to_string(),
            message: "stack item must have an `action` key".to_string(),
        }
    })?;

    // Collect the universal `no_error` flag. Sibling parameter keys are no
    // longer accepted at the stack-item level: a scalar `action: <verb>` must
    // be a bare verb with zero arguments, and key/value parameters must live
    // inside an explicit `{ action: verb, ... }` object.
    let mut stack_no_error: Option<bool> = None;
    let mut sibling_keys: Vec<&str> = Vec::new();
    for (key, value) in obj {
        if matches!(key.as_str(), "when" | "action") {
            continue;
        }
        if key == "no_error" {
            stack_no_error = Some(match value {
                serde_json::Value::Bool(b) => *b,
                other => {
                    return Err(CompositionError::LifecycleStackInvalidShape {
                        source_path: source_file.to_path_buf(),
                        property: property_name.to_string(),
                        message: format!(
                            "`no_error` must be a boolean, got {}",
                            json_type_name(other)
                        ),
                    });
                }
            });
            continue;
        }
        sibling_keys.push(key.as_str());
    }
    if !sibling_keys.is_empty() {
        sibling_keys.sort_unstable();
        let message = if let serde_json::Value::String(verb) = raw_action {
            format!(
                "scalar `action` value `{verb}` cannot take sibling parameter(s) ({}); \
                 use the explicit key/value form `{{{{ action: {verb}, ... }}}}``",
                sibling_keys.join(", ")
            )
        } else {
            format!(
                "stack item with an {} `action` cannot carry sibling parameter(s) ({}); \
                 move them into each array element",
                json_type_name(raw_action),
                sibling_keys.join(", ")
            )
        };
        return Err(CompositionError::LifecycleStackInvalidShape {
            source_path: source_file.to_path_buf(),
            property: property_name.to_string(),
            message,
        });
    }

    let actions = match raw_action {
        // Scalar string action: only bare verb-name zero-arg form is accepted.
        // Any string containing `(` is the removed short-form grammar.
        serde_json::Value::String(s) => {
            let no_error = stack_no_error.unwrap_or(false);
            let action = parse_scalar_action(signal, s, no_error, source_file, property_name)?;
            vec![action]
        }
        // Array of actions, each self-contained.
        serde_json::Value::Array(items) => {
            if stack_no_error.is_some() {
                return Err(CompositionError::LifecycleStackInvalidShape {
                    source_path: source_file.to_path_buf(),
                    property: property_name.to_string(),
                    message: "stack item with an array `action` cannot carry a sibling `no_error`; \
                        move `no_error` into each array element"
                        .to_string(),
                });
            }
            let mut actions = Vec::with_capacity(items.len());
            for item in items {
                let action = match item {
                    serde_json::Value::String(s) => {
                        if s.contains('(') {
                            return Err(CompositionError::LifecycleShortFormRemoved {
                                source_path: source_file.to_path_buf(),
                                property: property_name.to_string(),
                                raw: s.clone(),
                                rewrite: rewrite_to_positional(s),
                            });
                        }
                        parse_bare_verb_string(signal, s, false, source_file, property_name)?
                    }
                    serde_json::Value::Object(inner) => {
                        parse_stack_item_action_object(
                            signal,
                            inner,
                            source_file,
                            property_name,
                        )?
                    }
                    other => {
                        return Err(CompositionError::LifecycleStackInvalidShape {
                            source_path: source_file.to_path_buf(),
                            property: property_name.to_string(),
                            message: format!(
                                "array `action` element must be a string or object, got {}",
                                json_type_name(other)
                            ),
                        });
                    }
                };
                actions.push(action);
            }
            actions
        }
        // Object form: either a positional single-key object (`{success: "x"}`)
        // or a nested key/value object (`{action: {action: verb, ...}}`).
        serde_json::Value::Object(obj) => {
            vec![parse_stack_item_action_object(
                signal,
                obj,
                source_file,
                property_name,
            )?]
        }
        other => {
            return Err(CompositionError::LifecycleStackInvalidShape {
                source_path: source_file.to_path_buf(),
                property: property_name.to_string(),
                message: format!(
                    "`action` must be a string or array, got {}",
                    json_type_name(other)
                ),
            });
        }
    };

    // Cardinality: at most one lifecycle control action, and it must be last.
    let lifecycle_indices: Vec<usize> = actions
        .iter()
        .enumerate()
        .filter_map(|(i, a)| a.is_lifecycle_control().then_some(i))
        .collect();
    if lifecycle_indices.len() > 1 {
        return Err(CompositionError::LifecycleMultipleLifecycleActions {
            source_path: source_file.to_path_buf(),
            property: property_name.to_string(),
        });
    }
    if let Some(idx) = lifecycle_indices.first()
        && *idx != actions.len() - 1
    {
        return Err(CompositionError::LifecycleActionOrder {
            source_path: source_file.to_path_buf(),
            property: property_name.to_string(),
        });
    }

    // Per-event "Where valid" matrix for lifecycle control actions. Runs
    // after cardinality so the structural violation is reported first.
    for action in &actions {
        if let LifecycleActionKind::LifecycleControl(control) = &action.kind
            && !control.is_valid_for(signal)
        {
            return Err(CompositionError::LifecycleActionPlacement {
                source_path: source_file.to_path_buf(),
                property: property_name.to_string(),
                action: control.verb().to_string(),
                event: signal.property_name().to_string(),
            });
        }
    }

    Ok(LifecycleStackItem { when, actions })
}

/// Parse an action object that appears either as an array element or as the
/// single value of a stack item's `action:` key.
///
/// Disambiguation (per the positional-and-key-value spec):
/// - Object with an `action:` key → key/value long form.
/// - Single-key object whose key is a known verb → positional form.
/// - Single-key object whose key is not a known verb → unknown-verb error.
/// - Multi-key object without an `action:` key → ambiguous error.
fn parse_stack_item_action_object(
    signal: LifecycleSignal,
    obj: &serde_json::Map<String, serde_json::Value>,
    source_file: &Path,
    property_name: &str,
) -> Result<LifecycleAction, CompositionError> {
    if obj.contains_key("action") {
        return parse_long_form_action_object(signal, obj, source_file, property_name);
    }

    match obj.len() {
        0 => Err(CompositionError::LifecycleStackInvalidShape {
            source_path: source_file.to_path_buf(),
            property: property_name.to_string(),
            message: "action object cannot be empty".to_string(),
        }),
        1 => {
            let (verb, value) = obj.iter().next().expect("single-key object");
            parse_positional_action(signal, verb, value, source_file, property_name)
        }
        _ => {
            let mut keys: Vec<&str> = obj.keys().map(String::as_str).collect();
            keys.sort_unstable();
            let known_verb = keys.iter().copied().find(|k| is_known_lifecycle_verb(k));
            let positional_rewrite = known_verb.map(|verb| format!("`{verb}: ...`"));
            let kv_verb = known_verb.unwrap_or_else(|| keys.first().expect("multi-key object"));
            let kv_rewrite = format!("`{{{{ action: {kv_verb}, ... }}}}``");
            let mut rewrites: Vec<String> = Vec::new();
            if let Some(rw) = positional_rewrite {
                rewrites.push(rw);
            }
            rewrites.push(kv_rewrite);
            Err(CompositionError::LifecycleStackAmbiguous {
                source_path: source_file.to_path_buf(),
                property: property_name.to_string(),
                message: format!(
                    "multi-key action object without an `action` key is ambiguous; \
                     did you mean {}?",
                    rewrites.join(" or ")
                ),
            })
        }
    }
}

/// Parse a scalar string `action` value.
///
/// Only the bare verb-name zero-arg form is accepted. A string containing
/// `(` is the removed short-form grammar and surfaces a typed
/// [`CompositionError::LifecycleShortFormRemoved`] with a positional rewrite.
fn parse_scalar_action(
    signal: LifecycleSignal,
    raw: &str,
    no_error: bool,
    source_file: &Path,
    property_name: &str,
) -> Result<LifecycleAction, CompositionError> {
    let trimmed = raw.trim();

    if trimmed.is_empty() {
        return Err(CompositionError::LifecycleActionInvalidShortForm {
            source_path: source_file.to_path_buf(),
            property: property_name.to_string(),
            raw: raw.to_string(),
            message: "action cannot be empty".to_string(),
        });
    }

    if trimmed.contains('(') {
        return Err(CompositionError::LifecycleShortFormRemoved {
            source_path: source_file.to_path_buf(),
            property: property_name.to_string(),
            raw: raw.to_string(),
            rewrite: rewrite_to_positional(raw),
        });
    }

    parse_bare_verb_string(signal, trimmed, no_error, source_file, property_name)
}

/// Parse a bare verb-name string as a zero-arg positional action.
///
/// Validates that the verb is known and that its zero-arg form is arity-legal
/// (e.g. `stop` is accepted, `proxy` is rejected as wrong-arity).
fn parse_bare_verb_string(
    signal: LifecycleSignal,
    raw: &str,
    no_error: bool,
    source_file: &Path,
    property_name: &str,
) -> Result<LifecycleAction, CompositionError> {
    let trimmed = raw.trim();

    if trimmed.is_empty() {
        return Err(CompositionError::LifecycleActionInvalidShortForm {
            source_path: source_file.to_path_buf(),
            property: property_name.to_string(),
            raw: raw.to_string(),
            message: "action cannot be empty".to_string(),
        });
    }

    if !is_known_lifecycle_verb(trimmed) {
        let rewrite = did_you_mean_verb(trimmed)
            .map(|suggestion| format!("; did you mean `{suggestion}`?"))
            .unwrap_or_default();
        return Err(CompositionError::LifecycleUnknownVerb {
            source_path: source_file.to_path_buf(),
            property: property_name.to_string(),
            verb: trimmed.to_string(),
            rewrite,
        });
    }

    let mut action =
        validate_positional_arity_and_build(signal, trimmed, Vec::new(), source_file, property_name)?;
    action.no_error = no_error;
    Ok(action)
}

/// Parse one long-form action object (used inside an `action:` array).
///
/// The object's keys are the action's parameters, including `action:` as
/// the verb discriminator and the universal `no_error:` flag.
fn parse_long_form_action_object(
    signal: LifecycleSignal,
    obj: &serde_json::Map<String, serde_json::Value>,
    source_file: &Path,
    property_name: &str,
) -> Result<LifecycleAction, CompositionError> {
    let verb_value = obj.get("action").ok_or_else(|| {
        CompositionError::LifecycleActionInvalidLongForm {
            source_path: source_file.to_path_buf(),
            property: property_name.to_string(),
            action: "<missing>".to_string(),
            message: "long-form action object must have an `action` key".to_string(),
        }
    })?;
    let verb = match verb_value {
        serde_json::Value::String(s) => s.clone(),
        other => {
            return Err(CompositionError::LifecycleActionInvalidLongForm {
                source_path: source_file.to_path_buf(),
                property: property_name.to_string(),
                action: "<invalid>".to_string(),
                message: format!(
                    "`action` must be a string, got {}",
                    json_type_name(other)
                ),
            });
        }
    };

    if !is_known_lifecycle_verb(&verb) {
        let rewrite = did_you_mean_verb(&verb)
            .map(|suggestion| format!("; did you mean `{suggestion}`?"))
            .unwrap_or_default();
        return Err(CompositionError::LifecycleUnknownVerb {
            source_path: source_file.to_path_buf(),
            property: property_name.to_string(),
            verb: verb.clone(),
            rewrite,
        });
    }

    let no_error = match obj.get("no_error") {
        Some(serde_json::Value::Bool(b)) => *b,
        Some(other) => {
            return Err(CompositionError::LifecycleActionInvalidLongForm {
                source_path: source_file.to_path_buf(),
                property: property_name.to_string(),
                action: verb,
                message: format!(
                    "`no_error` must be a boolean, got {}",
                    json_type_name(other)
                ),
            });
        }
        None => false,
    };

    let mut params: Vec<(String, Expr)> = Vec::new();
    for (key, value) in obj {
        if matches!(key.as_str(), "action" | "no_error") {
            continue;
        }
        // Direct YAML object literals are not accepted as parameter values;
        // object data must be passed through a whole-value `{{ ... }}` span.
        if let serde_json::Value::Object(_) = value {
            return Err(CompositionError::LifecycleObjectDataThroughInterpolationParameter {
                source_path: source_file.to_path_buf(),
                property: property_name.to_string(),
                verb: verb.clone(),
                param: key.clone(),
            });
        }
        let expr = action_value_to_expr(value).map_err(|message| {
            CompositionError::LifecycleActionInvalidLongForm {
                source_path: source_file.to_path_buf(),
                property: property_name.to_string(),
                action: verb.clone(),
                message: format!("`{key}` is not a valid value: {message}"),
            }
        })?;
        params.push((key.clone(), expr));
    }

    build_action_from_params(signal, &verb, params, no_error, source_file)
}

/// Parse a positional action: `{verb: value}`.
///
/// The value is classified into arguments per the spec:
/// - scalar (string/number/bool) → 1 argument
/// - array → N arguments, each element converted independently
/// - null or empty array → 0 arguments
/// - direct object → rejected as object-data-through-interpolation
///
/// Arity is validated against the verb's signature (communication, shell,
/// control, side-effect, or expression-function) and produces a typed
/// [`CompositionError::LifecycleWrongArity`] when it does not match.
fn parse_positional_action(
    signal: LifecycleSignal,
    verb: &str,
    value: &serde_json::Value,
    source_file: &Path,
    property_name: &str,
) -> Result<LifecycleAction, CompositionError> {
    if !is_known_lifecycle_verb(verb) {
        let rewrite = did_you_mean_verb(verb)
            .map(|suggestion| format!("; did you mean `{suggestion}`?"))
            .unwrap_or_default();
        return Err(CompositionError::LifecycleUnknownVerb {
            source_path: source_file.to_path_buf(),
            property: property_name.to_string(),
            verb: verb.to_string(),
            rewrite,
        });
    }

    let args = classify_positional_value(verb, value, source_file, property_name)?;
    validate_positional_arity_and_build(signal, verb, args, source_file, property_name)
}

/// Classify a positional action value into zero or more [`Expr`] arguments.
fn classify_positional_value(
    verb: &str,
    value: &serde_json::Value,
    source_file: &Path,
    property_name: &str,
) -> Result<Vec<Expr>, CompositionError> {
    match value {
        serde_json::Value::Null => Ok(Vec::new()),
        serde_json::Value::String(_)
        | serde_json::Value::Number(_)
        | serde_json::Value::Bool(_) => Ok(vec![action_value_to_expr(value).map_err(|message| {
            CompositionError::LifecycleActionInvalidLongForm {
                source_path: source_file.to_path_buf(),
                property: property_name.to_string(),
                action: verb.to_string(),
                message,
            }
        })?]),
        serde_json::Value::Array(items) => {
            let mut args = Vec::with_capacity(items.len());
            for item in items {
                args.push(action_value_to_expr(item).map_err(|message| {
                    CompositionError::LifecycleActionInvalidLongForm {
                        source_path: source_file.to_path_buf(),
                        property: property_name.to_string(),
                        action: verb.to_string(),
                        message,
                    }
                })?);
            }
            Ok(args)
        }
        serde_json::Value::Object(_) => Err(
            CompositionError::LifecycleObjectDataThroughInterpolationPositional {
                source_path: source_file.to_path_buf(),
                property: property_name.to_string(),
                verb: verb.to_string(),
            },
        ),
    }
}

/// Validate the positional argument count for `verb` and build the typed
/// action.
fn validate_positional_arity_and_build(
    signal: LifecycleSignal,
    verb: &str,
    args: Vec<Expr>,
    source_file: &Path,
    property_name: &str,
) -> Result<LifecycleAction, CompositionError> {
    use LifecycleControlAction as A;

    // Lifecycle control verbs.
    let control = match verb {
        "stop" => {
            check_exact_positional_arity(verb, &args, 0, source_file, property_name)?;
            A::Stop
        }
        "skip" => {
            check_exact_positional_arity(verb, &args, 0, source_file, property_name)?;
            A::Skip
        }
        "error" => {
            check_optional_positional_arity(verb, &args, 0, 1, source_file, property_name)?;
            A::Error {
                reason: args.into_iter().next(),
            }
        }
        "proxy" => {
            check_exact_positional_arity(verb, &args, 1, source_file, property_name)?;
            A::Proxy {
                target: args.into_iter().next().expect("arity checked"),
            }
        }
        "retry" => {
            check_optional_positional_arity(verb, &args, 0, 1, source_file, property_name)?;
            A::Retry {
                max_attempts: args.into_iter().next(),
                backoff: None,
                delay: None,
            }
        }
        "resume" => {
            check_exact_positional_arity(verb, &args, 1, source_file, property_name)?;
            A::Resume {
                message: args.into_iter().next().expect("arity checked"),
                max_attempts: None,
            }
        }
        "defer" => {
            check_exact_positional_arity(verb, &args, 1, source_file, property_name)?;
            A::Defer {
                delay: args.into_iter().next().expect("arity checked"),
                reason: None,
            }
        }
        _ => {
            // Not a control verb; fall through to communication / shell /
            // side-effect / expression-function dispatch.
            return build_non_control_positional_action(
                signal, verb, args, source_file, property_name,
            );
        }
    };

    Ok(LifecycleAction {
        kind: LifecycleActionKind::LifecycleControl(control),
        no_error: false,
    })
}

/// Build a positional action for verbs that are not lifecycle controls.
fn build_non_control_positional_action(
    _signal: LifecycleSignal,
    verb: &str,
    args: Vec<Expr>,
    source_file: &Path,
    property_name: &str,
) -> Result<LifecycleAction, CompositionError> {
    // Communication channels.
    if let Some(channel) = CommunicationChannel::from_verb(verb) {
        check_exact_positional_arity(verb, &args, 1, source_file, property_name)?;
        return Ok(LifecycleAction {
            kind: LifecycleActionKind::Communication(CommunicationAction {
                channel,
                message: args.into_iter().next().expect("arity checked"),
                route: None,
            }),
            no_error: false,
        });
    }

    // Shell.
    if verb == "shell" {
        check_exact_positional_arity(verb, &args, 1, source_file, property_name)?;
        return Ok(LifecycleAction {
            kind: LifecycleActionKind::Shell(ShellAction {
                command: args.into_iter().next().expect("arity checked"),
                on_error: None,
            }),
            no_error: false,
        });
    }

    // Side-effect verbs.
    if let Some(signature) = side_effect_signature(verb) {
        check_positional_signature(verb, &args, &signature, source_file, property_name)?;
        return Ok(LifecycleAction {
            kind: LifecycleActionKind::SideEffect(SideEffectAction {
                verb: verb.to_string(),
                args,
            }),
            no_error: false,
        });
    }

    // Expression-function verbs.
    let signature = super::lifecycle_actions::expression_function_signature(verb)
        .unwrap_or_else(|| super::lifecycle_actions::Signature {
            verb: verb.to_string(),
            params: Vec::new(),
            optional_tail: 0,
            variadic: true,
        });
    check_positional_signature(verb, &args, &signature, source_file, property_name)?;
    Ok(LifecycleAction {
        kind: LifecycleActionKind::ExpressionFunction(ExpressionFunctionAction {
            function: verb.to_string(),
            args,
        }),
        no_error: false,
    })
}

/// Enforce an exact positional arity.
fn check_exact_positional_arity(
    verb: &str,
    args: &[Expr],
    expected: usize,
    source_file: &Path,
    property_name: &str,
) -> Result<(), CompositionError> {
    if args.len() != expected {
        let message = if expected == 0 {
            format!("`{verb}` takes no arguments, got {}", args.len())
        } else if expected == 1 {
            format!("`{verb}` expects exactly 1 argument, got {}", args.len())
        } else {
            format!("`{verb}` expects exactly {expected} arguments, got {}", args.len())
        };
        return Err(CompositionError::LifecycleWrongArity {
            source_path: source_file.to_path_buf(),
            property: property_name.to_string(),
            verb: verb.to_string(),
            message,
        });
    }
    Ok(())
}

/// Enforce a positional arity in the inclusive range `[min, max]`.
fn check_optional_positional_arity(
    verb: &str,
    args: &[Expr],
    min: usize,
    max: usize,
    source_file: &Path,
    property_name: &str,
) -> Result<(), CompositionError> {
    if args.len() < min || args.len() > max {
        let message = if min == max {
            format!("`{verb}` expects exactly {min} argument(s), got {}", args.len())
        } else {
            format!(
                "`{verb}` expects {min} to {max} argument(s), got {}",
                args.len()
            )
        };
        return Err(CompositionError::LifecycleWrongArity {
            source_path: source_file.to_path_buf(),
            property: property_name.to_string(),
            verb: verb.to_string(),
            message,
        });
    }
    Ok(())
}

/// Enforce positional arity against a descriptor signature.
fn check_positional_signature(
    verb: &str,
    args: &[Expr],
    signature: &super::lifecycle_actions::Signature,
    source_file: &Path,
    property_name: &str,
) -> Result<(), CompositionError> {
    let min = signature.required_count();
    let max = signature.max_count();

    let too_few = args.len() < min;
    let too_many = max.is_some_and(|m| args.len() > m);
    if too_few || too_many {
        let message = match max {
            None => format!(
                "`{verb}` is variadic and requires at least {min} argument(s), got {}; \
                 expected {}",
                args.len(),
                signature.params.join(", ")
            ),
            Some(max) if min == max => format!(
                "`{verb}` expects exactly {min} argument(s) ({}), got {}",
                signature.params.join(", "),
                args.len()
            ),
            Some(max) => format!(
                "`{verb}` expects {min} to {max} argument(s) ({}), got {}",
                signature.params.join(", "),
                args.len()
            ),
        };
        return Err(CompositionError::LifecycleWrongArity {
            source_path: source_file.to_path_buf(),
            property: property_name.to_string(),
            verb: verb.to_string(),
            message,
        });
    }
    Ok(())
}

/// Suggest a known lifecycle verb that is close to the unknown `verb`.
fn did_you_mean_verb(verb: &str) -> Option<&'static str> {
    use darkmatter::catalog::levenshtein;

    let normalized = verb.to_lowercase();
    let threshold = (normalized.chars().count() / 3).max(2);
    let mut best: Option<(usize, &'static str)> = None;
    for candidate in all_lifecycle_verbs() {
        let distance = levenshtein(&normalized, &candidate.to_lowercase());
        if distance <= threshold {
            if best.is_none_or(|(best_distance, _)| distance < best_distance) {
                best = Some((distance, candidate));
            }
        }
    }
    best.map(|(_, candidate)| candidate)
}

/// Convert a raw action parameter value into a Darkmatter `Expr` using the
/// single lifecycle evaluation rule.
///
/// - Strings are stored as [`Expr::StringLiteral`] unless the trimmed value is
///   exactly one `{{ … }}` interpolation span, in which case the inner
///   expression is parsed and its typed result is preserved (e.g. `{{ true }}`
///   becomes a bool, `{{ 3 }}` a number, `{{ payload }}` a variable reference
///   whose runtime value may be an object/array).
/// - YAML numeric and boolean scalars become [`Expr::NumberLiteral`] and
///   [`Expr::BoolLiteral`].
/// - Direct YAML objects and arrays are rejected; object/array data must be
///   passed through a whole-value interpolation span such as `{{ payload }}`.
///
/// The `when`/`until`/`while` keys are **not** action parameters — they remain
/// boolean expressions parsed by [`parse_condition`].
fn action_value_to_expr(value: &serde_json::Value) -> Result<Expr, String> {
    match value {
        serde_json::Value::String(s) => {
            let trimmed = s.trim();
            if trimmed.is_empty() {
                return Ok(Expr::StringLiteral(String::new()));
            }

            let spans = ExpressionFinder::find_all_plain(trimmed);
            if let Some(span) = spans.first()
                && spans.len() == 1
                && span.start == 0
                && span.end == trimmed.len()
            {
                // Whole-value expansion: preserve the expression's typed value.
                parse(&span.expression)
                    .map_err(|e| format!("whole-value expression is not valid: {e}"))
            } else {
                // Literal text, including mixed interpolation.
                Ok(Expr::StringLiteral(s.clone()))
            }
        }
        serde_json::Value::Number(n) => {
            let f = n
                .as_f64()
                .ok_or_else(|| format!("unsupported number `{n}`"))?;
            Ok(Expr::NumberLiteral(f))
        }
        serde_json::Value::Bool(b) => Ok(Expr::BoolLiteral(*b)),
        serde_json::Value::Null => Err(
            "null values are not supported as action parameters; use a whole-value `{{ null }}` interpolation to pass null"
                .to_string(),
        ),
        other => Err(format!(
            "{} values are not supported as action parameters; pass object/array data through a whole-value `{{{{ ... }}}}` interpolation",
            json_type_name(other)
        )),
    }
}

/// Build a typed action from an explicit key/value verb + named parameters.
///
/// Key/value form allows per-verb named parameters that don't fit the
/// positional `{verb: value}` shape (e.g. `shell`'s `on_error`, `retry`'s
/// `backoff`, `set_frontmatter`'s `file`/`prop`/`value`).
fn build_action_from_params(
    signal: LifecycleSignal,
    verb: &str,
    params: Vec<(String, Expr)>,
    no_error: bool,
    source_file: &Path,
) -> Result<LifecycleAction, CompositionError> {
    let property = signal.property_name();

    // Params are consumed via `params_map.remove(...)` as each verb reads its
    // known keys; any leftovers are reported by `reject_extra_params`.
    let mut params_map = params.into_iter().collect::<std::collections::HashMap<_, _>>();

    // Lifecycle control actions.
    if let Some(control) =
        parse_lifecycle_control_long(verb, &mut params_map, signal, source_file)?
    {
        reject_extra_params(control.verb(), &params_map, property, source_file)?;
        return Ok(LifecycleAction {
            kind: LifecycleActionKind::LifecycleControl(control),
            no_error,
        });
    }

    // Communication actions.
    if let Some(channel) = CommunicationChannel::from_verb(verb) {
        let message = params_map
            .remove("message")
            .or_else(|| params_map.remove("text"))
            .or_else(|| params_map.remove("sound"))
            .ok_or_else(|| CompositionError::LifecycleActionInvalidLongForm {
                source_path: source_file.to_path_buf(),
                property: property.to_string(),
                action: verb.to_string(),
                message: format!("`{verb}` requires a `message` parameter"),
            })?;
        let route = params_map.remove("route");
        reject_extra_params(verb, &params_map, property, source_file)?;
        return Ok(LifecycleAction {
            kind: LifecycleActionKind::Communication(CommunicationAction {
                channel,
                message,
                route,
            }),
            no_error,
        });
    }

    // Shell action: `command` required, `on_error` optional.
    if verb == "shell" {
        let command = params_map.remove("command").ok_or_else(|| {
            CompositionError::LifecycleActionInvalidLongForm {
                source_path: source_file.to_path_buf(),
                property: property.to_string(),
                action: verb.to_string(),
                message: "`shell` requires a `command` parameter".to_string(),
            }
        })?;
        let on_error = params_map.remove("on_error");
        reject_extra_params(verb, &params_map, property, source_file)?;
        return Ok(LifecycleAction {
            kind: LifecycleActionKind::Shell(ShellAction { command, on_error }),
            no_error,
        });
    }


    // Expression-function actions with concrete named parameters. Variadic
    // functions (`and`, `or`) are positional-only and reject key/value form.
    if let Some(signature) = expression_function_signature(verb) {
        if signature.variadic {
            return Err(CompositionError::LifecycleExpressionFunctionKeyValueUnsupported {
                source_path: source_file.to_path_buf(),
                property: property.to_string(),
                verb: verb.to_string(),
            });
        }
        let args =
            collect_named_signature_args(verb, &signature, &mut params_map, property, source_file)?;
        reject_extra_params(verb, &params_map, property, source_file)?;
        return Ok(LifecycleAction {
            kind: LifecycleActionKind::ExpressionFunction(ExpressionFunctionAction {
                function: verb.to_string(),
                args,
            }),
            no_error,
        });
    }

    // Side-effect action: any remaining verb with named params. The
    // Darkmatter catalog is the authority. For a verb with a known signature
    // we reorder the named params into positional call order (and reject any
    // param the signature does not name) so the executor can dispatch
    // positionally. For an unknown verb we keep the params in alphabetical
    // key order for deterministic storage; the executor surfaces it as an
    // unknown side effect at runtime.
    if let Some(signature) = super::lifecycle_actions::side_effect_signature(verb) {
        let args =
            collect_named_signature_args(verb, &signature, &mut params_map, property, source_file)?;
        reject_extra_params(verb, &params_map, property, source_file)?;
        return Ok(LifecycleAction {
            kind: LifecycleActionKind::SideEffect(SideEffectAction {
                verb: verb.to_string(),
                args,
            }),
            no_error,
        });
    }

    let mut named: Vec<(String, Expr)> = params_map.into_iter().collect();
    named.sort_by(|a, b| a.0.cmp(&b.0));
    let args = named.into_iter().map(|(_, v)| v).collect();
    Ok(LifecycleAction {
        kind: LifecycleActionKind::SideEffect(SideEffectAction {
            verb: verb.to_string(),
            args,
        }),
        no_error,
    })
}

/// Parse a long-form lifecycle control action by named parameters.
fn parse_lifecycle_control_long(
    verb: &str,
    params: &mut std::collections::HashMap<String, Expr>,
    signal: LifecycleSignal,
    source_file: &Path,
) -> Result<Option<LifecycleControlAction>, CompositionError> {
    use LifecycleControlAction as A;
    let property = signal.property_name();
    let invalid_args = |message: String| CompositionError::LifecycleInvalidArgs {
        source_path: source_file.to_path_buf(),
        property: property.to_string(),
        action: verb.to_string(),
        message,
    };

    let control = match verb {
        "stop" => A::Stop,
        "skip" => A::Skip,
        "error" => A::Error {
            reason: params.remove("reason"),
        },
        "proxy" => A::Proxy {
            target: params.remove("target").ok_or_else(|| {
                invalid_args("`proxy` requires a `target` parameter".to_string())
            })?,
        },
        "retry" => {
            let max_attempts = params.remove("max_attempts");
            let backoff = match params.remove("backoff") {
                Some(expr) => {
                    let raw = expr_to_string(&expr);
                    let parsed = RetryBackoff::parse(&raw).ok_or_else(|| {
                        invalid_args(format!(
                            "`retry.backoff` must be `fixed` or `exponential`, got `{raw}`"
                        ))
                    })?;
                    Some(parsed)
                }
                None => None,
            };
            let delay = params.remove("delay");
            A::Retry {
                max_attempts,
                backoff,
                delay,
            }
        }
        "resume" => A::Resume {
            message: params.remove("message").ok_or_else(|| {
                invalid_args("`resume` requires a `message` parameter".to_string())
            })?,
            max_attempts: params.remove("max_attempts"),
        },
        "defer" => A::Defer {
            delay: params.remove("delay").ok_or_else(|| {
                invalid_args("`requeue` requires a `delay` parameter".to_string())
            })?,
            reason: params.remove("reason"),
        },
        _ => return Ok(None),
    };
    Ok(Some(control))
}

/// Collect key/value action arguments in the descriptor's positional order,
/// enforcing the required-through-max parameter band at parse time.
///
/// Walks `signature.params` and consumes each named parameter present in
/// `params_map`. A parameter in the first [`required_count`] positions that is
/// absent is a parse-time error naming the missing required parameter(s);
/// optional-tail parameters being absent is allowed.
///
/// [`required_count`]: super::lifecycle_actions::Signature::required_count
fn collect_named_signature_args(
    verb: &str,
    signature: &super::lifecycle_actions::Signature,
    params_map: &mut std::collections::HashMap<String, Expr>,
    property: &str,
    source_file: &Path,
) -> Result<Vec<Expr>, CompositionError> {
    let required = signature.required_count();
    let mut args = Vec::with_capacity(signature.params.len());
    let mut missing: Vec<&str> = Vec::new();
    for (index, name) in signature.params.iter().enumerate() {
        match params_map.remove(name) {
            Some(expr) => args.push(expr),
            None if index < required => missing.push(name),
            None => {}
        }
    }
    if !missing.is_empty() {
        return Err(CompositionError::LifecycleActionInvalidLongForm {
            source_path: source_file.to_path_buf(),
            property: property.to_string(),
            action: verb.to_string(),
            message: format!(
                "`{verb}` is missing required parameter(s): {}",
                missing.join(", ")
            ),
        });
    }
    Ok(args)
}

/// Reject leftover parameters after a long-form action consumes its known
/// keys (so typos surface as parse-time errors).
fn reject_extra_params(
    verb: &str,
    leftovers: &std::collections::HashMap<String, Expr>,
    property: &str,
    source_file: &Path,
) -> Result<(), CompositionError> {
    if leftovers.is_empty() {
        return Ok(());
    }
    let mut keys: Vec<&str> = leftovers.keys().map(String::as_str).collect();
    keys.sort_unstable();
    Err(CompositionError::LifecycleActionInvalidLongForm {
        source_path: source_file.to_path_buf(),
        property: property.to_string(),
        action: verb.to_string(),
        message: format!("unknown parameter(s): {}", keys.join(", ")),
    })
}

/// Render an [`Expr`] back to a string for cases where a parameter is parsed
/// from an expression but needs a known domain (e.g. `backoff: "fixed"`).
fn expr_to_string(expr: &Expr) -> String {
    match expr {
        Expr::StringLiteral(s) => s.clone(),
        Expr::Variable(v) => v.clone(),
        other => other.to_string(),
    }
}

/// Get a mutable reference to the notification slot for a signal.
fn event_notification_field_mut(
    signal: LifecycleSignal,
    config: &mut LifecycleConfig,
) -> &mut Option<LifecycleNotification> {
    match signal {
        LifecycleSignal::Initialize => &mut config.initialize,
        LifecycleSignal::Start => &mut config.start,
        LifecycleSignal::Success => &mut config.success,
        LifecycleSignal::Blocked => &mut config.blocked,
        LifecycleSignal::Failure => &mut config.failure,
        LifecycleSignal::Finalize => &mut config.finalize,
        LifecycleSignal::Loop => &mut config.loop_concerns,
    }
}

/// Get a mutable reference to the typed stack slot for a signal.
fn event_stack_field_mut(
    signal: LifecycleSignal,
    config: &mut LifecycleConfig,
) -> &mut Option<Vec<LifecycleStackItem>> {
    match signal {
        LifecycleSignal::Initialize => &mut config.stacks.initialize,
        LifecycleSignal::Start => &mut config.stacks.start,
        LifecycleSignal::Success => &mut config.stacks.success,
        LifecycleSignal::Blocked => &mut config.stacks.blocked,
        LifecycleSignal::Failure => &mut config.stacks.failure,
        LifecycleSignal::Finalize => &mut config.stacks.finalize,
        LifecycleSignal::Loop => &mut config.stacks.loop_gate,
    }
}

/// JSON type label for diagnostics.
fn json_type_name(value: &serde_json::Value) -> &'static str {
    match value {
        serde_json::Value::Null => "null",
        serde_json::Value::Bool(_) => "boolean",
        serde_json::Value::Number(_) => "number",
        serde_json::Value::String(_) => "string",
        serde_json::Value::Array(_) => "array",
        serde_json::Value::Object(_) => "object",
    }
}

/// Validates that no rendered lifecycle string contains a surviving
/// `{{ … }}` interpolation span.
///
/// Walks every configured event in [`LifecycleSignal::ALL`] order and every
/// communication field on the notification (`say`, `say_first`, `message`,
/// `stderr`, `notify`, `info`, `warn`). Additionally walks every reachable
/// stack expression surface (when clauses, action arguments, communication
/// message bodies, shell commands, side-effect args, control-action
/// operands) and scans string literals inside those parsed expression trees
/// for surviving `{{ … }}` spans.
///
/// The first field or expression with a non-empty span list aborts with
/// [`CompositionError::LifecycleInterpolationLeak`].
///
/// This runs **after** composition, when expressions should have resolved.
/// It intentionally does *not* run inside [`parse_lifecycle_config`], which
/// is also used for raw frontmatter inspection where unresolved templates
/// are legitimate.
///
/// ## Arguments
///
/// * `config` — parsed lifecycle configuration.
/// * `source_path` — composed prompt file, used for the diagnostic.
/// * `warnings` — compose report warnings, used best-effort to enrich the
///   leak reason.
pub fn validate_no_interpolation_leaks(
    config: &LifecycleConfig,
    source_path: &Path,
    warnings: &[darkmatter::markdown::compose::ComposeWarning],
) -> Result<(), CompositionError> {
    for signal in LifecycleSignal::ALL {
        let Some(notification) = config.get(signal) else {
            continue;
        };

        for (field_name, value) in notification_comm_fields(notification) {
            let Some(text) = value else { continue };
            if text.is_empty() {
                continue;
            }

            let spans = ExpressionFinder::find_all_plain(text);
            if let Some(first) = spans.first() {
                let property = format!("{}.{}", signal.property_name(), field_name);
                let expression = first.expression.clone();
                let reason = find_matching_warning_reason(&expression, warnings);
                return Err(CompositionError::LifecycleInterpolationLeak {
                    source_path: source_path.to_path_buf(),
                    property,
                    expression,
                    reason,
                });
            }
        }
    }

    // Stack expression surfaces: scan string literals inside parsed Expr
    // trees for surviving `{{ … }}` spans. A string literal in a parsed
    // expression is passed through verbatim to the evaluated result, so a
    // literal containing template syntax would leak the raw braces into
    // user-visible output.
    for surface in iter_stack_expression_surfaces(config) {
        let mut found: Option<(String, String)> = None;
        visit_string_literals(surface.expr, &mut |literal| {
            if found.is_some() {
                return;
            }
            if let Some(span) = ExpressionFinder::find_all_plain(literal).first() {
                found = Some((span.expression.clone(), literal.to_string()));
            }
        });
        if let Some((expression, _literal)) = found {
            let reason = find_matching_warning_reason(&expression, warnings);
            return Err(CompositionError::LifecycleInterpolationLeak {
                source_path: source_path.to_path_buf(),
                property: surface.property,
                expression,
                reason,
            });
        }
    }

    Ok(())
}

/// Best-effort extraction of a warning reason mentioning the leaked expression.
fn find_matching_warning_reason(
    expression: &str,
    warnings: &[darkmatter::markdown::compose::ComposeWarning],
) -> String {
    let inner = expression
        .trim_start_matches("{{")
        .trim_end_matches("}}")
        .trim();

    for warning in warnings {
        if warning.message.contains(expression) || warning.message.contains(inner) {
            return warning.message.clone();
        }
    }

    String::new()
}

/// A single expression surface discovered by [`iter_stack_expression_surfaces`].
struct LifecycleExpressionSurface<'a> {
    /// Dotted property path for diagnostics, e.g. `start.stack[1].when` or
    /// `failure.stack[0].action`.
    property: String,
    /// The owning event — used by the `err` static scan to decide whether
    /// `err` references are permitted.
    signal: LifecycleSignal,
    /// The parsed expression tree.
    expr: &'a Expr,
}

/// Walk every reachable expression surface in every configured lifecycle
/// stack and yield it for scanning.
///
/// Surfaces include:
/// - `stack_item.when` (the condition expression)
/// - communication-action message expressions
/// - shell `command` and `on_error` expressions
/// - side-effect positional arguments
/// - expression-function positional arguments
/// - lifecycle-control action operands (`reason`, `target`, `max_attempts`,
///   `delay`, `message`)
///
/// The iteration order is deterministic: events are walked in
/// [`LifecycleSignal::ALL`] order, stack items in array order, actions in
/// execution order.
fn iter_stack_expression_surfaces<'a>(
    config: &'a LifecycleConfig,
) -> Vec<LifecycleExpressionSurface<'a>> {
    let mut surfaces = Vec::new();
    for signal in LifecycleSignal::ALL {
        let Some(stack) = config.stack(signal) else {
            continue;
        };
        let event_name = signal.property_name();
        for (idx, item) in stack.iter().enumerate() {
            let prefix = format!("{event_name}.stack[{idx}]");
            if let Some(when) = &item.when {
                surfaces.push(LifecycleExpressionSurface {
                    property: format!("{prefix}.when"),
                    signal,
                    expr: when,
                });
            }
            for (action_idx, action) in item.actions.iter().enumerate() {
                let action_prefix = format!("{prefix}.action[{action_idx}]");
                iter_action_expressions(&action.kind, &action_prefix, signal, &mut surfaces);
            }
        }
    }
    surfaces
}

/// Walk every parsed expression inside a single action body and append to
/// `surfaces`.
fn iter_action_expressions<'a>(
    kind: &'a LifecycleActionKind,
    prefix: &str,
    signal: LifecycleSignal,
    surfaces: &mut Vec<LifecycleExpressionSurface<'a>>,
) {
    match kind {
        LifecycleActionKind::LifecycleControl(control) => match control {
            LifecycleControlAction::Error { reason } => {
                if let Some(reason) = reason {
                    surfaces.push(LifecycleExpressionSurface {
                        property: format!("{prefix}.reason"),
                        signal,
                        expr: reason,
                    });
                }
            }
            LifecycleControlAction::Proxy { target } => surfaces.push(LifecycleExpressionSurface {
                property: format!("{prefix}.target"),
                signal,
                expr: target,
            }),
            LifecycleControlAction::Retry {
                max_attempts,
                delay,
                ..
            } => {
                if let Some(max_attempts) = max_attempts {
                    surfaces.push(LifecycleExpressionSurface {
                        property: format!("{prefix}.max_attempts"),
                        signal,
                        expr: max_attempts,
                    });
                }
                if let Some(delay) = delay {
                    surfaces.push(LifecycleExpressionSurface {
                        property: format!("{prefix}.delay"),
                        signal,
                        expr: delay,
                    });
                }
            }
            LifecycleControlAction::Resume {
                message,
                max_attempts,
            } => {
                surfaces.push(LifecycleExpressionSurface {
                    property: format!("{prefix}.message"),
                    signal,
                    expr: message,
                });
                if let Some(max_attempts) = max_attempts {
                    surfaces.push(LifecycleExpressionSurface {
                        property: format!("{prefix}.max_attempts"),
                        signal,
                        expr: max_attempts,
                    });
                }
            }
            LifecycleControlAction::Defer { delay, reason } => {
                surfaces.push(LifecycleExpressionSurface {
                    property: format!("{prefix}.delay"),
                    signal,
                    expr: delay,
                });
                if let Some(reason) = reason {
                    surfaces.push(LifecycleExpressionSurface {
                        property: format!("{prefix}.reason"),
                        signal,
                        expr: reason,
                    });
                }
            }
            LifecycleControlAction::Stop | LifecycleControlAction::Skip => {}
        },
        LifecycleActionKind::Communication(comm) => {
            surfaces.push(LifecycleExpressionSurface {
                property: format!("{prefix}.message"),
                signal,
                expr: &comm.message,
            });
            if let Some(route) = &comm.route {
                surfaces.push(LifecycleExpressionSurface {
                    property: format!("{prefix}.route"),
                    signal,
                    expr: route,
                });
            }
        }
        LifecycleActionKind::Shell(shell) => {
            surfaces.push(LifecycleExpressionSurface {
                property: format!("{prefix}.command"),
                signal,
                expr: &shell.command,
            });
            if let Some(on_error) = &shell.on_error {
                surfaces.push(LifecycleExpressionSurface {
                    property: format!("{prefix}.on_error"),
                    signal,
                    expr: on_error,
                });
            }
        }
        LifecycleActionKind::SideEffect(effect) => {
            for (i, arg) in effect.args.iter().enumerate() {
                surfaces.push(LifecycleExpressionSurface {
                    property: format!("{prefix}.arg[{i}]"),
                    signal,
                    expr: arg,
                });
            }
        }
        LifecycleActionKind::ExpressionFunction(func) => {
            for (i, arg) in func.args.iter().enumerate() {
                surfaces.push(LifecycleExpressionSurface {
                    property: format!("{prefix}.arg[{i}]"),
                    signal,
                    expr: arg,
                });
            }
        }
    }
}

/// Visit every `Expr::StringLiteral` reachable in the expression tree,
/// depth-first, calling `visitor` with the literal's value.
///
/// Used by the leak scan to detect surviving `{{ … }}` spans inside parsed
/// expression literals (e.g. `say('leaked {{ expr }}')`).
fn visit_string_literals<F: FnMut(&str)>(expr: &Expr, visitor: &mut F) {
    match expr {
        Expr::StringLiteral(s) => visitor(s),
        Expr::Variable(_) | Expr::NumberLiteral(_) | Expr::BoolLiteral(_) => {}
        Expr::UnaryNot(inner) | Expr::UnaryMinus(inner) | Expr::Paren(inner) => {
            visit_string_literals(inner, visitor);
        }
        Expr::Binary { left, right, .. } | Expr::Comparison { left, right, .. } => {
            visit_string_literals(left, visitor);
            visit_string_literals(right, visitor);
        }
        Expr::Index { base, index } => {
            visit_string_literals(base, visitor);
            visit_string_literals(index, visitor);
        }
        Expr::MemberAccess { base, .. } => visit_string_literals(base, visitor),
        Expr::FunctionCall { args, .. } => {
            for arg in args {
                visit_string_literals(arg, visitor);
            }
        }
        Expr::Fallback { primary, fallback } => {
            visit_string_literals(primary, visitor);
            visit_string_literals(fallback, visitor);
        }
        Expr::Ternary {
            condition,
            then_branch,
            else_branch,
        } => {
            visit_string_literals(condition, visitor);
            visit_string_literals(then_branch, visitor);
            visit_string_literals(else_branch, visitor);
        }
    }
}

/// Validates that no raw lifecycle string references a bare variable that is
/// undefined after composition, and that no lifecycle stack expression
/// references an undefined bare variable.
///
/// Darkmatter resolves an unknown bare variable to an empty string with no
/// warning and no error — even in fail-fast mode (see
/// `frontmatter_interpolation::missing_variable_resolves_to_empty`). So the
/// post-compose [`validate_no_interpolation_leaks`] guard, which only scans
/// the *rendered* string for surviving spans, never sees the collapsed
/// reference. This guard closes that gap by inspecting the **raw**
/// (pre-composition) lifecycle strings, where the `{{ … }}` span is still
/// present, and resolving each bare variable against the composed frontmatter.
///
/// The stack-expression half walks the parsed `Expr` trees on
/// [`LifecycleConfig::stacks`]. Bare names in stack expressions resolve
/// against the composed frontmatter plus the lifecycle globals
/// (`err`, `timing`, `current`) and the runtime namespaces (`ctx`, `env`,
/// `doc`); a bare name not in any of those is reported as undefined.
///
/// Every bare variable reachable in the parsed expression tree is checked, not
/// just spans that are exactly `{{ variable }}`: a missing operand buried in a
/// function argument (`{{ parent_dir(missing) }}`), comparison, or arithmetic
/// node is rejected the same way a top-level `{{ missing }}` is. A ternary
/// condition (`{{ missing ? 'a' : 'b' }}`) is descended because it is evaluated,
/// but the ternary branch operands and fallback (`{{ x || 'y' }}`) subtrees
/// intentionally tolerate undefined operands, so they are skipped. `ctx.*` /
/// `env.*` / `doc` references resolve from outside the frontmatter and are
/// skipped — a bare name resolves only against top-level frontmatter keys.
///
/// Iterates events in [`LifecycleSignal::ALL`] order and communication fields
/// in [`LIFECYCLE_COMM_FIELDS`] order; the first undefined variable aborts
/// with [`CompositionError::LifecycleUndefinedVariable`].
///
/// ## Arguments
///
/// * `raw_frontmatter` — the pre-composition frontmatter holding the original
///   lifecycle strings (`{{ … }}` spans intact).
/// * `effective_frontmatter` — the composed frontmatter object; a bare
///   variable is "defined" when its root segment is one of these keys.
/// * `lifecycle` — the parsed lifecycle configuration with typed stacks.
/// * `source_path` — prompt file, used for the diagnostic.
pub fn validate_no_undefined_lifecycle_variables(
    raw_frontmatter: &darkmatter::markdown::Frontmatter,
    effective_frontmatter: &serde_json::Value,
    lifecycle: &LifecycleConfig,
    source_path: &Path,
) -> Result<(), CompositionError> {
    let raw_map = raw_frontmatter.as_map();
    let defined = effective_frontmatter.as_object();

    // Top-level communication fields across all seven events.
    for signal in LifecycleSignal::ALL {
        let Some(serde_json::Value::Object(notification)) = raw_map.get(signal.property_name())
        else {
            continue;
        };

        for field in LIFECYCLE_COMM_FIELDS {
            let Some(serde_json::Value::String(text)) = notification.get(*field) else {
                continue;
            };

            for span in ExpressionFinder::find_all_plain(text) {
                let Ok(expr) = parse(&span.expression) else {
                    continue;
                };
                if let Some(variable) = find_undefined_top_level_variable(&expr, defined) {
                    return Err(CompositionError::LifecycleUndefinedVariable {
                        source_path: source_path.to_path_buf(),
                        property: format!("{}.{}", signal.property_name(), field),
                        variable: variable.to_string(),
                    });
                }
            }
        }
    }

    // Stack expression surfaces: walk parsed Expr trees for bare undefined
    // references. The lifecycle globals (err, timing, current) and runtime
    // namespaces (ctx, env, doc) are always considered defined here —
    // bare `err` misuse in no-error events is caught separately by
    // [`validate_no_err_in_no_error_events`].
    for surface in iter_stack_expression_surfaces(lifecycle) {
        if let Some(variable) = find_undefined_stack_variable(surface.expr, defined) {
            return Err(CompositionError::LifecycleUndefinedVariable {
                source_path: source_path.to_path_buf(),
                property: surface.property,
                variable: variable.to_string(),
            });
        }
    }

    Ok(())
}

/// Recursively walks `expr`, returning the first frontmatter-scoped bare
/// variable whose root key is undefined in the composed frontmatter.
///
/// Used for top-level communication fields. The runtime namespaces
/// (`ctx`/`env`/`doc`) and the lifecycle late-binding globals
/// ([`LATE_BINDING_ROOTS`]: `err`/`timing`/`current`) are known roots — they
/// resolve at event-time, not against frontmatter — so a bare `err`/`timing`/
/// `current` is not flagged. Only genuinely-unknown roots (typos) are reported.
///
/// A ternary condition is descended because it is evaluated during composition,
/// but the ternary branch operands and fallback (`||`) subtrees are not: those
/// forms exist precisely to tolerate an undefined operand, so a miss inside them
/// is intentional, not a leak. Every other node — function-call arguments,
/// comparisons, arithmetic, indexing, member access, unary, parens — is
/// descended so an undefined variable buried in `parent_dir(missing)` is caught
/// like a top-level `{{ missing }}`. The returned reference borrows from `expr`.
fn find_undefined_top_level_variable<'a>(
    expr: &'a Expr,
    defined: Option<&serde_json::Map<String, serde_json::Value>>,
) -> Option<&'a str> {
    match expr {
        Expr::Variable(path) => undefined_bare_variable(path, defined),
        // Ternary conditions are evaluated, but the branches intentionally
        // tolerate undefined operands by design.
        Expr::Ternary { condition, .. } => {
            find_undefined_top_level_variable(condition, defined)
        }
        Expr::Fallback { .. } => None,
        Expr::StringLiteral(_) | Expr::NumberLiteral(_) | Expr::BoolLiteral(_) => None,
        Expr::UnaryNot(inner) | Expr::UnaryMinus(inner) | Expr::Paren(inner) => {
            find_undefined_top_level_variable(inner, defined)
        }
        Expr::Binary { left, right, .. } | Expr::Comparison { left, right, .. } => {
            find_undefined_top_level_variable(left, defined)
                .or_else(|| find_undefined_top_level_variable(right, defined))
        }
        Expr::Index { base, index } => {
            find_undefined_top_level_variable(base, defined)
                .or_else(|| find_undefined_top_level_variable(index, defined))
        }
        Expr::MemberAccess { base, .. } => find_undefined_top_level_variable(base, defined),
        Expr::FunctionCall { args, .. } => args
            .iter()
            .find_map(|arg| find_undefined_top_level_variable(arg, defined)),
    }
}

/// Like [`find_undefined_top_level_variable`] but for stack expression
/// surfaces, where the lifecycle globals (`err`, `timing`, `current`) and
/// the runtime namespaces (`ctx`, `env`, `doc`) are always defined.
///
/// Stack `when:` clauses are parsed in condition mode, so `||`/`&&` lower to
/// `or(...)`/`and(...)` function calls rather than `Expr::Fallback`. Those two
/// functions get the same skip-the-operands tolerance a `Fallback` does — they
/// exist to guard an undefined operand.
fn find_undefined_stack_variable<'a>(
    expr: &'a Expr,
    defined: Option<&serde_json::Map<String, serde_json::Value>>,
) -> Option<&'a str> {
    match expr {
        Expr::Variable(path) => undefined_stack_variable(path, defined),
        Expr::Ternary { condition, .. } => find_undefined_stack_variable(condition, defined),
        Expr::Fallback { .. } => None,
        Expr::StringLiteral(_) | Expr::NumberLiteral(_) | Expr::BoolLiteral(_) => None,
        Expr::UnaryNot(inner) | Expr::UnaryMinus(inner) | Expr::Paren(inner) => {
            find_undefined_stack_variable(inner, defined)
        }
        Expr::Binary { left, right, .. } | Expr::Comparison { left, right, .. } => {
            find_undefined_stack_variable(left, defined)
                .or_else(|| find_undefined_stack_variable(right, defined))
        }
        Expr::Index { base, index } => {
            find_undefined_stack_variable(base, defined)
                .or_else(|| find_undefined_stack_variable(index, defined))
        }
        Expr::MemberAccess { base, .. } => find_undefined_stack_variable(base, defined),
        // `or`/`and` are the condition-parse-mode (`parse_condition`) lowering of
        // `||`/`&&`. Like an interpolation-mode `Expr::Fallback`, they exist to
        // tolerate an undefined/falsy operand by design (`maybe_missing || false`
        // is a guarded optional, not a typo), so their operands are not scanned.
        Expr::FunctionCall { name, .. } if name == "or" || name == "and" => None,
        Expr::FunctionCall { args, .. } => args
            .iter()
            .find_map(|arg| find_undefined_stack_variable(arg, defined)),
    }
}

/// Returns the first frontmatter-scoped bare variable in `expr` whose root key
/// is undefined, applying the lifecycle-stack tolerance (`ctx`/`env`/`doc` and
/// the late-binding globals `err`/`timing`/`current` are known roots; `||`
/// fallbacks are skipped and only a ternary's condition is descended).
///
/// Exposed for the executor's event-time `when:` guard, which fails closed on a
/// genuinely-unknown root rather than silently treating the guard as false.
/// Returns `None` when every root resolves. The reference borrows from `expr`.
pub(crate) fn first_undefined_stack_variable<'a>(
    expr: &'a Expr,
    defined: Option<&serde_json::Map<String, serde_json::Value>>,
) -> Option<&'a str> {
    find_undefined_stack_variable(expr, defined)
}

/// Whether `path`'s root resolves outside top-level frontmatter — the runtime
/// namespaces (`ctx.*` / `env.*` / `doc`) or a lifecycle late-binding global
/// ([`LATE_BINDING_ROOTS`]: `err`/`timing`/`current`).
///
/// Such a reference is never an undefined *frontmatter* variable, so the
/// undefined scan skips it. A bare `err` *misuse* in a no-error event is caught
/// separately by [`validate_no_err_in_no_error_events`].
fn resolves_outside_frontmatter(path: &str) -> bool {
    if path.starts_with("ctx.")
        || path.starts_with("env.")
        || path == "doc"
        || path.starts_with("doc.")
    {
        return true;
    }
    let root = path.split('.').next().unwrap_or(path);
    LATE_BINDING_ROOTS.contains(&root)
}

/// Returns the bare variable name when `path` is a frontmatter-scoped reference
/// whose root segment is absent from the composed frontmatter, or `None` when
/// it resolves elsewhere ([`resolves_outside_frontmatter`]) or its root key
/// exists.
///
/// Nested misses (`{{ a.b }}` where `a` exists but `b` does not) are treated as
/// defined: only the bare-root contract the spec describes is enforced.
fn undefined_bare_variable<'a>(
    path: &'a str,
    defined: Option<&serde_json::Map<String, serde_json::Value>>,
) -> Option<&'a str> {
    if resolves_outside_frontmatter(path) {
        return None;
    }
    let root = path.split('.').next().unwrap_or(path);
    if root.is_empty() {
        return None;
    }
    match defined {
        Some(map) if map.contains_key(root) => None,
        _ => Some(root),
    }
}

/// Identical to [`undefined_bare_variable`]: stack expression surfaces and
/// top-level fields now share one known-root contract (`ctx`/`env`/`doc` plus
/// the late-binding globals are known; only typos are flagged).
fn undefined_stack_variable<'a>(
    path: &'a str,
    defined: Option<&serde_json::Map<String, serde_json::Value>>,
) -> Option<&'a str> {
    undefined_bare_variable(path, defined)
}

/// Validates that the lifecycle-stack-only `err` global is not referenced
/// in events that never carry an error.
///
/// Per the spec's `err` static-scan rule:
/// - `initialize`, `start`, `success`, and `loop` never carry an error, so
///   any reference to the bare `err` global (or `err.*` member access) in
///   their stack surfaces is faulty logic.
/// - `blocked` and `failure` always carry an error; `finalize` optionally
///   carries one. References in those events are allowed.
/// - The `doc.err` escape hatch is exempt everywhere — it reaches a literal
///   frontmatter property named `err`, not the lifecycle global.
///
/// Two kinds of surface are scanned in events that cannot carry an error:
///
/// - **Communication/action strings** (top-level `say`/`message`/`stderr`/…
///   fields and single-parameter action message bodies) are literal text whose
///   only path to the `err` global is a `{{ … }}` interpolation span. Each span
///   is parsed and rejected when it references bare `err`.
/// - **Expression surfaces** (`when:` clauses, multi-argument expression-verb
///   args, control-action operands) evaluate the whole expression, so a bare
///   `err` reference anywhere in the tree is rejected.
///
/// `timing`/`current` are allowed everywhere; `doc.err` remains the escape hatch
/// (it reaches a literal frontmatter `err` property, not the lifecycle global).
/// The first violation aborts with [`CompositionError::LifecycleErrNotAvailable`].
pub fn validate_no_err_in_no_error_events(
    lifecycle: &LifecycleConfig,
    source_path: &Path,
) -> Result<(), CompositionError> {
    // Top-level communication fields: `err` reaches them only through a
    // `{{ … }}` span, so scan each span rather than the whole string.
    for signal in LifecycleSignal::ALL {
        if signal.can_carry_error() {
            continue;
        }
        let Some(notification) = lifecycle.get(signal) else {
            continue;
        };
        for (field_name, value) in notification_comm_fields(notification) {
            let Some(text) = value else { continue };
            if literal_spans_reference_err(text) {
                return Err(CompositionError::LifecycleErrNotAvailable {
                    source_path: source_path.to_path_buf(),
                    property: format!("{}.{}", signal.property_name(), field_name),
                    event: signal.property_name().to_string(),
                });
            }
        }
    }

    // Stack surfaces: an expression surface is rejected for a bare `err`
    // anywhere in its tree; a string literal (a single-parameter message body)
    // is rejected for a bare `err` inside any of its `{{ … }}` spans.
    for surface in iter_stack_expression_surfaces(lifecycle) {
        if surface.signal.can_carry_error() {
            continue;
        }
        if surface_references_err(surface.expr) {
            return Err(CompositionError::LifecycleErrNotAvailable {
                source_path: source_path.to_path_buf(),
                property: surface.property,
                event: surface.signal.property_name().to_string(),
            });
        }
    }
    Ok(())
}

/// Whether an expression surface references the lifecycle `err` global, either
/// as a bare expression reference or inside a `{{ … }}` span of a string literal
/// embedded in the tree (a single-parameter action message body).
fn surface_references_err(expr: &Expr) -> bool {
    if references_bare_err(expr) {
        return true;
    }
    let mut found = false;
    visit_string_literals(expr, &mut |literal| {
        if !found && literal_spans_reference_err(literal) {
            found = true;
        }
    });
    found
}

/// Whether any `{{ … }}` span inside a literal communication/action string
/// references the bare lifecycle `err` global.
fn literal_spans_reference_err(literal: &str) -> bool {
    ExpressionFinder::find_all_plain(literal).iter().any(|span| {
        parse(&span.expression)
            .map(|expr| references_bare_err(&expr))
            .unwrap_or(false)
    })
}

/// Returns `true` when the expression tree references the lifecycle `err`
/// global as a bare name or member-access base.
///
/// `doc.err` (and any `doc.*` path) is exempt: the `doc` namespace reaches
/// literal frontmatter, so `doc.err` is a property lookup, not a lifecycle
/// global reference.
fn references_bare_err(expr: &Expr) -> bool {
    match expr {
        Expr::Variable(path) => {
            let root = path.split('.').next().unwrap_or(path);
            root == "err"
        }
        Expr::MemberAccess { base, .. } => {
            // `doc.anything` (including `doc.err`) is not a bare err
            // reference — it reaches the frontmatter.
            if let Expr::Variable(base_path) = base.as_ref() {
                let root = base_path.split('.').next().unwrap_or(base_path);
                if root == "doc" {
                    return false;
                }
            }
            references_bare_err(base)
        }
        Expr::UnaryNot(inner) | Expr::UnaryMinus(inner) | Expr::Paren(inner) => {
            references_bare_err(inner)
        }
        Expr::Binary { left, right, .. } | Expr::Comparison { left, right, .. } => {
            references_bare_err(left) || references_bare_err(right)
        }
        Expr::Index { base, index } => references_bare_err(base) || references_bare_err(index),
        Expr::FunctionCall { args, .. } => args.iter().any(references_bare_err),
        Expr::Fallback { primary, fallback } => {
            references_bare_err(primary) || references_bare_err(fallback)
        }
        Expr::Ternary {
            condition,
            then_branch,
            else_branch,
        } => {
            references_bare_err(condition)
                || references_bare_err(then_branch)
                || references_bare_err(else_branch)
        }
        Expr::StringLiteral(_) | Expr::NumberLiteral(_) | Expr::BoolLiteral(_) => false,
    }
}

/// Collects the shell commands reachable from every lifecycle stack, for
/// inclusion in the pre-flight shell whitelist audit.
///
/// Returns `(command_expr, property_path)` pairs in deterministic order:
/// events in [`LifecycleSignal::ALL`] order, stack items in array order,
/// actions in execution order. The `command_expr` is the parsed expression
/// for the shell `command` field, which the caller renders to a string
/// (static literals are pre-known; expression-driven commands are also
/// gathered condition-blind, matching the existing template `::shell`
/// audit posture).
///
/// `on_error` commands are also collected because they execute on
/// non-zero exit. Each entry's property path names the source location
/// (e.g. `start.stack[1].action.command`).
pub fn collect_lifecycle_shell_commands(
    lifecycle: &LifecycleConfig,
) -> Vec<(String, String)> {
    let mut commands = Vec::new();
    for surface in iter_stack_expression_surfaces(lifecycle) {
        if let Some(literal) = expr_as_string_literal(surface.expr) {
            if surface.property.ends_with(".command") || surface.property.ends_with(".on_error") {
                commands.push((literal, surface.property));
            }
        }
    }
    commands
}

/// Render an [`Expr`] to its literal string value when it is a string
/// literal, a bare variable, or a number/bool literal. `None` otherwise
/// (complex expressions are not collected — they depend on runtime state
/// not visible at pre-flight time).
fn expr_as_string_literal(expr: &Expr) -> Option<String> {
    match expr {
        Expr::StringLiteral(s) => Some(s.clone()),
        Expr::Variable(v) => Some(v.clone()),
        Expr::NumberLiteral(n) => Some(n.to_string()),
        Expr::BoolLiteral(b) => Some(b.to_string()),
        _ => None,
    }
}

/// Normalizes empty or whitespace-only strings to `None`.
fn normalize_empty_string(field: &mut Option<String>) {
    if let Some(s) = field {
        if s.trim().is_empty() {
            *field = None;
        }
    }
}

/// Parse a `serde_json` "unknown field" error to extract the field name
/// and the list of expected fields.
///
/// Serde's message format is:
/// `unknown field `X`, expected one of `A`, `B`, `C``
///
/// Returns `(Some("X"), vec!["A", "B", "C"])` on match. For any serde error
/// that is **not** an unknown-field error (e.g. `invalid type: map, expected
/// a sequence` when `stack:` is authored as a map instead of a list) returns
/// `(None, vec![])` — the caller renders the raw serde message rather than a
/// fabricated "Unknown property" diagnostic, since the "expected" token in a
/// type-mismatch message ("expected a sequence") is unrelated to the field
/// catalog.
fn parse_serde_unknown_field(err: &serde_json::Error) -> (Option<String>, Vec<String>) {
    let msg = err.to_string();

    if !msg.contains("unknown field") {
        return (None, Vec::new());
    }

    // Extract unknown field name between first pair of backticks.
    let unknown_field = extract_backtick_value(&msg, 0);

    // Extract expected fields from "expected one of `A`, `B`, `C`"
    // or "expected `A`" (single field).
    let expected = if let Some(idx) = msg.find("expected") {
        collect_backtick_values(&msg[idx..])
    } else {
        LIFECYCLE_NOTIFICATION_FIELDS
            .iter()
            .map(|s| (*s).to_string())
            .collect()
    };

    (unknown_field, expected)
}

/// The canonical field names for [`LifecycleNotification`].
/// The canonical communication-field names for [`LifecycleNotification`].
///
/// Excludes `stack` because `stack` is a structured (list) field rather
/// than a string. Use [`LIFECYCLE_CONCERN_KEYS`] when the full set of
/// lifecycle concern keys (including `stack`) is needed.
pub(crate) const LIFECYCLE_NOTIFICATION_FIELDS: &[&str] = &[
    "say",
    "say_first",
    "effect",
    "message",
    "stderr",
    "notify",
    "info",
    "warn",
    "success",
    "stdout",
];

/// Extract the text inside the `n`th pair of backticks in `s`.
fn extract_backtick_value(s: &str, nth: usize) -> Option<String> {
    let mut start = 0;
    for i in 0..=nth {
        let open = s[start..].find('`')?;
        let abs_open = start + open;
        let close = s[abs_open + 1..].find('`')?;
        if i == nth {
            return Some(s[abs_open + 1..abs_open + 1 + close].to_string());
        }
        start = abs_open + 1 + close + 1;
    }
    None
}

/// Collect all backtick-delimited values from `s`.
fn collect_backtick_values(s: &str) -> Vec<String> {
    let mut vals = Vec::new();
    let mut chars = s.char_indices().peekable();
    while let Some(&(i, ch)) = chars.peek() {
        if ch == '`' {
            chars.next();
            let start = i + 1;
            let mut end = start;
            for (j, c) in chars.by_ref() {
                if c == '`' {
                    end = j;
                    break;
                }
            }
            if end > start {
                vals.push(s[start..end].to_string());
            }
        } else {
            chars.next();
        }
    }
    if vals.is_empty() {
        LIFECYCLE_NOTIFICATION_FIELDS
            .iter()
            .map(|s| (*s).to_string())
            .collect()
    } else {
        vals
    }
}

/// Build a `TtsConfig` from global settings.
pub(crate) fn tts_config_from_settings(tts: Option<&TtsSettings>) -> TtsConfig {
    let mut config = TtsConfig::new();
    let Some(settings) = tts else {
        return config;
    };

    if let Some(voice) = settings.voice.as_deref() {
        config = config.with_voice(voice);
    }
    if let Some(rate) = settings.rate {
        config = config.with_speed(SpeedLevel::Explicit(rate));
    }
    if let Some(provider) = settings.provider.as_deref() {
        if let Some(provider) = biscuit_speaks::parse_provider_name(provider) {
            config = config.with_failover(TtsFailoverStrategy::SpecificProvider(provider));
        } else {
            warn!(
                provider,
                "Unknown TTS provider in settings; using automatic selection"
            );
        }
    }
    config
}

/// Maximum wall-clock time a single lifecycle TTS playback may block the
/// composition thread before it is abandoned.
///
/// A wedged TTS provider (a stalled network voice, a contended audio device,
/// a hung `say` subprocess) must never freeze a compose run — the danger is
/// acute *between* loop iterations, where no child wait loop is installed and
/// the Ctrl+C interrupt flag cannot reach a synchronous call. Generous enough
/// for a long sentence, short enough that a hang can't wedge the run.
const TTS_PLAYBACK_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);

/// Maximum wall-clock time a single lifecycle sound effect may block.
const EFFECT_PLAYBACK_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(15);

/// Run a blocking side effect on a detached worker thread, bounding how long
/// the caller waits for it.
///
/// Returns once the work finishes or `timeout` elapses, whichever comes first.
/// On timeout the worker is detached (it may keep running harmlessly in the
/// background) and a warning is logged. This is the lifecycle analogue of the
/// wrapper's `join_with_timeout`: the only safe way to bound an arbitrary
/// blocking call (subprocess wait, audio device, network voice) from outside
/// is to stop waiting on it.
fn run_blocking_with_timeout<F>(label: &'static str, timeout: std::time::Duration, work: F)
where
    F: FnOnce() + Send + 'static,
{
    let (tx, rx) = std::sync::mpsc::channel::<()>();
    std::thread::spawn(move || {
        work();
        // A closed receiver (timed-out caller already moved on) is expected;
        // the send is best-effort.
        let _ = tx.send(());
    });
    match rx.recv_timeout(timeout) {
        Ok(()) => {}
        Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
            warn!(
                label,
                ?timeout,
                "lifecycle side effect exceeded its time budget; detaching and continuing"
            );
        }
        // The worker dropped its sender without signaling (e.g. it panicked).
        // There is nothing left to wait for, and no timeout was breached.
        Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {}
    }
}

/// Play a sound effect synchronously (blocking), bounded by
/// [`EFFECT_PLAYBACK_TIMEOUT`].
fn play_effect_blocking(name: &str) {
    let name = name.to_string();
    run_blocking_with_timeout("effect", EFFECT_PLAYBACK_TIMEOUT, move || {
        let Some(effect) = playa::SoundEffect::from_name(&name) else {
            warn!(%name, "Unknown sound effect in lifecycle notification");
            return;
        };
        match playa::Playa::from_bytes(effect.bytes().to_vec()) {
            Ok(player) => {
                if let Err(e) = player.play() {
                    warn!(%e, "Lifecycle sound effect playback failed");
                }
            }
            Err(e) => warn!(%e, "Failed to construct sound effect player"),
        }
    });
}

/// Speak text using the Tokio runtime, bounded by [`TTS_PLAYBACK_TIMEOUT`].
///
/// The playback runs on a detached worker thread that drives the async
/// `play()` future via the current runtime's `Handle` (cloned in before the
/// thread is spawned, since `Handle::block_on` works from any thread). This
/// both bounds the wait and avoids `block_in_place`, which would panic off a
/// runtime worker thread.
fn say_blocking(text: &str, config: TtsConfig) {
    let Ok(handle) = tokio::runtime::Handle::try_current() else {
        warn!("No Tokio runtime available for lifecycle TTS");
        return;
    };
    let text = text.to_string();
    run_blocking_with_timeout("tts", TTS_PLAYBACK_TIMEOUT, move || {
        handle.block_on(async move {
            if let Err(e) = biscuit_speaks::Speak::new(text)
                .with_config(config)
                .play()
                .await
            {
                warn!(%e, "Lifecycle TTS playback failed");
            }
        });
    });
}

/// Emit a lifecycle signal with deterministic audio ordering.
///
/// Dispatches non-audio targets (stderr, message) immediately, then
/// plays audio phases in order. All errors are logged as warnings and
/// never propagated.
///
/// Prefer [`LifecycleRunGuard`] for new code — it uses this same
/// emission logic but adds mechanical state-transition enforcement.
#[deprecated(
    since = "0.1.0",
    note = "use LifecycleRunGuard instead, which enforces state transitions and emits signals via the LifecycleEmitter trait"
)]
pub fn emit_lifecycle_signal(
    config: &LifecycleConfig,
    signal: LifecycleSignal,
    ctx: &LifecycleRuntimeContext<'_>,
) {
    let Some(notification) = config.get(signal) else {
        return;
    };

    // --- Non-audio fan-out (immediate) ---

    // stderr — plain prose, no status glyph (see DefaultLifecycleEmitter::emit_stderr)
    if let Some(stderr_text) = &notification.stderr {
        let rendered = Prose::new(stderr_text).render(ctx.term);
        eprintln!("{rendered}");
    }

    // message
    if let Some(message_text) = &notification.message {
        crate::messaging::execute_resolved_message(
            message_text,
            None,
            Some(ctx.source_path),
            ctx.repo_root,
            ctx.messaging,
        );
    }
    // notify
    if let Some(notify_title) = &notification.notify {
        crate::messaging::execute_notification(notify_title, None);
    }

    // --- Audio phases (sequential, blocking, lazy TTS config) ---

    let phases = audio_phases(notification);
    let mut tts_config: Option<TtsConfig> = None;

    for phase in phases {
        match phase {
            AudioPhase::Speak(text) => {
                let config = tts_config
                    .get_or_insert_with(|| tts_config_from_settings(ctx.settings.tts.as_ref()))
                    .clone();
                say_blocking(&text, config);
            }
            AudioPhase::Effect(name) => play_effect_blocking(&name),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn dummy_path() -> &'static Path {
        Path::new("test.md")
    }

    /// A blocking lifecycle side effect that wedges (never returns) must not
    /// be able to freeze the composition thread: `run_blocking_with_timeout`
    /// has to return after roughly its budget, not after the work finishes.
    /// This is the core of fix #1 — a hung TTS / sound provider between loop
    /// iterations used to lock the run with no way for Ctrl+C to break in.
    #[test]
    fn run_blocking_with_timeout_returns_when_work_hangs() {
        use std::time::{Duration, Instant};

        let start = Instant::now();
        run_blocking_with_timeout("test-hang", Duration::from_millis(100), || {
            // Simulate a wedged audio device / network voice.
            std::thread::sleep(Duration::from_secs(30));
        });
        let elapsed = start.elapsed();

        assert!(
            elapsed < Duration::from_secs(5),
            "must abandon the wedged side effect near the 100ms budget, \
             not wait out the 30s sleep; took {elapsed:?}"
        );
    }

    /// The happy path must still run the work to completion and return its
    /// result — bounding the wait must not turn into fire-and-forget for work
    /// that finishes within budget.
    #[test]
    fn run_blocking_with_timeout_runs_work_to_completion_within_budget() {
        use std::sync::Arc;
        use std::sync::atomic::{AtomicBool, Ordering};
        use std::time::Duration;

        let done = Arc::new(AtomicBool::new(false));
        let done_clone = Arc::clone(&done);
        run_blocking_with_timeout("test-quick", Duration::from_secs(5), move || {
            std::thread::sleep(Duration::from_millis(20));
            done_clone.store(true, Ordering::SeqCst);
        });

        assert!(
            done.load(Ordering::SeqCst),
            "work that finishes within budget must complete before the call returns"
        );
    }

    #[test]
    fn parses_valid_lifecycle_config() {
        let frontmatter = json!({
            "start": {
                "message": "Starting composition..."
            },
            "success": {
                "say": "All done!",
                "effect": "confirmation"
            }
        });

        let config = parse_lifecycle_config(&frontmatter, dummy_path()).unwrap();
        assert!(config.start.is_some());
        assert_eq!(
            config.start.as_ref().unwrap().message.as_deref(),
            Some("Starting composition...")
        );

        assert!(config.success.is_some());
        let success = config.success.as_ref().unwrap();
        assert_eq!(success.say.as_deref(), Some("All done!"));
        assert_eq!(success.effect.as_deref(), Some("confirmation"));
    }

    #[test]
    fn scan_rejects_pre_checks_removed_key() {
        let frontmatter = json!({
            "pre_checks": [{"command": "test"}],
            "start": { "message": "ok" }
        });
        let (key, replacement) = scan_removed_validation_keys(&frontmatter).unwrap();
        assert_eq!(key, "pre_checks");
        assert!(replacement.contains("initialize"), "replacement: {replacement}");
    }

    #[test]
    fn scan_rejects_post_checks_removed_key() {
        let frontmatter = json!({
            "post_checks": [{"command": "test"}],
            "success": { "message": "ok" }
        });
        let (key, replacement) = scan_removed_validation_keys(&frontmatter).unwrap();
        assert_eq!(key, "post_checks");
        assert!(replacement.contains("success"), "replacement: {replacement}");
    }

    #[test]
    fn scan_rejects_handle_removed_key() {
        let frontmatter = json!({
            "handle": "shell('fix')",
            "start": { "message": "ok" }
        });
        let (key, replacement) = scan_removed_validation_keys(&frontmatter).unwrap();
        assert_eq!(key, "handle");
        assert!(replacement.contains("shell"), "replacement: {replacement}");
    }

    #[test]
    fn scan_rejects_deviate_removed_key() {
        let frontmatter = json!({
            "deviate": "shell('fix')",
            "start": { "message": "ok" }
        });
        let (key, replacement) = scan_removed_validation_keys(&frontmatter).unwrap();
        assert_eq!(key, "deviate");
        assert!(replacement.contains("retry"), "replacement: {replacement}");
    }

    #[test]
    fn scan_rejects_handle_timeout_removed_key() {
        let frontmatter = json!({
            "handle_timeout": [{"action": "retry"}],
            "failure": { "message": "ok" }
        });
        let (key, replacement) = scan_removed_validation_keys(&frontmatter).unwrap();
        assert_eq!(key, "handle_timeout");
        assert!(replacement.contains("blocked"), "replacement: {replacement}");
    }

    #[test]
    fn scan_rejects_handle_inline_body_unchanged_removed_key() {
        let frontmatter = json!({
            "handle_inline_body_unchanged": [{"action": "retry"}],
            "failure": { "message": "ok" }
        });
        let (key, replacement) = scan_removed_validation_keys(&frontmatter).unwrap();
        assert_eq!(key, "handle_inline_body_unchanged");
        assert!(replacement.contains("failure"), "replacement: {replacement}");
    }

    #[test]
    fn scan_allows_handle_underscore_without_suffix() {
        // `handle_` with no suffix is not one of the removed keys; only exact
        // `handle` and `handle_<non-empty>` are rejected.
        let frontmatter = json!({
            "handle_": { "message": "ok" }
        });
        assert!(scan_removed_validation_keys(&frontmatter).is_none());
    }

    #[test]
    fn scan_returns_none_for_clean_frontmatter() {
        let frontmatter = json!({
            "start": { "message": "ok" }
        });
        assert!(scan_removed_validation_keys(&frontmatter).is_none());
    }

    #[test]
    fn rejects_both_say_and_say_first() {
        let frontmatter = json!({
            "start": {
                "say": "Starting",
                "say_first": "Also starting"
            }
        });

        let result = parse_lifecycle_config(&frontmatter, dummy_path());
        assert!(matches!(
            result,
            Err(CompositionError::LifecycleSayConflict(_))
        ));
    }

    #[test]
    fn trims_empty_strings_to_none() {
        let frontmatter = json!({
            "start": {
                "message": "   ",
                "say": ""
            }
        });

        let config = parse_lifecycle_config(&frontmatter, dummy_path()).unwrap();
        let start = config.start.as_ref().unwrap();
        assert!(start.message.is_none());
        assert!(start.say.is_none());
    }

    #[test]
    fn rejects_unknown_keys() {
        let frontmatter = json!({
            "start": {
                "message": "Starting",
                "unknown_field": "value"
            }
        });

        let result = parse_lifecycle_config(&frontmatter, dummy_path());
        assert!(result.is_err());
    }

    #[test]
    fn rejects_unknown_effect_name() {
        let frontmatter = json!({
            "start": {
                "effect": "nonexistent-effect"
            }
        });

        let result = parse_lifecycle_config(&frontmatter, dummy_path());
        assert!(matches!(
            result,
            Err(CompositionError::LifecycleUnknownEffect(_, _))
        ));
    }

    #[test]
    fn say_plus_effect_is_valid() {
        let frontmatter = json!({
            "success": {
                "say": "Done!",
                "effect": "confirmation"
            }
        });

        let config = parse_lifecycle_config(&frontmatter, dummy_path()).unwrap();
        let success = config.success.as_ref().unwrap();
        assert_eq!(success.say.as_deref(), Some("Done!"));
        assert_eq!(success.effect.as_deref(), Some("confirmation"));
    }

    #[test]
    fn say_first_plus_effect_is_valid() {
        let frontmatter = json!({
            "success": {
                "say_first": "Starting now",
                "effect": "confirmation"
            }
        });

        let config = parse_lifecycle_config(&frontmatter, dummy_path()).unwrap();
        let success = config.success.as_ref().unwrap();
        assert_eq!(success.say_first.as_deref(), Some("Starting now"));
        assert_eq!(success.effect.as_deref(), Some("confirmation"));
    }

    #[test]
    fn empty_frontmatter_returns_default() {
        let frontmatter = json!({});
        let config = parse_lifecycle_config(&frontmatter, dummy_path()).unwrap();
        assert!(config.is_empty());
    }

    #[test]
    fn non_object_frontmatter_returns_default() {
        let frontmatter = json!("not an object");
        let config = parse_lifecycle_config(&frontmatter, dummy_path()).unwrap();
        assert!(config.is_empty());
    }

    #[test]
    fn null_lifecycle_property_is_skipped() {
        let frontmatter = json!({
            "start": null,
            "success": {
                "message": "Done"
            }
        });

        let config = parse_lifecycle_config(&frontmatter, dummy_path()).unwrap();
        assert!(config.start.is_none());
        assert!(config.success.is_some());
    }

    #[test]
    fn frontmatter_with_non_lifecycle_keys_is_fine() {
        let frontmatter = json!({
            "title": "My Composition",
            "agent": "claude",
            "start": {
                "message": "Starting"
            }
        });

        let config = parse_lifecycle_config(&frontmatter, dummy_path()).unwrap();
        assert!(config.start.is_some());
    }

    #[test]
    fn audio_order_say_plus_effect() {
        let n = LifecycleNotification {
            say: Some("Hello".into()),
            effect: Some("doorbell".into()),
            ..Default::default()
        };
        let phases = audio_phases(&n);
        assert_eq!(phases.len(), 2);
        assert!(matches!(phases[0], AudioPhase::Effect(_)));
        assert!(matches!(phases[1], AudioPhase::Speak(_)));
    }

    #[test]
    fn audio_order_say_first_plus_effect() {
        let n = LifecycleNotification {
            say_first: Some("Hello".into()),
            effect: Some("doorbell".into()),
            ..Default::default()
        };
        let phases = audio_phases(&n);
        assert_eq!(phases.len(), 2);
        assert!(matches!(phases[0], AudioPhase::Speak(_)));
        assert!(matches!(phases[1], AudioPhase::Effect(_)));
    }

    #[test]
    fn audio_order_speech_only() {
        let n = LifecycleNotification {
            say: Some("Hello".into()),
            ..Default::default()
        };
        let phases = audio_phases(&n);
        assert_eq!(phases.len(), 1);
        assert!(matches!(phases[0], AudioPhase::Speak(_)));
    }

    #[test]
    fn audio_order_effect_only() {
        let n = LifecycleNotification {
            effect: Some("doorbell".into()),
            ..Default::default()
        };
        let phases = audio_phases(&n);
        assert_eq!(phases.len(), 1);
        assert!(matches!(phases[0], AudioPhase::Effect(_)));
    }

    #[test]
    fn audio_order_no_audio() {
        let n = LifecycleNotification {
            stderr: Some("Status only".into()),
            ..Default::default()
        };
        let phases = audio_phases(&n);
        assert!(phases.is_empty());
    }

    #[test]
    fn status_state_mapping() {
        assert_eq!(LifecycleSignal::Start.status_state(), StatusState::Info);
        assert_eq!(
            LifecycleSignal::Success.status_state(),
            StatusState::Success
        );
        assert_eq!(LifecycleSignal::Blocked.status_state(), StatusState::Error);
        assert_eq!(LifecycleSignal::Failure.status_state(), StatusState::Error);
    }

    #[test]
    fn property_names() {
        assert_eq!(LifecycleSignal::Start.property_name(), "start");
        assert_eq!(LifecycleSignal::Success.property_name(), "success");
        assert_eq!(LifecycleSignal::Blocked.property_name(), "blocked");
        assert_eq!(LifecycleSignal::Failure.property_name(), "failure");
    }

    #[test]
    fn lifecycle_config_get() {
        let fm = json!({
            "start": { "stderr": "Starting" },
            "failure": { "stderr": "Failed" }
        });
        let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
        assert!(config.get(LifecycleSignal::Start).is_some());
        assert!(config.get(LifecycleSignal::Success).is_none());
        assert!(config.get(LifecycleSignal::Blocked).is_none());
        assert!(config.get(LifecycleSignal::Failure).is_some());
    }

    #[test]
    fn lifecycle_config_is_empty() {
        let empty = LifecycleConfig::default();
        assert!(empty.is_empty());

        let fm = json!({ "start": { "stderr": "Go" } });
        let non_empty = parse_lifecycle_config(&fm, dummy_path()).unwrap();
        assert!(!non_empty.is_empty());
    }

    #[test]
    #[allow(deprecated)]
    fn lifecycle_runtime_state_defaults() {
        let state = LifecycleRuntimeState::default();
        assert!(!state.start_emitted);
        assert!(!state.provider_launch_started);
    }

    // -- RecordingEmitter + LifecycleRunGuard tests -------------------------

    use std::sync::Mutex;

    #[derive(Debug, Clone, PartialEq)]
    enum EmittedAction {
        Stderr {
            signal: LifecycleSignal,
            text: String,
        },
        Message {
            text: String,
        },
        Notification {
            title: String,
        },
        Speech {
            text: String,
        },
        Effect {
            name: String,
        },
    }

    struct RecordingEmitter {
        actions: Mutex<Vec<EmittedAction>>,
    }

    impl RecordingEmitter {
        fn new() -> Self {
            Self {
                actions: Mutex::new(Vec::new()),
            }
        }

        fn actions(&self) -> Vec<EmittedAction> {
            self.actions.lock().unwrap().clone()
        }

        fn signals(&self) -> Vec<LifecycleSignal> {
            self.actions
                .lock()
                .unwrap()
                .iter()
                .filter_map(|a| match a {
                    EmittedAction::Stderr { signal, .. } => Some(*signal),
                    _ => None,
                })
                .collect()
        }
    }

    impl LifecycleEmitter for RecordingEmitter {
        fn emit_stderr(&self, signal: LifecycleSignal, text: &str, _term: &Terminal) {
            self.actions.lock().unwrap().push(EmittedAction::Stderr {
                signal,
                text: text.to_string(),
            });
        }

        fn emit_message(
            &self,
            text: &str,
            _source_path: &Path,
            _repo_root: Option<&Path>,
            _messaging: &RuntimeMessagingSettings,
        ) {
            self.actions.lock().unwrap().push(EmittedAction::Message {
                text: text.to_string(),
            });
        }

        fn emit_speech(&self, text: &str, _tts_config: TtsConfig) {
            self.actions.lock().unwrap().push(EmittedAction::Speech {
                text: text.to_string(),
            });
        }

        fn emit_effect(&self, name: &str) {
            self.actions.lock().unwrap().push(EmittedAction::Effect {
                name: name.to_string(),
            });
        }

        fn emit_notification(&self, title: &str) {
            self.actions
                .lock()
                .unwrap()
                .push(EmittedAction::Notification {
                    title: title.to_string(),
                });
        }
    }

    fn test_config() -> LifecycleConfig {
        parse_lifecycle_config(
            &json!({
                "start":   { "stderr": "starting" },
                "success": { "stderr": "done" },
                "blocked": { "stderr": "blocked" },
                "failure": { "stderr": "failed" },
            }),
            dummy_path(),
        )
        .unwrap()
    }

    fn test_ctx() -> (GlobalSettings, RuntimeMessagingSettings, Terminal) {
        (
            GlobalSettings::default(),
            RuntimeMessagingSettings {
                user: None,
                repo: None,
            },
            Terminal::default(),
        )
    }

    fn make_guard<'a>(
        config: &'a LifecycleConfig,
        ctx: &'a LifecycleRuntimeContext<'a>,
        emitter: &'a RecordingEmitter,
    ) -> LifecycleRunGuard<'a> {
        LifecycleRunGuard::new(config, ctx, emitter)
    }

    #[test]
    fn guard_emits_start_once() {
        let config = test_config();
        let (settings, messaging, term) = test_ctx();
        let ctx = LifecycleRuntimeContext {
            settings: &settings,
            messaging: &messaging,
            term: &term,
            source_path: Path::new("/tmp/test.md"),
            repo_root: None,
            launch_area: None,
        };
        let emitter = RecordingEmitter::new();
        let mut guard = make_guard(&config, &ctx, &emitter);

        guard.emit_start_once();
        guard.emit_start_once(); // second call is idempotent
        guard.defuse();

        assert_eq!(emitter.signals(), vec![LifecycleSignal::Start]);
    }

    #[test]
    fn guard_drop_emits_blocked_before_launch() {
        let config = test_config();
        let (settings, messaging, term) = test_ctx();
        let ctx = LifecycleRuntimeContext {
            settings: &settings,
            messaging: &messaging,
            term: &term,
            source_path: Path::new("/tmp/test.md"),
            repo_root: None,
            launch_area: None,
        };
        let emitter = RecordingEmitter::new();

        {
            let mut guard = make_guard(&config, &ctx, &emitter);
            guard.emit_start_once();
            // drop without terminal signal, not launched
        }

        assert_eq!(
            emitter.signals(),
            vec![LifecycleSignal::Start, LifecycleSignal::Blocked]
        );
    }

    #[test]
    fn guard_drop_emits_failure_after_launch() {
        let config = test_config();
        let (settings, messaging, term) = test_ctx();
        let ctx = LifecycleRuntimeContext {
            settings: &settings,
            messaging: &messaging,
            term: &term,
            source_path: Path::new("/tmp/test.md"),
            repo_root: None,
            launch_area: None,
        };
        let emitter = RecordingEmitter::new();

        {
            let mut guard = make_guard(&config, &ctx, &emitter);
            guard.emit_start_once();
            guard.mark_provider_launched();
            // drop without terminal signal, but launched
        }

        assert_eq!(
            emitter.signals(),
            vec![LifecycleSignal::Start, LifecycleSignal::Failure]
        );
    }

    #[test]
    fn guard_drop_silent_without_start() {
        let config = test_config();
        let (settings, messaging, term) = test_ctx();
        let ctx = LifecycleRuntimeContext {
            settings: &settings,
            messaging: &messaging,
            term: &term,
            source_path: Path::new("/tmp/test.md"),
            repo_root: None,
            launch_area: None,
        };
        let emitter = RecordingEmitter::new();

        {
            let _guard = make_guard(&config, &ctx, &emitter);
            // drop without ever emitting start
        }

        assert!(emitter.signals().is_empty());
    }

    #[test]
    fn guard_emit_terminal_prevents_drop_emission() {
        let config = test_config();
        let (settings, messaging, term) = test_ctx();
        let ctx = LifecycleRuntimeContext {
            settings: &settings,
            messaging: &messaging,
            term: &term,
            source_path: Path::new("/tmp/test.md"),
            repo_root: None,
            launch_area: None,
        };
        let emitter = RecordingEmitter::new();

        {
            let mut guard = make_guard(&config, &ctx, &emitter);
            guard.emit_start_once();
            guard.mark_provider_launched();
            guard.emit_terminal(LifecycleSignal::Success);
            // drop after explicit terminal — no double emission
        }

        assert_eq!(
            emitter.signals(),
            vec![LifecycleSignal::Start, LifecycleSignal::Success]
        );
    }

    #[test]
    fn guard_defuse_prevents_drop_emission() {
        let config = test_config();
        let (settings, messaging, term) = test_ctx();
        let ctx = LifecycleRuntimeContext {
            settings: &settings,
            messaging: &messaging,
            term: &term,
            source_path: Path::new("/tmp/test.md"),
            repo_root: None,
            launch_area: None,
        };
        let emitter = RecordingEmitter::new();

        {
            let mut guard = make_guard(&config, &ctx, &emitter);
            guard.emit_start_once();
            guard.defuse();
        }

        // Only start, no terminal from Drop
        assert_eq!(emitter.signals(), vec![LifecycleSignal::Start]);
    }

    #[test]
    fn guard_emit_blocked_or_failure_pre_launch() {
        let config = test_config();
        let (settings, messaging, term) = test_ctx();
        let ctx = LifecycleRuntimeContext {
            settings: &settings,
            messaging: &messaging,
            term: &term,
            source_path: Path::new("/tmp/test.md"),
            repo_root: None,
            launch_area: None,
        };
        let emitter = RecordingEmitter::new();

        {
            let mut guard = make_guard(&config, &ctx, &emitter);
            guard.emit_start_once();
            guard.emit_blocked_or_failure(); // pre-launch → Blocked
        }

        assert_eq!(
            emitter.signals(),
            vec![LifecycleSignal::Start, LifecycleSignal::Blocked]
        );
    }

    #[test]
    fn guard_emit_blocked_or_failure_post_launch() {
        let config = test_config();
        let (settings, messaging, term) = test_ctx();
        let ctx = LifecycleRuntimeContext {
            settings: &settings,
            messaging: &messaging,
            term: &term,
            source_path: Path::new("/tmp/test.md"),
            repo_root: None,
            launch_area: None,
        };
        let emitter = RecordingEmitter::new();

        {
            let mut guard = make_guard(&config, &ctx, &emitter);
            guard.emit_start_once();
            guard.mark_provider_launched();
            guard.emit_blocked_or_failure(); // post-launch → Failure
        }

        assert_eq!(
            emitter.signals(),
            vec![LifecycleSignal::Start, LifecycleSignal::Failure]
        );
    }

    #[test]
    fn guard_state_accessors() {
        let config = test_config();
        let (settings, messaging, term) = test_ctx();
        let ctx = LifecycleRuntimeContext {
            settings: &settings,
            messaging: &messaging,
            term: &term,
            source_path: Path::new("/tmp/test.md"),
            repo_root: None,
            launch_area: None,
        };
        let emitter = RecordingEmitter::new();
        let mut guard = make_guard(&config, &ctx, &emitter);

        assert!(!guard.start_emitted());
        assert!(!guard.provider_launched());

        guard.emit_start_once();
        assert!(guard.start_emitted());
        assert!(!guard.provider_launched());

        guard.mark_provider_launched();
        assert!(guard.provider_launched());

        guard.defuse();
    }

    #[test]
    fn validation_is_the_dispatch_gate_for_leaked_lifecycle() {
        // The `LifecycleRunGuard` does not re-validate; it dispatches whatever
        // string the config holds. The contract "no side effect dispatches a
        // leaked expression" is upheld by `validate_no_interpolation_leaks`
        // running in the prepare layer, *before* a guard is ever built. This
        // test proves both halves of that boundary against the fake emitter.
        let leaked = parse_lifecycle_config(
            &json!({ "start": { "message": "{{ broken( }}" } }),
            dummy_path(),
        )
        .unwrap();

        // 1. Validation rejects the leaked config — the production choke point.
        let err = validate_no_interpolation_leaks(&leaked, dummy_path(), &[]).unwrap_err();
        assert!(matches!(
            err,
            CompositionError::LifecycleInterpolationLeak { .. }
        ));

        // 2. A guard built from that same config WOULD dispatch the raw span
        //    (the message reaches the emitter verbatim), confirming the guard
        //    itself is not the gate — only the prepare-layer validation is.
        let (settings, messaging, term) = test_ctx();
        let ctx = LifecycleRuntimeContext {
            settings: &settings,
            messaging: &messaging,
            term: &term,
            source_path: Path::new("/tmp/test.md"),
            repo_root: None,
            launch_area: None,
        };
        let emitter = RecordingEmitter::new();
        {
            let mut guard = make_guard(&leaked, &ctx, &emitter);
            guard.emit_start_once();
            guard.defuse();
        }
        assert!(
            emitter.actions().iter().any(|a| matches!(
                a,
                EmittedAction::Message { text } if text.contains("{{ broken(")
            )),
            "guard does not self-gate; validation must run before a guard exists"
        );
    }

    fn fm_from_json(value: serde_json::Value) -> darkmatter::markdown::Frontmatter {
        let mut fm = darkmatter::markdown::Frontmatter::new();
        if let serde_json::Value::Object(map) = value {
            for (key, val) in map {
                fm.insert(&key, val).unwrap();
            }
        }
        fm
    }

    #[test]
    fn undefined_bare_variable_flags_missing_root() {
        let effective = json!({ "area": "claudine" });
        let defined = effective.as_object();
        assert_eq!(undefined_bare_variable("missing", defined), Some("missing"));
        assert_eq!(undefined_bare_variable("area", defined), None);
        // Nested miss under a defined root is treated as defined.
        assert_eq!(undefined_bare_variable("area.sub", defined), None);
        // Runtime namespaces resolve outside the frontmatter.
        assert_eq!(undefined_bare_variable("ctx.area", defined), None);
        assert_eq!(undefined_bare_variable("env.HOME", defined), None);
        assert_eq!(undefined_bare_variable("doc", defined), None);
        assert_eq!(undefined_bare_variable("doc.area", defined), None);
    }

    #[test]
    fn undefined_lifecycle_variable_is_rejected() {
        let raw = fm_from_json(json!({
            "start": { "message": "before {{ missing_lifecycle_var }} after" }
        }));
        let effective = json!({ "start": { "message": "before  after" } });

        let err =
            validate_no_undefined_lifecycle_variables(&raw, &effective, &LifecycleConfig::default(), dummy_path()).unwrap_err();
        match err {
            CompositionError::LifecycleUndefinedVariable {
                property, variable, ..
            } => {
                assert_eq!(property, "start.message");
                assert_eq!(variable, "missing_lifecycle_var");
            }
            other => panic!("expected LifecycleUndefinedVariable, got: {other:?}"),
        }
    }

    #[test]
    fn defined_and_namespaced_lifecycle_variables_pass() {
        let raw = fm_from_json(json!({
            "start": { "message": "{{ area }} on {{ ctx.today }}" },
            "success": { "say": "{{ missing || 'fallback' }}" },
        }));
        let effective = json!({ "area": "claudine" });

        assert!(validate_no_undefined_lifecycle_variables(&raw, &effective, &LifecycleConfig::default(), dummy_path()).is_ok());
    }

    #[test]
    fn undefined_variable_inside_function_call_is_rejected() {
        // The original broken prompt used `parent_dir(review)`: a bare undefined
        // variable as a function argument must fail preparation, not collapse to
        // an empty string the way the whole-span-only guard let it.
        let raw = fm_from_json(json!({
            "start": { "message": "before {{ parent_dir(missing_review) }} after" }
        }));
        let effective = json!({ "area": "claudine" });

        let err =
            validate_no_undefined_lifecycle_variables(&raw, &effective, &LifecycleConfig::default(), dummy_path()).unwrap_err();
        match err {
            CompositionError::LifecycleUndefinedVariable {
                property, variable, ..
            } => {
                assert_eq!(property, "start.message");
                assert_eq!(variable, "missing_review");
            }
            other => panic!("expected LifecycleUndefinedVariable, got: {other:?}"),
        }
    }

    #[test]
    fn undefined_variable_inside_fallback_argument_passes() {
        // Fallback semantics tolerate the undefined operand even when it is
        // wrapped in a function call, so the whole subtree is skipped.
        let raw = fm_from_json(json!({
            "start": { "message": "{{ parent_dir(missing) || 'home' }}" }
        }));
        let effective = json!({ "area": "claudine" });

        assert!(validate_no_undefined_lifecycle_variables(&raw, &effective, &LifecycleConfig::default(), dummy_path()).is_ok());
    }

    #[test]
    fn undefined_variable_in_ternary_condition_is_rejected() {
        let raw = fm_from_json(json!({
            "start": { "message": "{{ missing == 'x' ? 'a' : 'b' }}" }
        }));
        let effective = json!({});

        let err =
            validate_no_undefined_lifecycle_variables(&raw, &effective, &LifecycleConfig::default(), dummy_path()).unwrap_err();
        match err {
            CompositionError::LifecycleUndefinedVariable {
                property, variable, ..
            } => {
                assert_eq!(property, "start.message");
                assert_eq!(variable, "missing");
            }
            other => panic!("expected LifecycleUndefinedVariable, got: {other:?}"),
        }
    }

    #[test]
    fn undefined_variable_in_ternary_truthy_condition_is_rejected() {
        let raw = fm_from_json(json!({
            "start": { "message": "{{ missing ? 'a' : 'b' }}" }
        }));
        let effective = json!({});

        let err =
            validate_no_undefined_lifecycle_variables(&raw, &effective, &LifecycleConfig::default(), dummy_path()).unwrap_err();
        match err {
            CompositionError::LifecycleUndefinedVariable {
                property, variable, ..
            } => {
                assert_eq!(property, "start.message");
                assert_eq!(variable, "missing");
            }
            other => panic!("expected LifecycleUndefinedVariable, got: {other:?}"),
        }
    }

    #[test]
    fn defined_condition_with_undefined_branch_operands_passes() {
        // Ternary branches intentionally tolerate undefined operands; only the
        // condition is checked.
        let raw = fm_from_json(json!({
            "start": { "message": "{{ defined ? missing : also_missing }}" }
        }));
        let effective = json!({ "defined": true });

        assert!(validate_no_undefined_lifecycle_variables(&raw, &effective, &LifecycleConfig::default(), dummy_path()).is_ok());
    }

    #[test]
    fn undefined_variable_in_index_is_rejected() {
        let raw = fm_from_json(json!({
            "start": { "message": "{{ missing[0] }}" }
        }));
        let effective = json!({});

        let err =
            validate_no_undefined_lifecycle_variables(&raw, &effective, &LifecycleConfig::default(), dummy_path()).unwrap_err();
        match err {
            CompositionError::LifecycleUndefinedVariable { variable, .. } => {
                assert_eq!(variable, "missing");
            }
            other => panic!("expected LifecycleUndefinedVariable, got: {other:?}"),
        }
    }

    #[test]
    fn undefined_variable_in_member_access_is_rejected() {
        let raw = fm_from_json(json!({
            "start": { "message": "{{ missing.foo }}" }
        }));
        let effective = json!({});

        let err =
            validate_no_undefined_lifecycle_variables(&raw, &effective, &LifecycleConfig::default(), dummy_path()).unwrap_err();
        match err {
            CompositionError::LifecycleUndefinedVariable { variable, .. } => {
                assert_eq!(variable, "missing");
            }
            other => panic!("expected LifecycleUndefinedVariable, got: {other:?}"),
        }
    }

    #[test]
    fn defined_variable_inside_function_call_passes() {
        let raw = fm_from_json(json!({
            "start": { "message": "{{ parent_dir(area) }}" }
        }));
        let effective = json!({ "area": "/repo/claudine" });

        assert!(validate_no_undefined_lifecycle_variables(&raw, &effective, &LifecycleConfig::default(), dummy_path()).is_ok());
    }

    #[test]
    fn guard_non_audio_before_audio() {
        let config = parse_lifecycle_config(
            &json!({
                "start": {
                    "stderr": "starting",
                    "message": "msg",
                    "notify": "notify-msg",
                    "say": "hello",
                    "effect": "confirmation",
                }
            }),
            dummy_path(),
        )
        .unwrap();
        let (settings, messaging, term) = test_ctx();
        let ctx = LifecycleRuntimeContext {
            settings: &settings,
            messaging: &messaging,
            term: &term,
            source_path: Path::new("/tmp/test.md"),
            repo_root: None,
            launch_area: None,
        };
        let emitter = RecordingEmitter::new();
        let mut guard = make_guard(&config, &ctx, &emitter);
        guard.emit_start_once();
        guard.defuse();

        let actions = emitter.actions();
        assert_eq!(actions.len(), 5);
        // Non-audio first
        assert!(matches!(actions[0], EmittedAction::Stderr { .. }));
        assert!(matches!(actions[1], EmittedAction::Message { .. }));
        assert!(matches!(actions[2], EmittedAction::Notification { .. }));
        // Audio: effect before say (default order)
        assert!(matches!(actions[3], EmittedAction::Effect { .. }));
        assert!(matches!(actions[4], EmittedAction::Speech { .. }));
    }

    #[test]
    fn guard_say_first_ordering() {
        let config = parse_lifecycle_config(
            &json!({
                "start": {
                    "say_first": "hello",
                    "effect": "confirmation",
                }
            }),
            dummy_path(),
        )
        .unwrap();
        let (settings, messaging, term) = test_ctx();
        let ctx = LifecycleRuntimeContext {
            settings: &settings,
            messaging: &messaging,
            term: &term,
            source_path: Path::new("/tmp/test.md"),
            repo_root: None,
            launch_area: None,
        };
        let emitter = RecordingEmitter::new();
        let mut guard = make_guard(&config, &ctx, &emitter);
        guard.emit_start_once();
        guard.defuse();

        let actions = emitter.actions();
        assert_eq!(actions.len(), 2);
        // say_first → speech before effect
        assert!(matches!(actions[0], EmittedAction::Speech { .. }));
        assert!(matches!(actions[1], EmittedAction::Effect { .. }));
    }

    #[test]
    #[serial_test::serial]
    fn emit_signal_skips_blocking_side_effects_when_interrupted() {
        // Bug fix (2026-05-09): a Ctrl+C during a long compose run must
        // skip messenger sends, desktop notifications, TTS, and sound
        // effects so the process exits promptly. Only the cheap stderr
        // line is allowed to render so the user sees the terminal status.
        let config = parse_lifecycle_config(
            &json!({
                "failure": {
                    "stderr": "failed",
                    "message": "Compose run failed",
                    "notify": "Compose failed",
                    "say": "compose failed",
                    "effect": "confirmation",
                }
            }),
            dummy_path(),
        )
        .unwrap();
        let (settings, messaging, term) = test_ctx();
        let ctx = LifecycleRuntimeContext {
            settings: &settings,
            messaging: &messaging,
            term: &term,
            source_path: Path::new("/tmp/test.md"),
            repo_root: None,
            launch_area: None,
        };
        let emitter = RecordingEmitter::new();
        let mut guard = make_guard(&config, &ctx, &emitter);

        crate::interrupt::clear_for_tests();
        crate::interrupt::mark_interrupted();
        guard.emit_terminal(LifecycleSignal::Failure);
        crate::interrupt::clear_for_tests();

        let actions = emitter.actions();
        assert_eq!(
            actions.len(),
            1,
            "interrupt must drop messenger/notification/TTS/effect; got: {actions:?}"
        );
        assert!(
            matches!(actions[0], EmittedAction::Stderr { .. }),
            "stderr line must still render so the user sees the terminal status"
        );
    }

    #[test]
    #[serial_test::serial]
    fn emit_signal_runs_all_side_effects_when_not_interrupted() {
        // Companion to the interrupt test: when no interrupt is observed,
        // every configured side effect still fires.
        let config = parse_lifecycle_config(
            &json!({
                "failure": {
                    "stderr": "failed",
                    "message": "Compose run failed",
                    "notify": "Compose failed",
                }
            }),
            dummy_path(),
        )
        .unwrap();
        let (settings, messaging, term) = test_ctx();
        let ctx = LifecycleRuntimeContext {
            settings: &settings,
            messaging: &messaging,
            term: &term,
            source_path: Path::new("/tmp/test.md"),
            repo_root: None,
            launch_area: None,
        };
        let emitter = RecordingEmitter::new();
        let mut guard = make_guard(&config, &ctx, &emitter);

        crate::interrupt::clear_for_tests();
        guard.emit_terminal(LifecycleSignal::Failure);

        let actions = emitter.actions();
        assert!(
            actions
                .iter()
                .any(|a| matches!(a, EmittedAction::Stderr { .. }))
        );
        assert!(
            actions
                .iter()
                .any(|a| matches!(a, EmittedAction::Message { .. }))
        );
        assert!(
            actions
                .iter()
                .any(|a| matches!(a, EmittedAction::Notification { .. }))
        );
    }

    // =====================================================================
    // notify parsing and emission (Phase 3)
    // =====================================================================

    #[test]
    fn parses_notify_for_all_signals() {
        let fm = json!({
            "start": { "notify": "Starting" },
            "success": { "notify": "Done" },
            "blocked": { "notify": "Blocked" },
            "failure": { "notify": "Failed" }
        });
        let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();

        assert_eq!(
            config.start.as_ref().unwrap().notify.as_deref(),
            Some("Starting")
        );
        assert_eq!(
            config.success.as_ref().unwrap().notify.as_deref(),
            Some("Done")
        );
        assert_eq!(
            config.blocked.as_ref().unwrap().notify.as_deref(),
            Some("Blocked")
        );
        assert_eq!(
            config.failure.as_ref().unwrap().notify.as_deref(),
            Some("Failed")
        );
    }

    #[test]
    fn parses_message_and_notify_independently() {
        let fm = json!({
            "start": {
                "message": "Remote message",
                "notify": "Local notification"
            }
        });
        let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
        let start = config.start.as_ref().unwrap();
        assert_eq!(start.message.as_deref(), Some("Remote message"));
        assert_eq!(start.notify.as_deref(), Some("Local notification"));
    }

    #[test]
    fn blank_notify_is_normalized_to_none() {
        let fm = json!({
            "start": { "notify": "   " },
            "success": { "notify": "" }
        });
        let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
        assert!(config.start.as_ref().unwrap().notify.is_none());
        assert!(config.success.as_ref().unwrap().notify.is_none());
    }

    #[test]
    fn notify_emits_without_active_route() {
        let config = parse_lifecycle_config(
            &json!({
                "start": { "notify": "Hello desktop" }
            }),
            dummy_path(),
        )
        .unwrap();
        let (settings, messaging, term) = test_ctx();
        let ctx = LifecycleRuntimeContext {
            settings: &settings,
            messaging: &messaging,
            term: &term,
            source_path: Path::new("/tmp/test.md"),
            repo_root: None,
            launch_area: None,
        };
        let emitter = RecordingEmitter::new();
        let mut guard = make_guard(&config, &ctx, &emitter);
        guard.emit_start_once();
        guard.defuse();

        let actions = emitter.actions();
        assert_eq!(actions.len(), 1);
        assert_eq!(
            actions[0],
            EmittedAction::Notification {
                title: "Hello desktop".to_string()
            }
        );
    }

    #[test]
    fn notify_emits_before_audio_phases() {
        let config = parse_lifecycle_config(
            &json!({
                "start": {
                    "notify": "Desktop first",
                    "say": "hello",
                    "effect": "confirmation",
                }
            }),
            dummy_path(),
        )
        .unwrap();
        let (settings, messaging, term) = test_ctx();
        let ctx = LifecycleRuntimeContext {
            settings: &settings,
            messaging: &messaging,
            term: &term,
            source_path: Path::new("/tmp/test.md"),
            repo_root: None,
            launch_area: None,
        };
        let emitter = RecordingEmitter::new();
        let mut guard = make_guard(&config, &ctx, &emitter);
        guard.emit_start_once();
        guard.defuse();

        let actions = emitter.actions();
        assert_eq!(actions.len(), 3);
        assert!(matches!(actions[0], EmittedAction::Notification { .. }));
        assert!(matches!(actions[1], EmittedAction::Effect { .. }));
        assert!(matches!(actions[2], EmittedAction::Speech { .. }));
    }

    #[test]
    fn notify_alone_no_other_outputs() {
        let config = parse_lifecycle_config(
            &json!({
                "success": { "notify": "Only notify" }
            }),
            dummy_path(),
        )
        .unwrap();
        let (settings, messaging, term) = test_ctx();
        let ctx = LifecycleRuntimeContext {
            settings: &settings,
            messaging: &messaging,
            term: &term,
            source_path: Path::new("/tmp/test.md"),
            repo_root: None,
            launch_area: None,
        };
        let emitter = RecordingEmitter::new();
        let mut guard = make_guard(&config, &ctx, &emitter);
        guard.emit_terminal(LifecycleSignal::Success);

        let actions = emitter.actions();
        assert_eq!(actions.len(), 1);
        assert_eq!(
            actions[0],
            EmittedAction::Notification {
                title: "Only notify".to_string()
            }
        );
    }

    #[tokio::test]
    async fn default_lifecycle_emitter_emit_notification_does_not_panic() {
        let emitter = DefaultLifecycleEmitter;
        // Fire-and-forget through the title-only trait method.
        emitter.emit_notification("unit testing");
        // And exercise the body-bearing path directly so the rendered
        // notification has a distinct title and message line.
        crate::messaging::execute_notification(
            "unit testing",
            Some("you can dismiss this notification"),
        );
        // Give the spawned tasks a moment to start
        tokio::task::yield_now().await;
    }

    #[test]
    fn lifecycle_invalid_error_renders_as_block_error() {
        use biscuit_terminal::errors::BlockError;
        use biscuit_terminal::utils::escape_codes::strip_escape_codes;

        let frontmatter = json!({
            "success": {
                "speak": "hello"
            }
        });

        let err =
            parse_lifecycle_config(&frontmatter, Path::new("prompts/sentrux.md")).unwrap_err();
        let CompositionError::LifecycleInvalid {
            property,
            unknown_field,
            expected_fields,
            source_file,
            ..
        } = &err
        else {
            panic!("expected LifecycleInvalid, got {err:?}");
        };

        assert_eq!(property, "success");
        assert_eq!(unknown_field.as_deref(), Some("speak"));
        assert_eq!(source_file, Path::new("prompts/sentrux.md"));
        assert!(expected_fields.contains(&"say".to_string()));
        assert!(expected_fields.contains(&"say_first".to_string()));
        assert!(expected_fields.contains(&"effect".to_string()));

        let rendered = strip_escape_codes(err.report_block_error_optimistic(Some(80)));
        assert!(
            rendered.contains("success.speak"),
            "dotted property should appear: {rendered}"
        );
        assert!(
            rendered.contains("sentrux.md"),
            "file name should appear: {rendered}"
        );
        assert!(
            rendered.contains("say"),
            "expected fields should list 'say': {rendered}"
        );
    }

    #[test]
    fn parse_serde_unknown_field_extracts_field_and_expected() {
        let frontmatter = json!({
            "failure": {
                "bogus_field": true
            }
        });

        let err = parse_lifecycle_config(&frontmatter, dummy_path()).unwrap_err();
        let CompositionError::LifecycleInvalid {
            property,
            unknown_field,
            expected_fields,
            ..
        } = &err
        else {
            panic!("expected LifecycleInvalid, got {err:?}");
        };

        assert_eq!(property, "failure");
        assert_eq!(unknown_field.as_deref(), Some("bogus_field"));
        assert!(!expected_fields.is_empty());
        assert!(expected_fields.contains(&"say".to_string()));
    }

    #[test]
    fn stack_as_map_reports_sequence_mismatch_not_unknown_property() {
        use biscuit_terminal::errors::BlockError;
        use biscuit_terminal::utils::escape_codes::strip_escape_codes;

        // `stack:` authored as a map (its items missing the leading `-`)
        // rather than a YAML list. This is a type mismatch, NOT an
        // unknown-field error, so no field name / "Expected one of" catalog
        // must be fabricated.
        let frontmatter = json!({
            "initialize": {
                "stack": {
                    "when": "phase >= total_phases",
                    "action": [{ "warn": "too big" }]
                }
            }
        });

        let err = parse_lifecycle_config(&frontmatter, dummy_path()).unwrap_err();
        let CompositionError::LifecycleInvalid {
            property,
            message,
            unknown_field,
            expected_fields,
            ..
        } = &err
        else {
            panic!("expected LifecycleInvalid, got {err:?}");
        };

        assert_eq!(property, "initialize");
        assert!(unknown_field.is_none());
        assert!(expected_fields.is_empty());
        assert!(
            message.contains("expected a sequence"),
            "raw serde message should be preserved: {message}"
        );

        let rendered = strip_escape_codes(err.report_block_error_optimistic(Some(80)));
        assert!(
            !rendered.contains("Unknown property"),
            "must not fabricate an unknown-property diagnostic: {rendered}"
        );
        assert!(
            !rendered.contains("Expected one of"),
            "must not fabricate a field catalog: {rendered}"
        );
        assert!(
            rendered.contains("stack"),
            "hint should point at the `stack` list shape: {rendered}"
        );
    }

    // =====================================================================
    // Phase 2: extended event inventory, lifecycle concerns, stacks
    // =====================================================================

    #[test]
    fn all_seven_signals_have_canonical_property_names() {
        assert_eq!(LifecycleSignal::Initialize.property_name(), "initialize");
        assert_eq!(LifecycleSignal::Start.property_name(), "start");
        assert_eq!(LifecycleSignal::Success.property_name(), "success");
        assert_eq!(LifecycleSignal::Blocked.property_name(), "blocked");
        assert_eq!(LifecycleSignal::Failure.property_name(), "failure");
        assert_eq!(LifecycleSignal::Finalize.property_name(), "finalize");
        assert_eq!(LifecycleSignal::Loop.property_name(), "loop");
    }

    #[test]
    fn signal_all_iterates_in_canonical_order() {
        let names: Vec<&'static str> =
            LifecycleSignal::ALL.iter().map(|s| s.property_name()).collect();
        assert_eq!(
            names,
            vec![
                "initialize",
                "start",
                "success",
                "blocked",
                "failure",
                "finalize",
                "loop",
            ]
        );
    }

    #[test]
    fn signal_can_carry_error_matrix() {
        // No-error events.
        for event in [
            LifecycleSignal::Initialize,
            LifecycleSignal::Start,
            LifecycleSignal::Success,
            LifecycleSignal::Loop,
        ] {
            assert!(
                !event.can_carry_error(),
                "{event:?} should not be able to carry an error"
            );
        }
        // Err-capable events.
        for event in [
            LifecycleSignal::Blocked,
            LifecycleSignal::Failure,
            LifecycleSignal::Finalize,
        ] {
            assert!(
                event.can_carry_error(),
                "{event:?} should be able to carry an error"
            );
        }
    }

    #[test]
    fn parses_initialize_finalize_top_level_events() {
        let fm = json!({
            "initialize": { "stderr": "composing" },
            "finalize": { "stderr": "cleanup" }
        });
        let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
        assert_eq!(
            config.initialize.as_ref().unwrap().stderr.as_deref(),
            Some("composing")
        );
        assert_eq!(
            config.finalize.as_ref().unwrap().stderr.as_deref(),
            Some("cleanup")
        );
        assert_eq!(
            config
                .get(LifecycleSignal::Initialize)
                .unwrap()
                .stderr
                .as_deref(),
            Some("composing")
        );
        assert_eq!(
            config
                .get(LifecycleSignal::Finalize)
                .unwrap()
                .stderr
                .as_deref(),
            Some("cleanup")
        );
    }

    #[test]
    fn parses_info_warn_and_success_top_level_fields() {
        let fm = json!({
            "start": { "info": "composing" },
            "failure": { "warn": "watch out" },
            "success": { "success": "all done" }
        });
        let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
        assert_eq!(
            config.start.as_ref().unwrap().info.as_deref(),
            Some("composing")
        );
        assert_eq!(
            config.failure.as_ref().unwrap().warn.as_deref(),
            Some("watch out")
        );
        assert_eq!(
            config.success.as_ref().unwrap().success.as_deref(),
            Some("all done")
        );
    }

    #[test]
    fn extracts_loop_lifecycle_concerns() {
        let fm = json!({
            "loop": {
                "while": "phase < total",
                "action": "increment(phase)",
                "say": "iterate",
                "stderr": "looping"
            }
        });
        let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
        let concerns = config.loop_concerns.as_ref().expect("loop concerns");
        assert_eq!(concerns.say.as_deref(), Some("iterate"));
        assert_eq!(concerns.stderr.as_deref(), Some("looping"));
        // `while` and `action` are iteration controls, not lifecycle
        // concerns, so they do not appear on the notification.
        assert_eq!(
            config.get(LifecycleSignal::Loop).unwrap().say.as_deref(),
            Some("iterate")
        );
    }

    #[test]
    fn empty_stack_is_normalized_to_none() {
        let fm = json!({
            "start": { "stack": [] }
        });
        let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
        assert!(config.stacks.start.is_none());
        assert!(config.stack(LifecycleSignal::Start).is_none());
    }

    #[test]
    fn rejects_short_form_say_action() {
        let fm = json!({
            "start": {
                "stack": [
                    {"action": "say('hello world')"}
                ]
            }
        });
        let err = parse_lifecycle_config(&fm, dummy_path()).unwrap_err();
        match err {
            CompositionError::LifecycleShortFormRemoved { raw, rewrite, .. } => {
                assert_eq!(raw, "say('hello world')");
                assert_eq!(rewrite, "say: \"hello world\"");
            }
            other => panic!("expected LifecycleShortFormRemoved, got: {other:?}"),
        }
    }

    #[test]
    fn positional_scalar_value_is_taken_literally() {
        // A positional scalar value is literal text by default — `ctx.repo` is
        // the text, not the context expression. Use a whole-value `{{ … }}`
        // span (resolved at event time) to interpolate a value.
        let fm = json!({
            "start": {
                "stack": [{"action": {"say": "ctx.repo"}}]
            }
        });
        let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
        let stack = config.stack(LifecycleSignal::Start).expect("start stack");
        let LifecycleActionKind::Communication(comm) = &stack[0].actions[0].kind else {
            panic!("expected communication action");
        };
        assert_eq!(comm.message, Expr::StringLiteral("ctx.repo".to_string()));
    }

    #[test]
    fn parses_when_condition_with_stack() {
        let fm = json!({
            "start": {
                "stack": [
                    {
                        "when": "env.AGENT == 'claude'",
                        "action": {"say": "using claude"}
                    }
                ]
            }
        });
        let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
        let stack = config.stack(LifecycleSignal::Start).expect("start stack");
        assert!(stack[0].when.is_some());
    }

    #[test]
    fn parses_multiple_actions_per_stack_item() {
        let fm = json!({
            "start": {
                "stack": [
                    {
                        "action": [
                            {"say": "first"},
                            {"info": "second"}
                        ]
                    }
                ]
            }
        });
        let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
        let stack = config.stack(LifecycleSignal::Start).expect("start stack");
        assert_eq!(stack[0].actions.len(), 2);
    }

    #[test]
    fn parses_stop_short_form() {
        let fm = json!({
            "initialize": {
                "stack": [{"action": "stop"}]
            }
        });
        let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
        let stack = config
            .stack(LifecycleSignal::Initialize)
            .expect("initialize stack");
        assert_eq!(stack[0].actions.len(), 1);
        assert!(stack[0].actions[0].is_lifecycle_control());
    }

    #[test]
    fn parses_skip_in_initialize() {
        let fm = json!({
            "initialize": {
                "stack": [{"action": "skip"}]
            }
        });
        let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
        let stack = config
            .stack(LifecycleSignal::Initialize)
            .expect("initialize stack");
        assert!(stack[0].actions[0].is_lifecycle_control());
    }

    #[test]
    fn parses_retry_with_count_in_blocked() {
        let fm = json!({
            "blocked": {
                "stack": [{"action": {"retry": 3}}]
            }
        });
        let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
        let stack = config
            .stack(LifecycleSignal::Blocked)
            .expect("blocked stack");
        assert!(stack[0].actions[0].is_lifecycle_control());
    }

    #[test]
    fn parses_proxy_with_file_arg_in_initialize() {
        let fm = json!({
            "initialize": {
                "stack": [{"action": {"proxy": "@fallback.md"}}]
            }
        });
        let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
        let stack = config
            .stack(LifecycleSignal::Initialize)
            .expect("initialize stack");
        assert!(stack[0].actions[0].is_lifecycle_control());
    }

    #[test]
    fn parses_shell_long_form_with_on_error_and_no_error() {
        // Long-form shell action: `command`, `on_error`, `no_error` live
        // inside the explicit `{ action: shell, ... }` object.
        let fm = json!({
            "start": {
                "stack": [
                    {
                        "action": {
                            "action": "shell",
                            "command": "git fetch --all",
                            "on_error": "fetch failed",
                            "no_error": true
                        }
                    }
                ]
            }
        });
        let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
        let stack = config.stack(LifecycleSignal::Start).expect("start stack");
        let action = &stack[0].actions[0];
        assert!(action.no_error);
    }

    #[test]
    fn parses_side_effect_long_form() {
        // Side-effect long form: `file`, `prop`, `value` live inside the
        // explicit `{ action: set_frontmatter, ... }` object.
        let fm = json!({
            "start": {
                "stack": [
                    {
                        "action": {
                            "action": "set_frontmatter",
                            "file": "@spec.md",
                            "prop": "status",
                            "value": "in-progress"
                        }
                    }
                ]
            }
        });
        let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
        let _ = config.stack(LifecycleSignal::Start).expect("start stack");
    }

    #[test]
    fn parses_side_effect_short_form() {
        let fm = json!({
            "start": {
                "stack": [{"action": {"ensure_file": "@out/log.md"}}]
            }
        });
        let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
        let _ = config.stack(LifecycleSignal::Start).expect("start stack");
    }

    #[test]
    fn rejects_skip_outside_initialize() {
        let fm = json!({
            "start": {"stack": [{"action": "skip"}]}
        });
        let err = parse_lifecycle_config(&fm, dummy_path()).unwrap_err();
        match err {
            CompositionError::LifecycleActionPlacement {
                action, event, ..
            } => {
                assert_eq!(action, "skip");
                assert_eq!(event, "start");
            }
            other => panic!("expected LifecycleActionPlacement, got: {other:?}"),
        }
    }

    #[test]
    fn flow_control_is_universal_across_events() {
        // Flow control reacts to state, not just errors, so `error`/`retry`/
        // `resume`/`requeue`/`proxy` parse in every event (only `skip` is
        // placement-restricted, to `initialize`). E.g. a `success` stack may
        // `resume` because an expected artifact was not produced.
        let cases: [(&str, serde_json::Value); 6] = [
            ("start", json!({"proxy": "@other.md"})),
            ("start", json!({"retry": null})),
            (
                "success",
                json!({"resume": "the file abc.md was never written; create it"}),
            ),
            ("blocked", json!({"resume": "please"})),
            ("initialize", json!({"defer": "5m"})),
            ("success", json!({"retry": 2})),
        ];
        for (event, action) in cases {
            let fm = json!({ event: {"stack": [{"action": action}]} });
            parse_lifecycle_config(&fm, dummy_path())
                .unwrap_or_else(|e| panic!("`{action}` in `{event}` should parse, got: {e:?}"));
        }
        // `loop` carries iteration controls; a `requeue` there parses too.
        let loop_fm = json!({ "loop": {"while": "true", "stack": [{"action": {"defer": "5m"}}]} });
        parse_lifecycle_config(&loop_fm, dummy_path())
            .unwrap_or_else(|e| panic!("`requeue` in `loop` should parse, got: {e:?}"));
    }

    #[test]
    fn accepts_recovery_actions_in_finalize() {
        // `finalize` is the optional-error terminal event and a last-chance
        // recovery surface, so retry/resume/requeue/proxy all parse there
        // (parity with the `failure` event).
        for action in [
            json!({"retry": 1}),
            json!({"resume": "finish the task"}),
            json!({"defer": "5m"}),
            json!({"proxy": "@other.md"}),
        ] {
            let fm = json!({
                "finalize": {"stack": [{"when": "err", "action": action}]}
            });
            parse_lifecycle_config(&fm, dummy_path())
                .unwrap_or_else(|e| panic!("finalize `{action}` should parse, got: {e:?}"));
        }
    }

    #[test]
    fn rejects_multiple_lifecycle_actions_in_one_item() {
        let fm = json!({
            "blocked": {
                "stack": [
                    {"action": ["stop", "skip"]}
                ]
            }
        });
        let err = parse_lifecycle_config(&fm, dummy_path()).unwrap_err();
        assert!(matches!(
            err,
            CompositionError::LifecycleMultipleLifecycleActions { .. }
        ));
    }

    #[test]
    fn rejects_lifecycle_action_not_last() {
        let fm = json!({
            "initialize": {
                "stack": [
                    {"action": ["stop", {"say": "unreachable"}]}
                ]
            }
        });
        let err = parse_lifecycle_config(&fm, dummy_path()).unwrap_err();
        assert!(matches!(
            err,
            CompositionError::LifecycleActionOrder { .. }
        ));
    }

    #[test]
    fn accepts_lifecycle_action_as_last() {
        let fm = json!({
            "initialize": {
                "stack": [
                    {"action": [{"say": "one"}, "stop"]}
                ]
            }
        });
        let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
        let stack = config
            .stack(LifecycleSignal::Initialize)
            .expect("initialize stack");
        assert_eq!(stack[0].actions.len(), 2);
        assert!(!stack[0].actions[0].is_lifecycle_control());
        assert!(stack[0].actions[1].is_lifecycle_control());
    }

    #[test]
    fn control_checks_fire_identically_for_key_value_form() {
        // The cardinality, ordering, and placement checks operate on the parsed
        // typed `LifecycleControlAction` — independent of whether the author
        // wrote the control positional (`{"action": "skip"}` / `{"stop": null}`)
        // or key/value (`{"action": {"action": "stop"}}`). The positional-form
        // tests above already pin the behavior; this pins the same diagnostics
        // for the key/value form so the two forms cannot drift.

        // Placement: a key/value `skip` outside `initialize` is the same
        // LifecycleActionPlacement error the positional `{"action": "skip"}`
        // trips in `rejects_skip_outside_initialize`.
        let fm = json!({
            "start": {"stack": [{"action": {"action": "skip"}}]}
        });
        match parse_lifecycle_config(&fm, dummy_path()).unwrap_err() {
            CompositionError::LifecycleActionPlacement { action, event, .. } => {
                assert_eq!(action, "skip");
                assert_eq!(event, "start");
            }
            other => panic!("expected LifecycleActionPlacement, got: {other:?}"),
        }

        // Cardinality: two key/value control actions in one item trip
        // LifecycleMultipleLifecycleActions (parity with the positional
        // `["stop", "skip"]` case).
        let fm = json!({
            "blocked": {"stack": [{"action": [{"action": "stop"}, {"action": "skip"}]}]}
        });
        assert!(matches!(
            parse_lifecycle_config(&fm, dummy_path()).unwrap_err(),
            CompositionError::LifecycleMultipleLifecycleActions { .. }
        ));

        // Ordering: a key/value control action before a non-control action trips
        // LifecycleActionOrder (parity with `["stop", {"say": ...}]`).
        let fm = json!({
            "initialize": {"stack": [{"action": [
                {"action": "stop"},
                {"action": "say", "message": "unreachable"}
            ]}]}
        });
        assert!(matches!(
            parse_lifecycle_config(&fm, dummy_path()).unwrap_err(),
            CompositionError::LifecycleActionOrder { .. }
        ));

        // Positive parity: a key/value control action as the LAST item is
        // accepted, exactly like the positional form.
        let fm = json!({
            "initialize": {"stack": [{"action": [
                {"action": "say", "message": "one"},
                {"action": "stop"}
            ]}]}
        });
        let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
        let stack = config
            .stack(LifecycleSignal::Initialize)
            .expect("initialize stack");
        assert_eq!(stack[0].actions.len(), 2);
        assert!(!stack[0].actions[0].is_lifecycle_control());
        assert!(stack[0].actions[1].is_lifecycle_control());
    }

    #[test]
    fn positional_scalar_value_is_literal_text() {
        // Positional scalar values are literal text by default — `using codex`
        // is the text, not an expression. Commas and colons inside are part of
        // the message.
        let cases: [( &str, serde_json::Value, &str); 4] = [
            ("say", json!({"say": "using codex"}), "using codex"),
            (
                "warn",
                json!({"warn": "phase 6, too big"}),
                "phase 6, too big",
            ),
            (
                "error",
                json!({"error": "invalid phase: 6"}),
                "invalid phase: 6",
            ),
            (
                "effect",
                json!({"effect": "crowd-applause"}),
                "crowd-applause",
            ),
        ];
        for (verb, action, expected) in cases {
            let fm = json!({ "blocked": { "stack": [{"action": action}] } });
            let config = parse_lifecycle_config(&fm, dummy_path())
                .unwrap_or_else(|e| panic!("`{verb}` positional scalar should parse, got: {e:?}"));
            let stack = config.stack(LifecycleSignal::Blocked).expect("blocked stack");
            let message = match &stack[0].actions[0].kind {
                LifecycleActionKind::Communication(c) => &c.message,
                LifecycleActionKind::LifecycleControl(LifecycleControlAction::Error {
                    reason: Some(r),
                }) => r,
                other => panic!("unexpected action kind for `{verb}`: {other:?}"),
            };
            assert_eq!(message, &Expr::StringLiteral(expected.to_string()), "{verb}");
        }
    }

    #[test]
    fn rejects_missing_closing_paren() {
        let fm = json!({
            "start": {
                "stack": [{"action": "say('hi'"}]
            }
        });
        let err = parse_lifecycle_config(&fm, dummy_path()).unwrap_err();
        assert!(matches!(
            err,
            CompositionError::LifecycleShortFormRemoved { .. }
        ));
    }

    #[test]
    fn rejects_retry_with_too_many_args() {
        let fm = json!({
            "blocked": {
                "stack": [{"action": {"retry": [3, 4]}}]
            }
        });
        let err = parse_lifecycle_config(&fm, dummy_path()).unwrap_err();
        assert!(matches!(
            err,
            CompositionError::LifecycleWrongArity { .. }
        ));
    }

    #[test]
    fn rejects_proxy_missing_target() {
        // `proxy` requires a `target` parameter; a null positional value is
        // wrong arity.
        let fm = json!({
            "initialize": {
                "stack": [{"action": {"proxy": null}}]
            }
        });
        let err = parse_lifecycle_config(&fm, dummy_path()).unwrap_err();
        assert!(matches!(
            err,
            CompositionError::LifecycleWrongArity { .. }
        ));
    }

    #[test]
    fn rejects_stack_item_missing_action_key() {
        let fm = json!({
            "start": {
                "stack": [{"when": "true"}]
            }
        });
        let err = parse_lifecycle_config(&fm, dummy_path()).unwrap_err();
        assert!(matches!(
            err,
            CompositionError::LifecycleStackInvalidShape { .. }
        ));
    }

    #[test]
    fn rejects_unknown_stack_item_key() {
        // A scalar `action` value cannot carry sibling parameter keys; the
        // `bogus` key is rejected as an invalid stack-item shape.
        let fm = json!({
            "start": {
                "stack": [{"action": "stop", "bogus": true}]
            }
        });
        let err = parse_lifecycle_config(&fm, dummy_path()).unwrap_err();
        assert!(matches!(
            err,
            CompositionError::LifecycleStackInvalidShape { .. }
        ));
    }

    #[test]
    fn rejects_stack_item_that_is_not_an_object() {
        let fm = json!({
            "start": {
                "stack": ["stop"]
            }
        });
        let err = parse_lifecycle_config(&fm, dummy_path()).unwrap_err();
        assert!(matches!(
            err,
            CompositionError::LifecycleStackInvalidShape { .. }
        ));
    }

    // -- positional action parser (Phase 4) ----------------------------------

    #[test]
    fn parses_positional_communication_scalar() {
        let fm = json!({
            "success": {
                "stack": [
                    {"action": {"message": "hello"}},
                    {"action": {"effect": "crowd-applause"}},
                    {"action": {"stderr": "an error"}},
                    {"action": {"success": "it worked"}}
                ]
            }
        });
        let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
        let stack = config.stack(LifecycleSignal::Success).expect("success stack");
        assert_eq!(stack.len(), 4);
        for item in stack {
            assert!(matches!(item.actions[0].kind, LifecycleActionKind::Communication(_)));
        }
    }

    #[test]
    fn parses_positional_shell_scalar() {
        let fm = json!({
            "start": {
                "stack": [{"action": {"shell": "git status"}}]
            }
        });
        let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
        let stack = config.stack(LifecycleSignal::Start).expect("start stack");
        assert!(matches!(stack[0].actions[0].kind, LifecycleActionKind::Shell(_)));
    }

    #[test]
    fn parses_positional_side_effect_array() {
        let fm = json!({
            "start": {
                "stack": [{"action": {"set_frontmatter": ["s.md", "status", "ready"]}}]
            }
        });
        let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
        let stack = config.stack(LifecycleSignal::Start).expect("start stack");
        let action = &stack[0].actions[0];
        let LifecycleActionKind::SideEffect(se) = &action.kind else {
            panic!("expected side-effect action, got {action:?}");
        };
        assert_eq!(se.verb, "set_frontmatter");
        assert_eq!(se.args.len(), 3);
    }

    #[test]
    fn parses_positional_optional_tail_side_effect() {
        let fm = json!({
            "start": {
                "stack": [
                    {"action": {"ensure_file": ["out/log.md"]}},
                    {"action": {"ensure_file": ["out/log.md", "# log"]}}
                ]
            }
        });
        let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
        let stack = config.stack(LifecycleSignal::Start).expect("start stack");
        assert_eq!(stack.len(), 2);
        for item in stack {
            let LifecycleActionKind::SideEffect(se) = &item.actions[0].kind else {
                panic!("expected side-effect action");
            };
            assert_eq!(se.verb, "ensure_file");
        }
    }

    #[test]
    fn parses_positional_control_verbs() {
        let fm = json!({
            "initialize": {
                "stack": [
                    {"action": {"stop": null}},
                    {"action": {"stop": []}},
                    {"action": {"error": "reason"}},
                    {"action": {"retry": 3}},
                    {"action": {"proxy": "@other.md"}}
                ]
            }
        });
        let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
        let stack = config.stack(LifecycleSignal::Initialize).expect("init stack");
        assert_eq!(stack.len(), 5);
        for item in stack {
            assert!(item.actions[0].is_lifecycle_control());
        }
    }

    #[test]
    fn parses_positional_expression_function_variadic() {
        let fm = json!({
            "start": {
                "stack": [
                    {"action": {"and": ["true", "true", "false"]}},
                    {"action": {"or": ["a", "b"]}}
                ]
            }
        });
        let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
        let stack = config.stack(LifecycleSignal::Start).expect("start stack");
        assert_eq!(stack.len(), 2);
        for item in stack {
            assert!(matches!(
                item.actions[0].kind,
                LifecycleActionKind::ExpressionFunction(_)
            ));
        }
    }

    #[test]
    fn parses_positional_expression_function_concrete() {
        let fm = json!({
            "start": {
                "stack": [{"action": {"length": "{{ items }}"}}]
            }
        });
        let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
        let stack = config.stack(LifecycleSignal::Start).expect("start stack");
        let LifecycleActionKind::ExpressionFunction(ef) = &stack[0].actions[0].kind else {
            panic!("expected expression-function action");
        };
        assert_eq!(ef.function, "length");
        assert_eq!(ef.args.len(), 1);
    }

    #[test]
    fn parses_positional_expression_function_bracket_optional() {
        // `number(x, [default])` — the bracketed param is optional, so the
        // one-argument form is valid arity.
        let fm = json!({
            "start": {
                "stack": [{"action": {"number": "{{ value }}"}}]
            }
        });
        let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
        let stack = config.stack(LifecycleSignal::Start).expect("start stack");
        let LifecycleActionKind::ExpressionFunction(ef) = &stack[0].actions[0].kind else {
            panic!("expected expression-function action");
        };
        assert_eq!(ef.function, "number");
        assert_eq!(ef.args.len(), 1);
    }

    #[test]
    fn parses_positional_expression_function_overload_one_arg() {
        // Overloaded functions accept their shortest (one-argument) form: the
        // longer overload's extra parameters are optional.
        let fm = json!({
            "start": {
                "stack": [
                    {"action": {"frontmatter": "state.md"}},
                    {"action": {"link": "state.md"}}
                ]
            }
        });
        let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
        let stack = config.stack(LifecycleSignal::Start).expect("start stack");
        assert_eq!(stack.len(), 2);

        let LifecycleActionKind::ExpressionFunction(frontmatter) = &stack[0].actions[0].kind else {
            panic!("expected frontmatter expression-function action");
        };
        assert_eq!(frontmatter.function, "frontmatter");
        assert_eq!(frontmatter.args.len(), 1);

        let LifecycleActionKind::ExpressionFunction(link) = &stack[1].actions[0].kind else {
            panic!("expected link expression-function action");
        };
        assert_eq!(link.function, "link");
        assert_eq!(link.args.len(), 1);
    }

    #[test]
    fn parses_positional_expression_function_happy_path() {
        // Confirm the existing fixed-arity expression functions still parse.
        let fm = json!({
            "start": {
                "stack": [
                    {"action": {"length": "{{ items }}"}},
                    {"action": {"contains": ["{{ haystack }}", "{{ needle }}"]}},
                    {"action": {"and": ["true", "true"]}},
                    {"action": {"or": ["a", "b"]}}
                ]
            }
        });
        let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
        let stack = config.stack(LifecycleSignal::Start).expect("start stack");
        assert_eq!(stack.len(), 4);
        for item in stack {
            assert!(matches!(
                item.actions[0].kind,
                LifecycleActionKind::ExpressionFunction(_)
            ));
        }
    }

    #[test]
    fn parses_positional_typed_arguments() {
        let fm = json!({
            "start": {
                "stack": [
                    {"action": {"set_frontmatter": ["s.md", "ready", "{{ true }}"]}},
                    {"action": {"merge_frontmatter": ["s.md", "{{ payload }}"]}}
                ]
            }
        });
        let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
        let stack = config.stack(LifecycleSignal::Start).expect("start stack");

        let LifecycleActionKind::SideEffect(set) = &stack[0].actions[0].kind else {
            panic!("expected set_frontmatter side-effect");
        };
        assert_eq!(set.args[2], Expr::BoolLiteral(true));

        let LifecycleActionKind::SideEffect(merge) = &stack[1].actions[0].kind else {
            panic!("expected merge_frontmatter side-effect");
        };
        assert!(matches!(merge.args[1], Expr::Variable(_)));
    }

    #[test]
    fn parses_positional_action_object_value() {
        // `action: { success: "..." }` is the single-object positional form.
        let fm = json!({
            "success": {
                "stack": [{"action": {"success": "it worked"}}]
            }
        });
        let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
        let stack = config.stack(LifecycleSignal::Success).expect("success stack");
        assert_eq!(stack[0].actions.len(), 1);
        assert!(matches!(
            stack[0].actions[0].kind,
            LifecycleActionKind::Communication(_)
        ));
    }

    #[test]
    fn rejects_positional_wrong_arity_side_effect() {
        let fm = json!({
            "start": {
                "stack": [{"action": {"set_frontmatter": ["s.md"]}}]
            }
        });
        let err = parse_lifecycle_config(&fm, dummy_path()).unwrap_err();
        assert!(
            matches!(err, CompositionError::LifecycleWrongArity { .. }),
            "expected wrong-arity error, got: {err:?}"
        );
    }

    #[test]
    fn rejects_positional_wrong_arity_communication() {
        let fm = json!({
            "success": {
                "stack": [{"action": {"message": ["a", "b"]}}]
            }
        });
        let err = parse_lifecycle_config(&fm, dummy_path()).unwrap_err();
        assert!(
            matches!(err, CompositionError::LifecycleWrongArity { .. }),
            "expected wrong-arity error, got: {err:?}"
        );
    }

    #[test]
    fn rejects_positional_bare_proxy_as_wrong_arity() {
        // `proxy` requires a target; a null/empty-array value is wrong arity,
        // not a short-form issue.
        let fm = json!({
            "initialize": {
                "stack": [{"action": {"proxy": null}}]
            }
        });
        let err = parse_lifecycle_config(&fm, dummy_path()).unwrap_err();
        assert!(
            matches!(err, CompositionError::LifecycleWrongArity { .. }),
            "expected wrong-arity error, got: {err:?}"
        );
    }

    #[test]
    fn rejects_positional_unknown_verb() {
        let fm = json!({
            "start": {
                "stack": [{"action": {"sucess": "it worked"}}]
            }
        });
        let err = parse_lifecycle_config(&fm, dummy_path()).unwrap_err();
        match err {
            CompositionError::LifecycleUnknownVerb { verb, .. } => {
                assert_eq!(verb, "sucess");
            }
            other => panic!("expected LifecycleUnknownVerb, got: {other:?}"),
        }
    }

    #[test]
    fn rejects_positional_object_value() {
        let fm = json!({
            "start": {
                "stack": [{"action": {"merge_frontmatter": {"status": "ready"}}}]
            }
        });
        let err = parse_lifecycle_config(&fm, dummy_path()).unwrap_err();
        assert!(
            matches!(
                err,
                CompositionError::LifecycleObjectDataThroughInterpolationPositional { .. }
            ),
            "expected object-data-through-interpolation error, got: {err:?}"
        );
    }

    #[test]
    fn rejects_ambiguous_multi_key_action_object() {
        let fm = json!({
            "start": {
                "stack": [{"action": {"message": "hi", "route": "team"}}]
            }
        });
        let err = parse_lifecycle_config(&fm, dummy_path()).unwrap_err();
        assert!(
            matches!(err, CompositionError::LifecycleStackAmbiguous { .. }),
            "expected ambiguous error, got: {err:?}"
        );
    }

    #[test]
    fn positional_and_key_value_action_object_coexist_in_array() {
        // The motivating shape from the spec: positional and key/value actions
        // in the same stack array.
        let fm = json!({
            "success": {
                "stack": [
                    {
                        "when": "true",
                        "action": [
                            {"success": "it worked"},
                            {"set_frontmatter": ["s.md", "status", "done"]},
                            {"action": "shell", "command": "git push"}
                        ]
                    }
                ]
            }
        });
        let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
        let stack = config.stack(LifecycleSignal::Success).expect("success stack");
        assert_eq!(stack[0].actions.len(), 3);
        assert!(matches!(
            stack[0].actions[0].kind,
            LifecycleActionKind::Communication(_)
        ));
        assert!(matches!(
            stack[0].actions[1].kind,
            LifecycleActionKind::SideEffect(_)
        ));
        assert!(matches!(stack[0].actions[2].kind, LifecycleActionKind::Shell(_)));
    }

    #[test]
    fn parses_stdout_field_on_event_block() {
        let fm = json!({
            "start": {"stdout": "hello"}
        });
        let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
        assert_eq!(
            config.start.as_ref().unwrap().stdout.as_deref(),
            Some("hello")
        );
    }

    #[test]
    fn parses_stdout_short_form_action() {
        let fm = json!({
            "start": {
                "stack": [{"action": {"stdout": "hello"}}]
            }
        });
        // `stdout: ...` is a recognized positional communication action;
        // parsing succeeds and produces a single-item stack.
        let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
        assert_eq!(config.stack(LifecycleSignal::Start).unwrap().len(), 1);
    }

    #[test]
    fn parses_stdout_field_on_loop_block() {
        // A top-level `loop.stdout` is extracted as a loop lifecycle concern,
        // alongside the iteration controls. The `while` key keeps the loop
        // block otherwise valid.
        let fm = json!({
            "loop": {"while": "true", "stdout": "hello"}
        });
        let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
        assert_eq!(
            config.loop_concerns.as_ref().unwrap().stdout.as_deref(),
            Some("hello")
        );
    }

    #[test]
    fn legacy_top_level_only_prompts_still_parse() {
        // Legacy prompts that only configure the four original top-level
        // events (`start`, `success`, `blocked`, `failure`) continue to parse
        // and expose those events through `LifecycleConfig::get` exactly as
        // before the seven-event model was introduced.
        let fm = json!({
            "start":   { "stderr": "starting" },
            "success": { "stderr": "done" },
            "blocked": { "stderr": "blocked" },
            "failure": { "stderr": "failed" }
        });
        let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
        assert!(config.initialize.is_none());
        assert!(config.finalize.is_none());
        assert!(config.loop_concerns.is_none());
        assert!(config.stacks.start.is_none());
        assert!(!config.is_empty());
        // `get` continues to work for the four legacy signals.
        for s in [
            LifecycleSignal::Start,
            LifecycleSignal::Success,
            LifecycleSignal::Blocked,
            LifecycleSignal::Failure,
        ] {
            assert!(config.get(s).is_some(), "expected {s:?} to be configured");
        }
    }

    // =====================================================================
    // Phase 5: positional-and-key-value action validation checkpoint
    // =====================================================================

    #[test]
    fn short_form_rejection_rewrites_to_positional() {
        // Removed `verb(args)` short form is rejected with a did-you-mean
        // positional rewrite.
        let cases: [(&str, serde_json::Value, &str); 3] = [
            ("success", json!({"success": "x"}), "success: \"x\""),
            ("shell", json!({"shell": "git push"}), "shell: \"git push\""),
            (
                "set_frontmatter",
                json!({"set_frontmatter": ["a", "b", "c"]}),
                "set_frontmatter: [\"a\", \"b\", \"c\"]",
            ),
        ];
        for (verb, action, expected_rewrite) in cases {
            let short_form = format!("{verb}({})", match verb {
                "success" => "\"x\"".to_string(),
                "shell" => "git push".to_string(),
                "set_frontmatter" => "'a','b','c'".to_string(),
                _ => unreachable!(),
            });
            let fm = json!({
                "start": {
                    "stack": [{"action": short_form.clone()}]
                }
            });
            let err = parse_lifecycle_config(&fm, dummy_path()).unwrap_err();
            match err {
                CompositionError::LifecycleShortFormRemoved { raw, rewrite, .. } => {
                    assert_eq!(raw, short_form, "{verb}");
                    assert_eq!(rewrite, expected_rewrite, "{verb}");
                }
                other => panic!("expected LifecycleShortFormRemoved for {verb}, got: {other:?}"),
            }

            // The positional rewrite itself parses cleanly.
            let fm = json!({
                "start": {
                    "stack": [{"action": action}]
                }
            });
            assert!(
                parse_lifecycle_config(&fm, dummy_path()).is_ok(),
                "{verb} positional rewrite should parse"
            );
        }
    }

    #[test]
    fn bare_stop_accepted_bare_proxy_rejected_wrong_arity() {
        // Zero-arg positional: bare `stop` is accepted.
        let fm = json!({
            "initialize": {
                "stack": [{"action": "stop"}]
            }
        });
        assert!(parse_lifecycle_config(&fm, dummy_path()).is_ok());

        // `proxy` requires a target; a bare verb is wrong arity.
        let fm = json!({
            "initialize": {
                "stack": [{"action": "proxy"}]
            }
        });
        let err = parse_lifecycle_config(&fm, dummy_path()).unwrap_err();
        assert!(
            matches!(err, CompositionError::LifecycleWrongArity { ref verb, .. } if verb == "proxy"),
            "expected wrong-arity for bare proxy, got: {err:?}"
        );
    }

    #[test]
    fn key_value_literal_default_vs_whole_value_interpolation() {
        // Key/value literal default: a plain string parameter is a literal.
        let fm = json!({
            "start": {
                "stack": [{"action": {"action": "message", "message": "ctx.area"}}]
            }
        });
        let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
        let stack = config.stack(LifecycleSignal::Start).expect("start stack");
        let LifecycleActionKind::Communication(comm) = &stack[0].actions[0].kind else {
            panic!("expected communication action");
        };
        assert_eq!(comm.message, Expr::StringLiteral("ctx.area".to_string()));

        // Whole-value interpolation resolves the expression at event time.
        let fm = json!({
            "start": {
                "stack": [{"action": {"action": "message", "message": "{{ ctx.area }}"}}]
            }
        });
        let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
        let stack = config.stack(LifecycleSignal::Start).expect("start stack");
        let LifecycleActionKind::Communication(comm) = &stack[0].actions[0].kind else {
            panic!("expected communication action");
        };
        assert_eq!(comm.message, Expr::Variable("ctx.area".to_string()));
    }

    #[test]
    fn full_disambiguation_table_for_positional_and_key_value() {
        // Same verb as positional single-key object and as explicit key/value.
        let positional = json!({"start": {"stack": [{"action": {"success": "it worked"}}]}});
        let key_value = json!({
            "start": {
                "stack": [{"action": {"action": "success", "message": "it worked"}}]
            }
        });
        for fm in [&positional, &key_value] {
            let config = parse_lifecycle_config(fm, dummy_path()).unwrap();
            let stack = config.stack(LifecycleSignal::Start).expect("start stack");
            assert!(matches!(
                stack[0].actions[0].kind,
                LifecycleActionKind::Communication(_)
            ));
        }

        // Multi-key object without an `action` key is ambiguous.
        let fm = json!({
            "start": {
                "stack": [{"action": {"message": "hi", "route": "team"}}]
            }
        });
        let err = parse_lifecycle_config(&fm, dummy_path()).unwrap_err();
        assert!(
            matches!(err, CompositionError::LifecycleStackAmbiguous { .. }),
            "expected ambiguous error, got: {err:?}"
        );
    }

    #[test]
    fn predicate_exception_when_evaluates_expression_scalar_stays_literal() {
        // `when` is always a boolean expression.
        let fm = json!({
            "start": {
                "stack": [
                    {"when": "true", "action": {"say": "true"}}
                ]
            }
        });
        let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
        let stack = config.stack(LifecycleSignal::Start).expect("start stack");
        assert!(stack[0].when.is_some());

        // The positional scalar `"true"` is literal text, not a bool.
        let LifecycleActionKind::Communication(comm) = &stack[0].actions[0].kind else {
            panic!("expected communication action");
        };
        assert_eq!(comm.message, Expr::StringLiteral("true".to_string()));
    }

    #[test]
    fn known_verb_validation_for_typoed_positional_and_key_value() {
        // Typoed positional verb gets a did-you-mean suggestion.
        let fm = json!({
            "success": {
                "stack": [{"action": {"sucess": "it worked"}}]
            }
        });
        let err = parse_lifecycle_config(&fm, dummy_path()).unwrap_err();
        match err {
            CompositionError::LifecycleUnknownVerb { verb, rewrite, .. } => {
                assert_eq!(verb, "sucess");
                assert!(rewrite.contains("success"), "got: {rewrite}");
            }
            other => panic!("expected LifecycleUnknownVerb for positional typo, got: {other:?}"),
        }

        // Typoed key/value verb gets the same suggestion.
        let fm = json!({
            "success": {
                "stack": [{"action": {"action": "sucess", "message": "it worked"}}]
            }
        });
        let err = parse_lifecycle_config(&fm, dummy_path()).unwrap_err();
        match err {
            CompositionError::LifecycleUnknownVerb { verb, rewrite, .. } => {
                assert_eq!(verb, "sucess");
                assert!(rewrite.contains("success"), "got: {rewrite}");
            }
            other => panic!("expected LifecycleUnknownVerb for key/value typo, got: {other:?}"),
        }
    }

    #[test]
    fn expression_function_actions_positional_key_value_and_variadic_rejection() {
        // Positional expression-function action.
        let fm = json!({
            "start": {
                "stack": [{"action": {"length": "{{ items }}"}}]
            }
        });
        let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
        let stack = config.stack(LifecycleSignal::Start).expect("start stack");
        let LifecycleActionKind::ExpressionFunction(ef) = &stack[0].actions[0].kind else {
            panic!("expected expression-function action");
        };
        assert_eq!(ef.function, "length");
        assert_eq!(ef.args.len(), 1);
        assert_eq!(ef.args[0], Expr::Variable("items".to_string()));

        // Key/value expression-function action with concrete named parameters.
        let fm = json!({
            "start": {
                "stack": [{
                    "action": {
                        "action": "contains",
                        "haystack": "{{ haystack }}",
                        "needle": "needle"
                    }
                }]
            }
        });
        let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
        let stack = config.stack(LifecycleSignal::Start).expect("start stack");
        let LifecycleActionKind::ExpressionFunction(ef) = &stack[0].actions[0].kind else {
            panic!("expected expression-function action");
        };
        assert_eq!(ef.function, "contains");
        assert_eq!(ef.args.len(), 2);

        // Variadic expression functions reject key/value form.
        for verb in ["and", "or"] {
            let fm = json!({
                "start": {
                    "stack": [{"action": {"action": verb, "a": "true", "b": "false"}}]
                }
            });
            let err = parse_lifecycle_config(&fm, dummy_path()).unwrap_err();
            assert!(
                matches!(
                    err,
                    CompositionError::LifecycleExpressionFunctionKeyValueUnsupported {
                        verb: ref v, ..
                    } if v == verb
                ),
                "{verb} key/value should be rejected, got: {err:?}"
            );
        }
    }

    #[test]
    fn key_value_expression_function_rejects_missing_required_param() {
        // `contains(haystack, needle)` — both required. Supplying only
        // `haystack` must fail at parse time, naming the missing `needle`.
        let fm = json!({
            "start": {
                "stack": [{
                    "action": {
                        "action": "contains",
                        "haystack": "{{ haystack }}"
                    }
                }]
            }
        });
        let err = parse_lifecycle_config(&fm, dummy_path()).unwrap_err();
        match err {
            CompositionError::LifecycleActionInvalidLongForm {
                action, message, ..
            } => {
                assert_eq!(action, "contains");
                assert!(
                    message.contains("needle"),
                    "message should name the missing `needle` param, got: {message}"
                );
            }
            other => panic!("expected LifecycleActionInvalidLongForm, got: {other:?}"),
        }
    }

    #[test]
    fn key_value_side_effect_rejects_missing_required_params() {
        // `set_frontmatter(file, prop, value)` — all required. Supplying only
        // `file` must fail at parse time, naming both missing params.
        let fm = json!({
            "start": {
                "stack": [{
                    "action": {
                        "action": "set_frontmatter",
                        "file": "@state.md"
                    }
                }]
            }
        });
        let err = parse_lifecycle_config(&fm, dummy_path()).unwrap_err();
        match err {
            CompositionError::LifecycleActionInvalidLongForm {
                action, message, ..
            } => {
                assert_eq!(action, "set_frontmatter");
                assert!(
                    message.contains("prop") && message.contains("value"),
                    "message should name both missing params, got: {message}"
                );
            }
            other => panic!("expected LifecycleActionInvalidLongForm, got: {other:?}"),
        }
    }

    #[test]
    fn key_value_omitting_optional_tail_param_parses() {
        // `frontmatter(file, [prop])` (expression function) — `prop` is an
        // optional tail param, so the `file`-only key/value form is valid.
        let fm = json!({
            "start": {
                "stack": [{
                    "action": {
                        "action": "frontmatter",
                        "file": "@spec.md"
                    }
                }]
            }
        });
        let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
        let stack = config.stack(LifecycleSignal::Start).expect("start stack");
        let LifecycleActionKind::ExpressionFunction(ef) = &stack[0].actions[0].kind else {
            panic!("expected expression-function action");
        };
        assert_eq!(ef.function, "frontmatter");
        assert_eq!(ef.args.len(), 1);

        // `ensure_file(file, [content])` (side effect) — `content` is optional,
        // so the `file`-only key/value form is valid.
        let fm = json!({
            "start": {
                "stack": [{
                    "action": {
                        "action": "ensure_file",
                        "file": "@out/log.md"
                    }
                }]
            }
        });
        let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
        let stack = config.stack(LifecycleSignal::Start).expect("start stack");
        let LifecycleActionKind::SideEffect(se) = &stack[0].actions[0].kind else {
            panic!("expected side-effect action");
        };
        assert_eq!(se.verb, "ensure_file");
        assert_eq!(se.args.len(), 1);
    }

    #[test]
    fn empty_frontmatter_yields_empty_seven_event_config() {
        let fm = json!({});
        let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
        assert!(config.is_empty());
        for s in LifecycleSignal::ALL {
            assert!(config.get(s).is_none(), "expected {s:?} to be None");
            assert!(config.stack(s).is_none(), "expected stack for {s:?} to be None");
        }
    }

    #[test]
    fn parse_lifecycle_config_handles_non_object_frontmatter() {
        let fm = json!("scalar");
        let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
        assert!(config.is_empty());
    }

    #[test]
    fn null_event_property_is_skipped() {
        let fm = json!({
            "initialize": null,
            "start": { "stderr": "go" }
        });
        let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
        assert!(config.initialize.is_none());
        assert!(config.start.is_some());
    }

    #[test]
    fn loop_concerns_stack_uses_loop_signal_for_placement() {
        // `Skip` is the one placement-restricted action (`initialize` only),
        // so it is invalid in the `loop` event.
        let fm = json!({
            "loop": {
                "while": "true",
                "stack": [{"action": "skip"}]
            }
        });
        let err = parse_lifecycle_config(&fm, dummy_path()).unwrap_err();
        match err {
            CompositionError::LifecycleActionPlacement { event, action, .. } => {
                assert_eq!(event, "loop");
                assert_eq!(action, "skip");
            }
            other => panic!("expected LifecycleActionPlacement, got: {other:?}"),
        }
    }

    #[test]
    fn stop_is_valid_in_every_event() {
        for s in LifecycleSignal::ALL {
            let fm = if s == LifecycleSignal::Loop {
                json!({
                    "loop": {
                        "while": "true",
                        "stack": [{"action": "stop"}]
                    }
                })
            } else {
                json!({
                    s.property_name(): {"stack": [{"action": "stop"}]}
                })
            };
            let config = parse_lifecycle_config(&fm, dummy_path());
            assert!(
                config.is_ok(),
                "`stop` should be valid in {s:?}, got: {:?}",
                config.err()
            );
        }
    }

    #[test]
    fn frontmatter_excerpt_included_for_placement_error() {
        // The `WithFrontmatter` wrapper is applied at the render boundary
        // (CLI handlers), not at the parse site. Here we only verify that the
        // underlying placement error carries the property name needed for
        // frontmatter highlighting.
        let fm = json!({
            "start": {"stack": [{"action": "skip"}]}
        });
        let err = parse_lifecycle_config(&fm, dummy_path()).unwrap_err();
        match err {
            CompositionError::LifecycleActionPlacement {
                property, event, ..
            } => {
                // The stack item is at index 0, so the annotated property
                // path is `start.stack[0]`. Frontmatter highlighting falls
                // back to the top-level `start` key when no per-stack-item
                // line is found.
                assert!(property.starts_with("start"), "got: {property}");
                assert_eq!(event, "start");
            }
            other => panic!("expected placement error, got: {other:?}"),
        }
    }

    // =====================================================================
    // Phase 3: lifecycle context, static scans, shell-audit collection
    // =====================================================================

    // -- err static scan ---------------------------------------------------

    #[test]
    fn err_in_start_stack_when_clause_is_rejected() {
        // `err` is forbidden in `start` (a no-error event) — even inside a
        // `when:` condition.
        let fm = json!({
            "start": {
                "stack": [
                    {"when": "err != null", "action": {"say": "has error"}}
                ]
            }
        });
        let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
        let err = validate_no_err_in_no_error_events(&config, dummy_path()).unwrap_err();
        match err {
            CompositionError::LifecycleErrNotAvailable { event, property, .. } => {
                assert_eq!(event, "start");
                assert!(property.contains("when"), "got: {property}");
            }
            other => panic!("expected LifecycleErrNotAvailable, got: {other:?}"),
        }
    }

    #[test]
    fn err_member_access_in_single_text_arg_is_literal() {
        // A positional scalar value is literal text by default — `err.msg` is
        // the text, not the `err` global. There is nothing to reject. To
        // reference the error in an error-carrying event, interpolate instead:
        // `{ say: "{{err.msg}}" }`.
        let fm = json!({
            "start": {
                "stack": [{"action": {"say": "err.msg"}}]
            }
        });
        let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
        assert!(validate_no_err_in_no_error_events(&config, dummy_path()).is_ok());
    }

    #[test]
    fn err_in_single_text_arg_is_literal_across_no_error_events() {
        // A positional scalar value is literal text in every no-error event —
        // the err-availability guard only governs expression surfaces (e.g.
        // `when:` clauses), not literal message bodies.
        for ev in ["initialize", "success"] {
            let fm = json!({
                ev: {"stack": [{"action": {"say": "err"}}]}
            });
            let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
            assert!(
                validate_no_err_in_no_error_events(&config, dummy_path()).is_ok(),
                "bare `err` in a {ev} message arg should be literal, not rejected"
            );
        }
        // Loop concerns live under `loop:`.
        let fm = json!({
            "loop": {
                "while": "true",
                "stack": [{"action": {"say": "err"}}]
            }
        });
        let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
        assert!(validate_no_err_in_no_error_events(&config, dummy_path()).is_ok());
    }

    #[test]
    fn err_in_blocked_failure_finalize_is_allowed() {
        // `err` is permitted in error-carrying events.
        for event in ["blocked", "failure", "finalize"] {
            let fm = json!({
                event: {"stack": [{"action": {"say": "err.msg"}}]}
            });
            let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
            let result = validate_no_err_in_no_error_events(&config, dummy_path());
            assert!(
                result.is_ok(),
                "err should be allowed in {event}, got: {:?}",
                result.err()
            );
        }
    }

    #[test]
    fn doc_err_escape_hatch_is_allowed_everywhere() {
        // `doc.err` reaches a literal frontmatter property, not the lifecycle
        // global, so it is permitted even in no-error events.
        for event in ["initialize", "start", "success", "loop"] {
            let fm = if event == "loop" {
                json!({
                    "loop": {
                        "while": "true",
                        "stack": [{"action": {"say": "doc.err"}}]
                    }
                })
            } else {
                json!({
                    event: {"stack": [{"action": {"say": "doc.err"}}]}
                })
            };
            let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
            let result = validate_no_err_in_no_error_events(&config, dummy_path());
            assert!(
                result.is_ok(),
                "doc.err should be allowed in {event}, got: {:?}",
                result.err()
            );
        }
    }

    #[test]
    fn err_in_control_reason_single_text_arg_is_literal() {
        // `error` with a positional scalar value takes its reason literally, so
        // `err.msg` is text, not a reference to the `err` global and is not
        // rejected.
        let fm = json!({
            "start": {
                "stack": [{"action": {"error": "err.msg"}}]
            }
        });
        let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
        assert!(validate_no_err_in_no_error_events(&config, dummy_path()).is_ok());
    }

    #[test]
    fn err_in_shell_command_single_text_arg_is_literal() {
        // `shell` with a positional scalar value takes its command literally, so
        // `err.msg` is text, not an `err`-global reference.
        let fm = json!({
            "loop": {
                "while": "true",
                "stack": [{"action": {"shell": "err.msg"}}]
            }
        });
        let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
        assert!(validate_no_err_in_no_error_events(&config, dummy_path()).is_ok());
    }

    // -- err static scan over interpolation spans (C4) --------------------

    #[test]
    fn err_interpolation_span_in_top_level_field_rejected_in_no_error_event() {
        // Late binding (C4): a top-level field reaches `err` only through a
        // `{{ … }}` span, and `err` is still forbidden in a no-error event.
        let fm = json!({ "start": { "message": "❌️  {{err.msg}}" } });
        let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
        let err = validate_no_err_in_no_error_events(&config, dummy_path()).unwrap_err();
        match err {
            CompositionError::LifecycleErrNotAvailable { event, property, .. } => {
                assert_eq!(event, "start");
                assert_eq!(property, "start.message");
            }
            other => panic!("expected LifecycleErrNotAvailable, got: {other:?}"),
        }
    }

    #[test]
    fn err_interpolation_span_in_stack_message_rejected_in_no_error_event() {
        // A positional scalar message body is literal text, but its `{{ … }}`
        // span still reaches the `err` global and must be rejected in `start`.
        let fm = json!({
            "start": { "stack": [{"action": {"message": "❌️  {{err.msg}}"}}] }
        });
        let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
        let err = validate_no_err_in_no_error_events(&config, dummy_path()).unwrap_err();
        match err {
            CompositionError::LifecycleErrNotAvailable { event, property, .. } => {
                assert_eq!(event, "start");
                assert!(property.starts_with("start.stack"), "got: {property}");
            }
            other => panic!("expected LifecycleErrNotAvailable, got: {other:?}"),
        }
    }

    #[test]
    fn timing_and_current_interpolation_allowed_in_no_error_events() {
        // `timing`/`current` are allowed everywhere, including no-error events.
        let fm = json!({
            "start": { "message": "took {{timing.document_ms}}ms on {{current.ctx.agent}}" }
        });
        let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
        assert!(validate_no_err_in_no_error_events(&config, dummy_path()).is_ok());
    }

    #[test]
    fn err_interpolation_span_allowed_in_error_carrying_event() {
        // The same `{{err.msg}}` span is fine in `failure` (an error event).
        let fm = json!({ "failure": { "message": "❌️  {{err.msg}}" } });
        let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
        assert!(validate_no_err_in_no_error_events(&config, dummy_path()).is_ok());
    }

    // -- deferred effect validation (C4) ----------------------------------

    #[test]
    fn effect_field_with_interpolation_skips_prepare_validation() {
        // An `effect: "{{name}}"` cannot be checked against the catalog at parse
        // time, so it parses cleanly and is validated at event-time instead.
        let fm = json!({ "success": { "effect": "{{effect_name}}" } });
        assert!(parse_lifecycle_config(&fm, dummy_path()).is_ok());
    }

    #[test]
    fn effect_field_literal_unknown_name_still_rejected_at_prepare() {
        // A literal (interpolation-free) unknown effect name is still rejected
        // at parse time.
        let fm = json!({ "success": { "effect": "nonexistent-effect-xyz" } });
        let err = parse_lifecycle_config(&fm, dummy_path()).unwrap_err();
        assert!(matches!(
            err,
            CompositionError::LifecycleUnknownEffect(_, _)
        ));
    }

    // -- stack leak scan ---------------------------------------------------

    #[test]
    fn stack_string_literal_with_interpolation_span_is_leak() {
        // A string literal inside a parsed expression that contains a
        // surviving `{{ … }}` span is a leak — the literal is passed through
        // verbatim to the evaluated result.
        let fm = json!({
            "start": {
                "stack": [{"action": {"say": "leaked {{ broken( }}"}}]
            }
        });
        let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
        let err = validate_no_interpolation_leaks(&config, dummy_path(), &[]).unwrap_err();
        match err {
            CompositionError::LifecycleInterpolationLeak { property, .. } => {
                assert!(
                    property.starts_with("start.stack"),
                    "expected stack property, got: {property}"
                );
            }
            other => panic!("expected LifecycleInterpolationLeak, got: {other:?}"),
        }
    }

    #[test]
    fn top_level_info_field_leak_is_caught() {
        // The `info` field is now covered by the leak scan (Phase 2 added
        // the field; Phase 3 extends the scan to cover it).
        let config = LifecycleConfig {
            start: Some(LifecycleNotification {
                info: Some("leaked {{ broken( }}".to_string()),
                ..Default::default()
            }),
            ..Default::default()
        };
        let err = validate_no_interpolation_leaks(&config, dummy_path(), &[]).unwrap_err();
        match err {
            CompositionError::LifecycleInterpolationLeak { property, .. } => {
                assert_eq!(property, "start.info");
            }
            other => panic!("expected leak, got: {other:?}"),
        }
    }

    #[test]
    fn top_level_warn_field_leak_is_caught() {
        let config = LifecycleConfig {
            start: Some(LifecycleNotification {
                warn: Some("leaked {{ broken( }}".to_string()),
                ..Default::default()
            }),
            ..Default::default()
        };
        let err = validate_no_interpolation_leaks(&config, dummy_path(), &[]).unwrap_err();
        assert!(matches!(
            err,
            CompositionError::LifecycleInterpolationLeak { property, .. } if property == "start.warn"
        ));
    }

    #[test]
    fn initialize_finalize_loop_top_level_leaks_are_caught() {
        // All seven events are now covered.
        for event in ["initialize", "finalize"] {
            let config = LifecycleConfig {
                initialize: if event == "initialize" {
                    Some(LifecycleNotification {
                        stderr: Some("leaked {{ broken( }}".to_string()),
                        ..Default::default()
                    })
                } else {
                    None
                },
                finalize: if event == "finalize" {
                    Some(LifecycleNotification {
                        stderr: Some("leaked {{ broken( }}".to_string()),
                        ..Default::default()
                    })
                } else {
                    None
                },
                ..Default::default()
            };
            let result = validate_no_interpolation_leaks(&config, dummy_path(), &[]);
            match result {
                Err(CompositionError::LifecycleInterpolationLeak { property, .. })
                    if property.starts_with(event) => {}
                other => panic!("expected leak for {event}, got: {other:?}"),
            }
        }
    }

    // -- stack undefined-variable scan -------------------------------------

    #[test]
    fn stack_undefined_variable_in_when_clause_is_rejected() {
        let fm = json!({
            "start": {
                "stack": [
                    {"when": "missing_var == 'x'", "action": {"say": "hi"}}
                ]
            }
        });
        let raw = fm_from_json(fm.clone());
        let effective = json!({});
        let lifecycle = parse_lifecycle_config(&fm, dummy_path()).unwrap();
        let err = validate_no_undefined_lifecycle_variables(&raw, &effective, &lifecycle, dummy_path())
            .unwrap_err();
        match err {
            CompositionError::LifecycleUndefinedVariable { property, variable, .. } => {
                assert!(property.contains("when"), "got: {property}");
                assert_eq!(variable, "missing_var");
            }
            other => panic!("expected undefined variable, got: {other:?}"),
        }
    }

    #[test]
    fn stack_err_global_is_not_undefined_in_failure() {
        // `err` is a lifecycle global in stack expressions, so it must not
        // trip the undefined-variable scan (the err static scan handles
        // misuse).
        let fm = json!({
            "failure": {
                "stack": [{"action": {"say": "err.msg"}}]
            }
        });
        let raw = fm_from_json(fm.clone());
        let effective = json!({});
        let lifecycle = parse_lifecycle_config(&fm, dummy_path()).unwrap();
        let result = validate_no_undefined_lifecycle_variables(&raw, &effective, &lifecycle, dummy_path());
        assert!(result.is_ok(), "err should not be undefined, got: {:?}", result.err());
    }

    #[test]
    fn stack_timing_and_current_globals_are_not_undefined() {
        let fm = json!({
            "start": {
                "stack": [
                    {"action": {"say": "timing.document_ms"}},
                    {"action": {"say": "current.ctx.agent"}}
                ]
            }
        });
        let raw = fm_from_json(fm.clone());
        let effective = json!({});
        let lifecycle = parse_lifecycle_config(&fm, dummy_path()).unwrap();
        let result = validate_no_undefined_lifecycle_variables(&raw, &effective, &lifecycle, dummy_path());
        assert!(result.is_ok(), "got: {:?}", result.err());
    }

    #[test]
    fn stack_bare_token_in_action_arg_is_literal_not_undefined_variable() {
        // A positional scalar value is literal text by default, so a bare token
        // is not an undefined-variable reference. Real references go through a
        // whole-value `{{ … }}` span.
        let fm = json!({
            "start": {
                "stack": [{"action": {"say": "missing_var"}}]
            }
        });
        let raw = fm_from_json(fm.clone());
        let effective = json!({});
        let lifecycle = parse_lifecycle_config(&fm, dummy_path()).unwrap();
        assert!(
            validate_no_undefined_lifecycle_variables(&raw, &effective, &lifecycle, dummy_path())
                .is_ok(),
            "a bare token in a literal message arg is not a variable reference"
        );
    }

    // -- lifecycle globals vs body/frontmatter interpolation --------------

    #[test]
    fn late_binding_global_in_top_level_field_is_a_known_root() {
        // Late binding (C4 / 5.3): `err`/`timing`/`current` are known roots in
        // top-level communication fields just like in stack surfaces — they
        // resolve at event-time, not against frontmatter — so the
        // undefined-variable scan does not flag a bare reference. (Placement
        // misuse — `err` in a no-error event — is caught separately by
        // `validate_no_err_in_no_error_events`.)
        for global in ["err", "timing", "current"] {
            let raw = fm_from_json(json!({
                "failure": { "message": format!("x: {{{{ {global} }}}}") }
            }));
            let effective = json!({});
            let result = validate_no_undefined_lifecycle_variables(
                &raw,
                &effective,
                &LifecycleConfig::default(),
                dummy_path(),
            );
            assert!(result.is_ok(), "`{global}` is a known root; got: {result:?}");
        }
    }

    #[test]
    fn bare_err_in_top_level_field_passes_when_frontmatter_defines_it() {
        // When frontmatter has a literal `err` property, `{{ err }}` in a
        // top-level field resolves to it — the lifecycle global does not
        // interfere.
        let raw = fm_from_json(json!({
            "start": { "message": "error: {{ err }}" }
        }));
        let effective = json!({ "err": "literal-value" });
        let result = validate_no_undefined_lifecycle_variables(
            &raw,
            &effective,
            &LifecycleConfig::default(),
            dummy_path(),
        );
        assert!(result.is_ok(), "got: {:?}", result.err());
    }

    // -- shell-audit collection -------------------------------------------

    #[test]
    fn collect_lifecycle_shell_commands_extracts_literal_commands() {
        let fm = json!({
            "start": {
                "stack": [
                    {"action": {"action": "shell", "command": "git fetch --all"}},
                    {"action": {"say": "not a shell command"}}
                ]
            },
            "failure": {
                "stack": [
                    {"action": {"action": "shell", "command": "git reset --hard", "on_error": "cleanup failed"}}
                ]
            }
        });
        let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
        let commands = collect_lifecycle_shell_commands(&config);
        let command_strings: Vec<&str> = commands.iter().map(|(c, _)| c.as_str()).collect();
        assert!(
            command_strings.contains(&"git fetch --all"),
            "expected git fetch, got: {command_strings:?}"
        );
        assert!(
            command_strings.contains(&"git reset --hard"),
            "expected git reset, got: {command_strings:?}"
        );
        assert!(
            command_strings.contains(&"cleanup failed"),
            "expected on_error command, got: {command_strings:?}"
        );
    }

    #[test]
    fn collect_lifecycle_shell_commands_empty_when_no_shells() {
        let fm = json!({
            "start": {
                "stack": [{"action": {"say": "hello"}}]
            }
        });
        let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
        let commands = collect_lifecycle_shell_commands(&config);
        assert!(commands.is_empty(), "got: {commands:?}");
    }

    #[test]
    fn collect_lifecycle_shell_commands_carries_property_path() {
        let fm = json!({
            "start": {
                "stack": [
                    {"action": {"action": "shell", "command": "echo hi"}}
                ]
            }
        });
        let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
        let commands = collect_lifecycle_shell_commands(&config);
        assert_eq!(commands.len(), 1);
        let (_, property) = &commands[0];
        assert!(
            property.contains("start.stack[0]") && property.contains(".command"),
            "expected property path, got: {property}"
        );
    }

    // -- no_error on every action category --------------------------------

    #[test]
    fn no_error_flag_is_accepted_on_every_action_category() {
        // The universal `no_error: true` flag must be accepted on every
        // action category: communication, shell, side-effect, and
        // expression-function.
        let fm = json!({
            "start": {
                "stack": [
                    {
                        "action": [
                            {"action": "say", "message": "hi", "no_error": true},
                            {"action": "shell", "command": "echo hi", "no_error": true},
                            {"action": "set_frontmatter", "file": "@a.md", "prop": "x", "value": "y", "no_error": true},
                            {"action": "length", "x": "hello", "no_error": true}
                        ]
                    }
                ]
            }
        });
        let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
        let stack = config.stack(LifecycleSignal::Start).expect("start stack");
        assert_eq!(stack[0].actions.len(), 4);
        for action in &stack[0].actions {
            assert!(action.no_error, "no_error should be true for {:?}", action.kind);
        }
    }

    #[test]
    fn no_error_on_scalar_form_threads_to_every_category() {
        // Scalar form: `no_error` is a sibling key alongside a bare-verb
        // zero-arg `action` value.
        let fm = json!({
            "start": {
                "stack": [
                    {
                        "action": "stop",
                        "no_error": true
                    }
                ]
            }
        });
        let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
        let stack = config.stack(LifecycleSignal::Start).expect("start stack");
        assert!(stack[0].actions[0].no_error);
    }

    #[test]
    fn no_error_defaults_to_false() {
        let fm = json!({
            "start": {
                "stack": [{"action": {"say": "hi"}}]
            }
        });
        let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
        let stack = config.stack(LifecycleSignal::Start).expect("start stack");
        assert!(!stack[0].actions[0].no_error);
    }

    // =====================================================================
    // Phase 5: runtime state machine
    // =====================================================================

    #[test]
    fn record_event_emission_tracks_state_and_prevents_double_emission() {
        let config = test_config();
        let (settings, messaging, term) = test_ctx();
        let emitter = RecordingEmitter::new();
        let ctx = LifecycleRuntimeContext {
            settings: &settings,
            messaging: &messaging,
            term: &term,
            source_path: dummy_path(),
            repo_root: None,
            launch_area: None,
        };
        let mut guard = LifecycleRunGuard::new(&config,
            &ctx,
            &emitter,
        );

        assert!(guard.record_event_emission(LifecycleSignal::Initialize));
        assert!(!guard.record_event_emission(LifecycleSignal::Initialize));

        assert!(guard.record_event_emission(LifecycleSignal::Start));
        assert!(!guard.record_event_emission(LifecycleSignal::Start));

        assert!(guard.record_event_emission(LifecycleSignal::Success));
        assert!(!guard.record_event_emission(LifecycleSignal::Success));
        assert!(!guard.record_event_emission(LifecycleSignal::Blocked));
        assert!(!guard.record_event_emission(LifecycleSignal::Failure));

        assert!(guard.record_event_emission(LifecycleSignal::Finalize));
        assert!(!guard.record_event_emission(LifecycleSignal::Finalize));
    }

    #[test]
    fn finalize_cannot_emit_without_terminal() {
        let config = test_config();
        let (settings, messaging, term) = test_ctx();
        let emitter = RecordingEmitter::new();
        let ctx = LifecycleRuntimeContext {
            settings: &settings,
            messaging: &messaging,
            term: &term,
            source_path: dummy_path(),
            repo_root: None,
            launch_area: None,
        };
        let mut guard = LifecycleRunGuard::new(&config,
            &ctx,
            &emitter,
        );
        assert!(!guard.record_event_emission(LifecycleSignal::Finalize));
    }

    /// Regression for the setup-stack failure path: `run_event_stack` records
    /// nothing, so running the `Failure` stack alone leaves `terminal_emitted`
    /// false and `Finalize` stays a no-op. Only `record_event_emission(Failure)`
    /// flips the flag so the subsequent `Finalize` fires. This is the
    /// bookkeeping invariant the `routes_to_failure` fix depends on.
    #[test]
    fn finalize_requires_recorded_terminal_not_just_stack_run() {
        let config = test_config();
        let (settings, messaging, term) = test_ctx();
        let emitter = RecordingEmitter::new();
        let ctx = LifecycleRuntimeContext {
            settings: &settings,
            messaging: &messaging,
            term: &term,
            source_path: dummy_path(),
            repo_root: None,
            launch_area: None,
        };

        // Running the failure stack via a context (without record) does not
        // touch the guard's terminal flag, so Finalize is still skipped.
        let mut guard = LifecycleRunGuard::new(&config, &ctx, &emitter);
        let stack_ctx = crate::composition::lifecycle_executor::StackExecutionContext {
            signal: LifecycleSignal::Failure,
            frontmatter: &serde_json::Map::new(),
            live_frontmatter: None,
            err: None,
            timing: None,
            current: None,
            base_dir: None,
            ctx_base_dir: None,
            effect_engine: &darkmatter::effects::EffectEngine::builder()
                .mutation_root(std::env::current_dir().unwrap())
                .auto_rehash(false)
                .build(),
            shell_runner: &crate::composition::lifecycle_executor::SystemShellRunner,
            emitter: &emitter,
            term: &term,
            source_path: dummy_path(),
            repo_root: None,
            messaging: &messaging,
            settings: &settings,
        };
        guard.run_event_stack(LifecycleSignal::Failure, &stack_ctx);
        assert!(
            !guard.record_event_emission(LifecycleSignal::Finalize),
            "Finalize must be a no-op when no terminal signal was recorded"
        );

        // Recording Failure first flips terminal_emitted, so Finalize fires.
        let mut guard = LifecycleRunGuard::new(&config, &ctx, &emitter);
        assert!(guard.record_event_emission(LifecycleSignal::Failure));
        assert!(
            guard.record_event_emission(LifecycleSignal::Finalize),
            "Finalize must fire once the terminal Failure signal is recorded"
        );
    }

    /// `redesignate_terminal_to_failure` overwrites a recorded `Success`/
    /// `Blocked` terminal slot with `Failure` while keeping `terminal_emitted`
    /// true — so a `success`/`blocked` stack's `error()` downgrade can run the
    /// `failure` event and still reach `finalize`. The success/blocked top-level
    /// emission stays fired (it happened before the stack), and re-designation
    /// is a no-op for any other slot.
    #[test]
    fn redesignate_terminal_to_failure_overwrites_success_keeps_finalize() {
        let config = test_config();
        let (settings, messaging, term) = test_ctx();
        let emitter = RecordingEmitter::new();
        let ctx = LifecycleRuntimeContext {
            settings: &settings,
            messaging: &messaging,
            term: &term,
            source_path: dummy_path(),
            repo_root: None,
            launch_area: None,
        };

        // Success slot → re-designate to Failure → finalize still fires.
        let mut guard = LifecycleRunGuard::new(&config, &ctx, &emitter);
        assert!(guard.record_event_emission(LifecycleSignal::Success));
        assert_eq!(guard.terminal_signal(), Some(LifecycleSignal::Success));
        assert!(guard.redesignate_terminal_to_failure());
        assert_eq!(guard.terminal_signal(), Some(LifecycleSignal::Failure));
        assert!(
            guard.record_event_emission(LifecycleSignal::Finalize),
            "finalize must still fire after a success→failure re-designation"
        );

        // Blocked slot re-designates too.
        let mut guard = LifecycleRunGuard::new(&config, &ctx, &emitter);
        assert!(guard.record_event_emission(LifecycleSignal::Blocked));
        assert!(guard.redesignate_terminal_to_failure());
        assert_eq!(guard.terminal_signal(), Some(LifecycleSignal::Failure));

        // No-op when the recorded slot is already Failure (or unset).
        let mut guard = LifecycleRunGuard::new(&config, &ctx, &emitter);
        assert!(!guard.redesignate_terminal_to_failure());
        assert!(guard.record_event_emission(LifecycleSignal::Failure));
        assert!(!guard.redesignate_terminal_to_failure());
        assert_eq!(guard.terminal_signal(), Some(LifecycleSignal::Failure));
    }

    #[test]
    fn run_event_stack_emits_top_level_and_stack() {
        let fm = json!({
            "start": {
                "stderr": "top-level",
                "stack": [{"action": {"stderr": "stack"}}]
            }
        });
        let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
        let (settings, messaging, term) = test_ctx();
        let emitter = RecordingEmitter::new();
        let ctx = LifecycleRuntimeContext {
            settings: &settings,
            messaging: &messaging,
            term: &term,
            source_path: dummy_path(),
            repo_root: None,
            launch_area: None,
        };
        let mut guard = LifecycleRunGuard::new(&config,
            &ctx,
            &emitter,
        );

        assert!(guard.record_event_emission(LifecycleSignal::Start));

        let stack_ctx = crate::composition::lifecycle_executor::StackExecutionContext {
            signal: LifecycleSignal::Start,
            frontmatter: &serde_json::Map::new(),
            live_frontmatter: None,
            err: None,
            timing: None,
            current: None,
            base_dir: None,
            ctx_base_dir: None,
            effect_engine: &darkmatter::effects::EffectEngine::builder()
                .mutation_root(std::env::current_dir().unwrap())
                .auto_rehash(false)
                .build(),
            shell_runner: &crate::composition::lifecycle_executor::SystemShellRunner,
            emitter: &emitter,
            term: &term,
            source_path: dummy_path(),
            repo_root: None,
            messaging: &messaging,
            settings: &settings,
        };
        let outcome = guard.run_event_stack(LifecycleSignal::Start, &stack_ctx);
        assert!(outcome.control.is_none());
        assert!(outcome.action_error.is_none());

        let stderr_signals: Vec<LifecycleSignal> = emitter
            .signals()
            .into_iter()
            .collect();
        assert_eq!(stderr_signals, vec![LifecycleSignal::Start, LifecycleSignal::Start]);
        let texts: Vec<String> = emitter
            .actions
            .lock()
            .unwrap()
            .iter()
            .filter_map(|a| match a {
                EmittedAction::Stderr { text, .. } => Some(text.clone()),
                _ => None,
            })
            .collect();
        assert_eq!(texts, vec!["top-level", "stack"]);
    }

    #[test]
    fn execute_event_still_runs_full_event() {
        let config = test_config();
        let (settings, messaging, term) = test_ctx();
        let emitter = RecordingEmitter::new();
        let ctx = LifecycleRuntimeContext {
            settings: &settings,
            messaging: &messaging,
            term: &term,
            source_path: dummy_path(),
            repo_root: None,
            launch_area: None,
        };
        let mut guard = LifecycleRunGuard::new(
            &config,
            &ctx,
            &emitter,
        );

        let stack_ctx = crate::composition::lifecycle_executor::StackExecutionContext {
            signal: LifecycleSignal::Start,
            frontmatter: &serde_json::Map::new(),
            live_frontmatter: None,
            err: None,
            timing: None,
            current: None,
            base_dir: None,
            ctx_base_dir: None,
            effect_engine: &darkmatter::effects::EffectEngine::builder()
                .mutation_root(std::env::current_dir().unwrap())
                .auto_rehash(false)
                .build(),
            shell_runner: &crate::composition::lifecycle_executor::SystemShellRunner,
            emitter: &emitter,
            term: &term,
            source_path: dummy_path(),
            repo_root: None,
            messaging: &messaging,
            settings: &settings,
        };
        let outcome = guard.execute_event(LifecycleSignal::Start, &stack_ctx);
        assert!(outcome.control.is_none());
        assert!(guard.start_emitted());
        assert_eq!(emitter.signals().len(), 1);
    }

    // -- action_value_to_expr -------------------------------------------------

    use darkmatter::markdown::compose::expression::{evaluate, EvaluationLookup};
    use serde_json::Value;
    use std::collections::HashMap;

    struct MapLookup(HashMap<String, Value>);

    impl EvaluationLookup for MapLookup {
        fn get(&self, path: &str) -> Option<Value> {
            self.0.get(path).cloned()
        }
    }

    struct EmptyLookup;

    impl EvaluationLookup for EmptyLookup {
        fn get(&self, _path: &str) -> Option<Value> {
            None
        }
    }

    #[test]
    fn action_value_to_expr_plain_literal() {
        let expr = action_value_to_expr(&json!("hello world")).unwrap();
        assert_eq!(expr, Expr::StringLiteral("hello world".into()));
    }

    #[test]
    fn action_value_to_expr_multi_span_interpolation_stays_literal() {
        let expr = action_value_to_expr(&json!("before {{ x }} after")).unwrap();
        assert_eq!(expr, Expr::StringLiteral("before {{ x }} after".into()));
    }

    #[test]
    fn action_value_to_expr_whole_value_bool() {
        let expr = action_value_to_expr(&json!("{{ true }}")).unwrap();
        assert_eq!(expr, Expr::BoolLiteral(true));
    }

    #[test]
    fn action_value_to_expr_whole_value_number() {
        let expr = action_value_to_expr(&json!("{{ 3 }}")).unwrap();
        assert_eq!(expr, Expr::NumberLiteral(3.0));
    }

    #[test]
    fn action_value_to_expr_whole_value_with_surrounding_whitespace() {
        let expr = action_value_to_expr(&json!("  {{ true }}  ")).unwrap();
        assert_eq!(expr, Expr::BoolLiteral(true));
    }

    #[test]
    fn action_value_to_expr_whole_value_null() {
        let expr = action_value_to_expr(&json!("{{ null }}")).unwrap();
        assert_eq!(evaluate(&expr, &EmptyLookup).unwrap(), Value::Null);
    }

    #[test]
    fn action_value_to_expr_whole_value_object_passthrough() {
        let payload = json!({ "status": "ready", "count": 7 });
        let lookup = MapLookup([("payload".to_string(), payload.clone())].into());
        let expr = action_value_to_expr(&json!("{{ payload }}")).unwrap();
        assert_eq!(evaluate(&expr, &lookup).unwrap(), payload);
    }

    #[test]
    fn action_value_to_expr_yaml_scalar_typing() {
        assert_eq!(
            action_value_to_expr(&json!(42)).unwrap(),
            Expr::NumberLiteral(42.0)
        );
        assert_eq!(
            action_value_to_expr(&json!(true)).unwrap(),
            Expr::BoolLiteral(true)
        );
    }

    #[test]
    fn action_value_to_expr_rejects_direct_object() {
        let err = action_value_to_expr(&json!({ "a": 1 })).unwrap_err();
        assert!(
            err.contains("object values are not supported"),
            "unexpected error: {err}"
        );
        assert!(
            err.contains("{{"),
            "error should mention whole-value interpolation: {err}"
        );
    }

    #[test]
    fn action_value_to_expr_rejects_direct_array() {
        let err = action_value_to_expr(&json!([1, 2, 3])).unwrap_err();
        assert!(
            err.contains("array values are not supported"),
            "unexpected error: {err}"
        );
        assert!(
            err.contains("{{"),
            "error should mention whole-value interpolation: {err}"
        );
    }
}
