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
    CommunicationAction, CommunicationChannel, ExpressionFunctionAction, LifecycleAction,
    LifecycleActionKind, LifecycleControlAction, LifecycleStackItem, RetryBackoff, ShellAction,
    SideEffectAction,
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

    // Validate effect name if present.
    if let Some(effect_name) = &notification.effect
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
///   action: <scalar string | array of (string | object)>
///   # When `action` is a scalar string, the remaining keys at this level
///   # are the action's long-form parameters (and the universal `no_error`
///   # flag). When `action` is an array, each element is self-contained
///   # (string short form or object long form).
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

    // Collect sibling keys (everything except `when` and `action`). These
    // apply only when `action` is a scalar string; for an array they are
    // ignored (each array element carries its own params).
    let mut sibling_params: Vec<(String, Expr)> = Vec::new();
    let mut stack_no_error: Option<bool> = None;
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
        let expr = value_to_expr(value).map_err(|message| {
            CompositionError::LifecycleStackInvalidShape {
                source_path: source_file.to_path_buf(),
                property: property_name.to_string(),
                message: format!("`{key}` is not a valid expression value: {message}"),
            }
        })?;
        sibling_params.push((key.clone(), expr));
    }

    let actions = match raw_action {
        // Scalar string action: short form (`verb(args)`) or bare verb with
        // sibling params.
        serde_json::Value::String(s) => {
            let no_error = stack_no_error.unwrap_or(false);
            let action =
                parse_scalar_action(signal, s, &sibling_params, no_error, source_file, property_name)?;
            vec![action]
        }
        // Array of actions, each self-contained.
        serde_json::Value::Array(items) => {
            if stack_no_error.is_some() || !sibling_params.is_empty() {
                return Err(CompositionError::LifecycleStackInvalidShape {
                    source_path: source_file.to_path_buf(),
                    property: property_name.to_string(),
                    message: "stack item with an array `action` cannot carry sibling parameters \
                        (move them into each array element)"
                        .to_string(),
                });
            }
            let mut actions = Vec::with_capacity(items.len());
            for item in items {
                let action = match item {
                    serde_json::Value::String(s) => {
                        parse_short_form_action(signal, s, source_file)?
                    }
                    serde_json::Value::Object(inner) => parse_long_form_action_object(
                        signal, inner, source_file, property_name,
                    )?,
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
        // Object form: long-form action presented as `{action: {verb: ...}}`.
        // The spec examples use siblings of `action:` rather than nesting,
        // but accept the nested form too as a tolerant extension.
        serde_json::Value::Object(_) => {
            return Err(CompositionError::LifecycleStackInvalidShape {
                source_path: source_file.to_path_buf(),
                property: property_name.to_string(),
                message: "`action` object form is not supported; use a scalar string verb with \
                    sibling parameter keys, or an array of actions"
                    .to_string(),
            });
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

/// Parse a scalar string `action` value, choosing short form or long form
/// based on whether the string contains parens.
fn parse_scalar_action(
    signal: LifecycleSignal,
    raw: &str,
    sibling_params: &[(String, Expr)],
    no_error: bool,
    source_file: &Path,
    property_name: &str,
) -> Result<LifecycleAction, CompositionError> {
    let trimmed = raw.trim();

    // Empty string is always an error.
    if trimmed.is_empty() {
        return Err(CompositionError::LifecycleActionInvalidShortForm {
            source_path: source_file.to_path_buf(),
            property: property_name.to_string(),
            raw: raw.to_string(),
            message: "action cannot be empty".to_string(),
        });
    }

    // If the string contains parens, treat as short form. Any sibling params
    // other than `no_error` are reported as unknown — short form carries its
    // args inside the parens.
    if trimmed.contains('(') {
        if !sibling_params.is_empty() {
            let mut keys: Vec<&str> = sibling_params.iter().map(|(k, _)| k.as_str()).collect();
            keys.sort_unstable();
            return Err(CompositionError::LifecycleActionInvalidLongForm {
                source_path: source_file.to_path_buf(),
                property: property_name.to_string(),
                action: trimmed.to_string(),
                message: format!(
                    "short-form action `{trimmed}` cannot take sibling parameters: {}",
                    keys.join(", ")
                ),
            });
        }
        let mut action = parse_short_form_action(signal, raw, source_file)?;
        action.no_error = no_error;
        return Ok(action);
    }

    // Otherwise, bare verb → long form using sibling params.
    let params: Vec<(String, Expr)> = sibling_params.to_vec();
    build_action_from_params(signal, trimmed, params, no_error, source_file)
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
        let expr = value_to_expr(value).map_err(|message| {
            CompositionError::LifecycleActionInvalidLongForm {
                source_path: source_file.to_path_buf(),
                property: property_name.to_string(),
                action: verb.clone(),
                message: format!("`{key}` is not a valid expression value: {message}"),
            }
        })?;
        params.push((key.clone(), expr));
    }

    build_action_from_params(signal, &verb, params, no_error, source_file)
}

/// Parse a short-form action: `verb(arg1, arg2, …)` or bare `verb`.
fn parse_short_form_action(
    signal: LifecycleSignal,
    raw: &str,
    source_file: &Path,
) -> Result<LifecycleAction, CompositionError> {
    let property = signal.property_name();
    let trimmed = raw.trim();

    if trimmed.is_empty() {
        return Err(CompositionError::LifecycleActionInvalidShortForm {
            source_path: source_file.to_path_buf(),
            property: property.to_string(),
            raw: raw.to_string(),
            message: "action cannot be empty".to_string(),
        });
    }

    let (verb, args_raw) = match trimmed.find('(') {
        Some(open) => {
            let close = trimmed.rfind(')').ok_or_else(|| {
                CompositionError::LifecycleActionInvalidShortForm {
                    source_path: source_file.to_path_buf(),
                    property: property.to_string(),
                    raw: raw.to_string(),
                    message: "missing closing `)`".to_string(),
                }
            })?;
            if close != trimmed.len() - 1 || open == 0 {
                return Err(CompositionError::LifecycleActionInvalidShortForm {
                    source_path: source_file.to_path_buf(),
                    property: property.to_string(),
                    raw: raw.to_string(),
                    message: "expected `verb(args)` form".to_string(),
                });
            }
            let verb = trimmed[..open].trim();
            let args_raw = &trimmed[open + 1..close];
            (verb, Some(args_raw))
        }
        None => (trimmed, None),
    };

    if verb.is_empty() {
        return Err(CompositionError::LifecycleActionInvalidShortForm {
            source_path: source_file.to_path_buf(),
            property: property.to_string(),
            raw: raw.to_string(),
            message: "missing verb before `(`".to_string(),
        });
    }

    let raw_args = match args_raw {
        Some(args_str) => super::loop_config::split_action_args(args_str).map_err(|e| match e {
            CompositionError::LoopInvalid(msg) => CompositionError::LifecycleActionInvalidShortForm {
                source_path: source_file.to_path_buf(),
                property: property.to_string(),
                raw: raw.to_string(),
                message: msg,
            },
            // split_action_args only returns LoopInvalid; route anything else
            // through the same diagnostic for a consistent surface.
            other => CompositionError::LifecycleActionInvalidShortForm {
                source_path: source_file.to_path_buf(),
                property: property.to_string(),
                raw: raw.to_string(),
                message: other.to_string(),
            },
        })?,
        None => Vec::new(),
    };

    let mut args = Vec::with_capacity(raw_args.len());
    for arg in raw_args {
        let expr = parse_action_arg(signal, &arg, raw, source_file)?;
        args.push(expr);
    }

    build_action(signal, verb, args, raw, /* no_error */ false, source_file)
}

/// Convert a raw JSON value into a Darkmatter `Expr`.
///
/// Long-form parameter values are stored as **string literals** when the
/// string cannot be parsed as a Darkmatter expression. This lets authors
/// write natural values like `command: "git push origin HEAD"` (a multi-word
/// shell command) or `file: "@spec.md"` (a file reference) without quoting
/// gymnastics. Values that *do* parse as expressions (e.g. `target: "ctx.repo"
/// ` or `max_attempts: 3`) preserve their parsed form for runtime evaluation
/// against the lifecycle context. Interpolation markers (`{{ … }}`) inside a
/// literal string are resolved at runtime by the same Darkmatter pipeline
/// that resolves top-level communication properties.
fn value_to_expr(value: &serde_json::Value) -> Result<Expr, String> {
    match value {
        serde_json::Value::String(s) => {
            if s.is_empty() {
                return Ok(Expr::StringLiteral(String::new()));
            }
            // Try plain-expression parse first; fall back to a literal string
            // so natural prose (commands, file refs, descriptions) survives.
            match parse(s) {
                Ok(expr) => Ok(expr),
                Err(_) => Ok(Expr::StringLiteral(s.clone())),
            }
        }
        serde_json::Value::Number(n) => {
            let f = n
                .as_f64()
                .ok_or_else(|| format!("unsupported number `{n}`"))?;
            Ok(Expr::NumberLiteral(f))
        }
        serde_json::Value::Bool(b) => Ok(Expr::BoolLiteral(*b)),
        other => Err(format!(
            "{} values are not supported as action parameters; use a scalar or string",
            json_type_name(other)
        )),
    }
}

/// Parse one short-form argument into an [`Expr`], rejecting unquoted
/// multi-word literals.
///
/// Per the spec, short-form arguments are Darkmatter expressions. A bare
/// unquoted multi-word string (e.g. `say(using codex)`) is invalid because
/// it parses as multiple tokens with no expression meaning. Detection: an
/// argument that fails to parse **and** contains whitespace outside any
/// quote or bracket region is the multi-word case; an argument that fails
/// to parse without such whitespace is a generic invalid-expression error.
fn parse_action_arg(
    signal: LifecycleSignal,
    arg: &str,
    raw_action: &str,
    source_file: &Path,
) -> Result<Expr, CompositionError> {
    let property = signal.property_name();
    let trimmed = arg.trim();
    if trimmed.is_empty() {
        return Err(CompositionError::LifecycleActionInvalidShortForm {
            source_path: source_file.to_path_buf(),
            property: property.to_string(),
            raw: raw_action.to_string(),
            message: "empty argument".to_string(),
        });
    }

    match parse(trimmed) {
        Ok(expr) => Ok(expr),
        Err(e) => {
            let message = if has_unquoted_whitespace(trimmed) {
                format!(
                    "argument `{trimmed}` looks like an unquoted multi-word literal; \
                     wrap multi-word strings in quotes (e.g. `'{trimmed}')")
            } else {
                format!("argument `{trimmed}` is not a valid expression: {e}")
            };
            Err(CompositionError::LifecycleActionInvalidShortForm {
                source_path: source_file.to_path_buf(),
                property: property.to_string(),
                raw: raw_action.to_string(),
                message,
            })
        }
    }
}

/// Returns `true` if `s` contains whitespace outside any quote or bracket
/// region. Used to detect the unquoted-multi-word-literal case.
fn has_unquoted_whitespace(s: &str) -> bool {
    let mut quote: Option<char> = None;
    let mut depth = 0i32;
    for ch in s.chars() {
        if let Some(qc) = quote {
            if ch == qc {
                quote = None;
            }
            continue;
        }
        match ch {
            '\'' | '"' => quote = Some(ch),
            '[' | '{' | '(' => depth += 1,
            ']' | '}' | ')' => depth -= 1,
            c if c.is_whitespace() && depth == 0 => return true,
            _ => {}
        }
    }
    false
}

/// Dispatch a parsed short-form action to the right builder. Placement is
/// NOT enforced here — the stack-item parser runs the cardinality check
/// first, then enforces placement for any lifecycle control actions.
fn build_action(
    signal: LifecycleSignal,
    verb: &str,
    args: Vec<Expr>,
    raw: &str,
    no_error: bool,
    source_file: &Path,
) -> Result<LifecycleAction, CompositionError> {
    // Lifecycle control actions.
    if let Some(control) = parse_lifecycle_control_short(verb, &args, signal, raw, source_file)? {
        return Ok(LifecycleAction {
            kind: LifecycleActionKind::LifecycleControl(control),
            no_error,
        });
    }

    // Communication actions.
    if let Some(channel) = CommunicationChannel::from_verb(verb) {
        let message = expect_one_arg(verb, &args, signal, source_file)?;
        return Ok(LifecycleAction {
            kind: LifecycleActionKind::Communication(CommunicationAction {
                channel,
                message,
                route: None,
            }),
            no_error,
        });
    }

    // Shell action: `shell(command)` accepts exactly one arg.
    if verb == "shell" {
        let command = expect_one_arg(verb, &args, signal, source_file)?;
        return Ok(LifecycleAction {
            kind: LifecycleActionKind::Shell(ShellAction {
                command,
                on_error: None,
            }),
            no_error,
        });
    }

    // Anything else: try side-effect verb or expression function.
    // Side-effect verbs are matched by the Darkmatter catalog; here we treat
    // any unknown verb as an expression function and let Phase 4 / runtime
    // decide whether it is actually a known side effect.
    Ok(LifecycleAction {
        kind: LifecycleActionKind::ExpressionFunction(ExpressionFunctionAction {
            function: verb.to_string(),
            args,
        }),
        no_error,
    })
}

/// Build a typed action from a long-form verb + named parameters.
///
/// Long-form allows per-verb named parameters that don't fit the short-form
/// `verb(args)` shape (e.g. `shell`'s `on_error`, `retry`'s `backoff`,
/// `set_frontmatter`'s `file`/`prop`/`value`).
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


    // Side-effect action: any remaining verb with named params. The
    // Darkmatter catalog is the authority. For a verb with a known signature
    // we reorder the named params into positional call order (and reject any
    // param the signature does not name) so the executor can dispatch
    // positionally. For an unknown verb we keep the params in alphabetical
    // key order for deterministic storage; the executor surfaces it as an
    // unknown side effect at runtime.
    if let Some(signature) = super::lifecycle_actions::side_effect_signature(verb) {
        let mut args = Vec::with_capacity(signature.len());
        for name in signature {
            if let Some(expr) = params_map.remove(*name) {
                args.push(expr);
            }
        }
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

/// Parse a short-form lifecycle control action (`stop`, `skip`, `retry(N)`,
/// etc.). Returns `Ok(None)` if the verb is not a control action.
fn parse_lifecycle_control_short(
    verb: &str,
    args: &[Expr],
    signal: LifecycleSignal,
    raw: &str,
    source_file: &Path,
) -> Result<Option<LifecycleControlAction>, CompositionError> {
    use LifecycleControlAction as A;

    let control = match verb {
        "stop" => {
            expect_no_args(verb, args, signal, source_file)?;
            A::Stop
        }
        "skip" => {
            expect_no_args(verb, args, signal, source_file)?;
            A::Skip
        }
        "error" => A::Error {
            reason: expect_optional_one_arg(verb, args, signal, source_file)?,
        },
        "proxy" => A::Proxy {
            target: expect_one_arg(verb, args, signal, source_file)?,
        },
        "retry" => A::Retry {
            max_attempts: expect_optional_one_arg(verb, args, signal, source_file)?,
            backoff: None,
            delay: None,
        },
        "resume" => A::Resume {
            message: expect_one_arg(verb, args, signal, source_file)?,
            max_attempts: None,
        },
        "requeue" => A::Requeue {
            delay: expect_one_arg(verb, args, signal, source_file)?,
            reason: None,
        },
        _ => return Ok(None),
    };
    let _ = raw;
    Ok(Some(control))
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
        "requeue" => A::Requeue {
            delay: params.remove("delay").ok_or_else(|| {
                invalid_args("`requeue` requires a `delay` parameter".to_string())
            })?,
            reason: params.remove("reason"),
        },
        _ => return Ok(None),
    };
    Ok(Some(control))
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

/// Expect exactly zero arguments.
fn expect_no_args(
    verb: &str,
    args: &[Expr],
    signal: LifecycleSignal,
    source_file: &Path,
) -> Result<(), CompositionError> {
    if !args.is_empty() {
        return Err(CompositionError::LifecycleInvalidArgs {
            source_path: source_file.to_path_buf(),
            property: signal.property_name().to_string(),
            action: verb.to_string(),
            message: format!("`{verb}` takes no arguments, got {}", args.len()),
        });
    }
    Ok(())
}

/// Expect exactly one argument and return it.
fn expect_one_arg(
    verb: &str,
    args: &[Expr],
    signal: LifecycleSignal,
    source_file: &Path,
) -> Result<Expr, CompositionError> {
    if args.len() != 1 {
        return Err(CompositionError::LifecycleInvalidArgs {
            source_path: source_file.to_path_buf(),
            property: signal.property_name().to_string(),
            action: verb.to_string(),
            message: format!("`{verb}` expects exactly 1 argument, got {}", args.len()),
        });
    }
    Ok(args[0].clone())
}

/// Expect zero or one argument and return it as `Option<Expr>`.
fn expect_optional_one_arg(
    verb: &str,
    args: &[Expr],
    signal: LifecycleSignal,
    source_file: &Path,
) -> Result<Option<Expr>, CompositionError> {
    match args.len() {
        0 => Ok(None),
        1 => Ok(Some(args[0].clone())),
        n => Err(CompositionError::LifecycleInvalidArgs {
            source_path: source_file.to_path_buf(),
            property: signal.property_name().to_string(),
            action: verb.to_string(),
            message: format!("`{verb}` expects 0 or 1 argument, got {n}"),
        }),
    }
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

        let fields: [(&str, Option<&String>); 9] = [
            ("say", notification.say.as_ref()),
            ("say_first", notification.say_first.as_ref()),
            ("message", notification.message.as_ref()),
            ("stderr", notification.stderr.as_ref()),
            ("notify", notification.notify.as_ref()),
            ("info", notification.info.as_ref()),
            ("warn", notification.warn.as_ref()),
            ("success", notification.success.as_ref()),
            ("stdout", notification.stdout.as_ref()),
        ];

        for (field_name, value) in fields {
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
            LifecycleControlAction::Requeue { delay, reason } => {
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
/// Used for top-level communication fields (post-composition leak scan):
/// lifecycle globals are **not** exempt here, so a bare `err`/`timing`/
/// `current` in a top-level field must resolve against frontmatter (the
/// body-interpolation contract).
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
        Expr::FunctionCall { args, .. } => args
            .iter()
            .find_map(|arg| find_undefined_stack_variable(arg, defined)),
    }
}

/// Returns the bare variable name when `path` is a frontmatter-scoped reference
/// whose root segment is absent from the composed frontmatter, or `None` when
/// it resolves elsewhere (`ctx.*` / `env.*` / `doc`) or its root key exists.
///
/// Nested misses (`{{ a.b }}` where `a` exists but `b` does not) are treated as
/// defined: only the bare-root contract the spec describes is enforced.
fn undefined_bare_variable<'a>(
    path: &'a str,
    defined: Option<&serde_json::Map<String, serde_json::Value>>,
) -> Option<&'a str> {
    if path.starts_with("ctx.")
        || path.starts_with("env.")
        || path == "doc"
        || path.starts_with("doc.")
    {
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

/// Like [`undefined_bare_variable`] but also exempts the lifecycle globals
/// (`err`, `timing`, `current`) — those are always defined inside stack
/// expression surfaces even when their value may be `null`. The
/// [`validate_no_err_in_no_error_events`] scan is responsible for catching
/// bare `err` misuse in no-error events.
fn undefined_stack_variable<'a>(
    path: &'a str,
    defined: Option<&serde_json::Map<String, serde_json::Value>>,
) -> Option<&'a str> {
    if path.starts_with("ctx.")
        || path.starts_with("env.")
        || path == "doc"
        || path.starts_with("doc.")
        || path == "err"
        || path.starts_with("err.")
        || path == "timing"
        || path.starts_with("timing.")
        || path == "current"
        || path.starts_with("current.")
    {
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
/// Walks every stack expression surface (`when:` clauses, action arguments,
/// communication messages, shell commands, side-effect args, control-action
/// operands) in events that cannot carry an error and rejects the first
/// bare `err` reference with [`CompositionError::LifecycleErrNotAvailable`].
///
/// Top-level communication fields (`say`/`message`/`stderr`/…) are **not**
/// scanned here. They go through Darkmatter composition, where `err`
/// resolves against frontmatter (a literal `err:` property) rather than
/// the lifecycle global. The undefined-variable scan handles misses there.
pub fn validate_no_err_in_no_error_events(
    lifecycle: &LifecycleConfig,
    source_path: &Path,
) -> Result<(), CompositionError> {
    for surface in iter_stack_expression_surfaces(lifecycle) {
        if surface.signal.can_carry_error() {
            continue;
        }
        if references_bare_err(surface.expr) {
            return Err(CompositionError::LifecycleErrNotAvailable {
                source_path: source_path.to_path_buf(),
                property: surface.property,
                event: surface.signal.property_name().to_string(),
            });
        }
    }
    Ok(())
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
/// Returns `(Some("X"), vec!["A", "B", "C"])` on match, or
/// `(None, LIFECYCLE_NOTIFICATION_FIELDS)` as a fallback.
fn parse_serde_unknown_field(err: &serde_json::Error) -> (Option<String>, Vec<String>) {
    let msg = err.to_string();

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
    fn parses_short_form_say_action() {
        let fm = json!({
            "start": {
                "stack": [
                    {"action": "say('hello world')"}
                ]
            }
        });
        let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
        let stack = config.stack(LifecycleSignal::Start).expect("start stack");
        assert_eq!(stack.len(), 1);
        let item = &stack[0];
        assert!(item.when.is_none());
        assert_eq!(item.actions.len(), 1);
        assert!(!item.actions[0].is_lifecycle_control());
    }

    #[test]
    fn parses_short_form_say_with_expression_arg() {
        let fm = json!({
            "start": {
                "stack": [{"action": "say(ctx.repo)"}]
            }
        });
        let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
        let _ = config.stack(LifecycleSignal::Start).expect("start stack");
    }

    #[test]
    fn parses_when_condition_with_stack() {
        let fm = json!({
            "start": {
                "stack": [
                    {
                        "when": "env.AGENT == 'claude'",
                        "action": "say('using claude')"
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
                            "say('first')",
                            "info('second')"
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
                "stack": [{"action": "retry(3)"}]
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
                "stack": [{"action": "proxy('@fallback.md')"}]
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
        // Long-form shell action: `command`, `on_error`, `no_error` are
        // sibling keys of the bare-verb `action: shell`.
        let fm = json!({
            "start": {
                "stack": [
                    {
                        "action": "shell",
                        "command": "git fetch --all",
                        "on_error": "fetch failed",
                        "no_error": true
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
        // Side-effect long form: `file`, `prop`, `value` are sibling keys
        // of the bare-verb `action: set_frontmatter`.
        let fm = json!({
            "start": {
                "stack": [
                    {
                        "action": "set_frontmatter",
                        "file": "@spec.md",
                        "prop": "status",
                        "value": "in-progress"
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
                "stack": [{"action": "ensure_file('@out/log.md')"}]
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
    fn rejects_proxy_in_start() {
        let fm = json!({
            "start": {"stack": [{"action": "proxy('@other.md')"}]}
        });
        let err = parse_lifecycle_config(&fm, dummy_path()).unwrap_err();
        assert!(matches!(
            err,
            CompositionError::LifecycleActionPlacement { .. }
        ));
    }

    #[test]
    fn rejects_retry_in_start() {
        let fm = json!({
            "start": {"stack": [{"action": "retry"}]}
        });
        let err = parse_lifecycle_config(&fm, dummy_path()).unwrap_err();
        assert!(matches!(
            err,
            CompositionError::LifecycleActionPlacement { .. }
        ));
    }

    #[test]
    fn rejects_resume_outside_failure() {
        let fm = json!({
            "blocked": {"stack": [{"action": "resume('please')"}]}
        });
        let err = parse_lifecycle_config(&fm, dummy_path()).unwrap_err();
        assert!(matches!(
            err,
            CompositionError::LifecycleActionPlacement { .. }
        ));
    }

    #[test]
    fn rejects_requeue_in_loop() {
        let fm = json!({
            "loop": {
                "while": "true",
                "stack": [{"action": "requeue('5m')"}]
            }
        });
        let err = parse_lifecycle_config(&fm, dummy_path()).unwrap_err();
        assert!(matches!(
            err,
            CompositionError::LifecycleActionPlacement { .. }
        ));
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
                    {"action": ["stop", "say('unreachable')"]}
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
                    {"action": ["say('one')", "stop"]}
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
    fn rejects_unquoted_multi_word_literal_in_short_form() {
        let fm = json!({
            "start": {
                "stack": [{"action": "say(using codex)"}]
            }
        });
        let err = parse_lifecycle_config(&fm, dummy_path()).unwrap_err();
        match err {
            CompositionError::LifecycleActionInvalidShortForm { message, .. } => {
                assert!(
                    message.contains("unquoted multi-word"),
                    "expected multi-word hint, got: {message}"
                );
            }
            other => panic!("expected LifecycleActionInvalidShortForm, got: {other:?}"),
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
            CompositionError::LifecycleActionInvalidShortForm { .. }
        ));
    }

    #[test]
    fn rejects_retry_with_too_many_args() {
        let fm = json!({
            "blocked": {
                "stack": [{"action": "retry(3, 4)"}]
            }
        });
        let err = parse_lifecycle_config(&fm, dummy_path()).unwrap_err();
        assert!(matches!(
            err,
            CompositionError::LifecycleInvalidArgs { .. }
        ));
    }

    #[test]
    fn rejects_proxy_missing_target() {
        // `proxy` requires a `target` parameter; without one, the long-form
        // builder fails with `LifecycleInvalidArgs`.
        let fm = json!({
            "initialize": {
                "stack": [{"action": "proxy"}]
            }
        });
        let err = parse_lifecycle_config(&fm, dummy_path()).unwrap_err();
        assert!(matches!(
            err,
            CompositionError::LifecycleInvalidArgs { .. }
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
        // With the long-form-via-sibling-params design, a `bogus` key on a
        // stack item is treated as a sibling parameter for the bare-verb
        // action. `stop` takes no parameters, so the long-form builder
        // rejects `bogus` as an unknown parameter.
        let fm = json!({
            "start": {
                "stack": [{"action": "stop", "bogus": true}]
            }
        });
        let err = parse_lifecycle_config(&fm, dummy_path()).unwrap_err();
        assert!(matches!(
            err,
            CompositionError::LifecycleActionInvalidLongForm { .. }
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
                "stack": [{"action": "stdout('hello')"}]
            }
        });
        // `stdout(...)` is now a recognized communication action; parsing
        // succeeds and produces a single-item stack.
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
        // `Skip` is invalid in the `loop` event per the "Where valid" matrix
        // because loop is a no-error event.
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
                    {"when": "err != null", "action": "say('has error')"}
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
    fn err_member_access_in_start_stack_is_rejected() {
        // `err.msg` is still a bare `err` reference.
        let fm = json!({
            "start": {
                "stack": [{"action": "say(err.msg)"}]
            }
        });
        let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
        let err = validate_no_err_in_no_error_events(&config, dummy_path()).unwrap_err();
        assert!(matches!(
            err,
            CompositionError::LifecycleErrNotAvailable { event, .. } if event == "start"
        ));
    }

    #[test]
    fn err_in_initialize_loop_success_is_rejected() {
        for ev in ["initialize", "success"] {
            let fm = json!({
                ev: {"stack": [{"action": "say(err)"}]}
            });
            let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
            let err = validate_no_err_in_no_error_events(&config, dummy_path()).unwrap_err();
            let returned_event = match err {
                CompositionError::LifecycleErrNotAvailable { event, .. } => event,
                other => panic!("expected err rejection for {ev}, got: {other:?}"),
            };
            assert_eq!(returned_event, ev);
        }
        // Loop concerns live under `loop:`.
        let fm = json!({
            "loop": {
                "while": "true",
                "stack": [{"action": "say(err)"}]
            }
        });
        let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
        let err = validate_no_err_in_no_error_events(&config, dummy_path()).unwrap_err();
        match err {
            CompositionError::LifecycleErrNotAvailable { event, .. } => {
                assert_eq!(event, "loop");
            }
            other => panic!("expected err rejection for loop, got: {other:?}"),
        }
    }

    #[test]
    fn err_in_blocked_failure_finalize_is_allowed() {
        // `err` is permitted in error-carrying events.
        for event in ["blocked", "failure", "finalize"] {
            let fm = json!({
                event: {"stack": [{"action": "say(err.msg)"}]}
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
                        "stack": [{"action": "say(doc.err)"}]
                    }
                })
            } else {
                json!({
                    event: {"stack": [{"action": "say(doc.err)"}]}
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
    fn err_in_control_action_reason_in_start_is_rejected() {
        // `error(err.msg)` in `start` references the `err` global.
        let fm = json!({
            "start": {
                "stack": [{"action": "error(err.msg)"}]
            }
        });
        let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
        let err = validate_no_err_in_no_error_events(&config, dummy_path()).unwrap_err();
        assert!(matches!(
            err,
            CompositionError::LifecycleErrNotAvailable { event, .. } if event == "start"
        ));
    }

    #[test]
    fn err_in_shell_command_in_loop_is_rejected() {
        // A shell command whose `command` expression references the `err`
        // global in the `loop` event (a no-error event).
        let fm = json!({
            "loop": {
                "while": "true",
                "stack": [{"action": "shell(err.msg)"}]
            }
        });
        let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
        let err = validate_no_err_in_no_error_events(&config, dummy_path()).unwrap_err();
        match err {
            CompositionError::LifecycleErrNotAvailable { event, .. } => {
                assert_eq!(event, "loop");
            }
            other => panic!("expected err rejection, got: {other:?}"),
        }
    }

    // -- stack leak scan ---------------------------------------------------

    #[test]
    fn stack_string_literal_with_interpolation_span_is_leak() {
        // A string literal inside a parsed expression that contains a
        // surviving `{{ … }}` span is a leak — the literal is passed through
        // verbatim to the evaluated result.
        let fm = json!({
            "start": {
                "stack": [{"action": "say('leaked {{ broken( }}')"}]
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
                    {"when": "missing_var == 'x'", "action": "say('hi')"}
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
                "stack": [{"action": "say(err.msg)"}]
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
                    {"action": "say(timing.document_ms)"},
                    {"action": "say(current.ctx.agent)"}
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
    fn stack_undefined_bare_variable_in_action_arg_is_rejected() {
        let fm = json!({
            "start": {
                "stack": [{"action": "say(missing_var)"}]
            }
        });
        let raw = fm_from_json(fm.clone());
        let effective = json!({});
        let lifecycle = parse_lifecycle_config(&fm, dummy_path()).unwrap();
        let err = validate_no_undefined_lifecycle_variables(&raw, &effective, &lifecycle, dummy_path())
            .unwrap_err();
        match err {
            CompositionError::LifecycleUndefinedVariable { variable, .. } => {
                assert_eq!(variable, "missing_var");
            }
            other => panic!("expected undefined variable, got: {other:?}"),
        }
    }

    // -- lifecycle globals vs body/frontmatter interpolation --------------

    #[test]
    fn bare_err_in_top_level_field_is_not_exempt() {
        // A bare `err` in a top-level communication field is NOT the
        // lifecycle global — it resolves against frontmatter like any
        // ordinary identifier. So an undefined `err` property is caught.
        let raw = fm_from_json(json!({
            "start": { "message": "error: {{ err }}" }
        }));
        let effective = json!({});
        let result = validate_no_undefined_lifecycle_variables(
            &raw,
            &effective,
            &LifecycleConfig::default(),
            dummy_path(),
        );
        match result {
            Err(CompositionError::LifecycleUndefinedVariable { variable, .. })
                if variable == "err" => {}
            other => panic!("expected undefined err, got: {other:?}"),
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
                    {"action": "shell", "command": "git fetch --all"},
                    {"action": "say('not a shell command')"}
                ]
            },
            "failure": {
                "stack": [
                    {"action": "shell", "command": "git reset --hard", "on_error": "cleanup failed"}
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
                "stack": [{"action": "say('hello')"}]
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
                    {"action": "shell", "command": "echo hi"}
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
        // expression-function (long-form arrays carry `no_error` per
        // element; scalar form carries it as a sibling key).
        let fm = json!({
            "start": {
                "stack": [
                    {
                        "action": [
                            {"action": "say('hi')", "no_error": true},
                            {"action": "shell", "command": "echo hi", "no_error": true},
                            {"action": "set_frontmatter", "file": "@a.md", "prop": "x", "value": "y", "no_error": true},
                            {"action": "length('hello')", "no_error": true}
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
        // Scalar form: `no_error` is a sibling key alongside `action: <verb>`.
        let fm = json!({
            "start": {
                "stack": [
                    {
                        "action": "say('hi')",
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
                "stack": [{"action": "say('hi')"}]
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
        };

        // Running the failure stack via a context (without record) does not
        // touch the guard's terminal flag, so Finalize is still skipped.
        let mut guard = LifecycleRunGuard::new(&config, &ctx, &emitter);
        let stack_ctx = crate::composition::lifecycle_executor::StackExecutionContext {
            signal: LifecycleSignal::Failure,
            frontmatter: &serde_json::Map::new(),
            err: None,
            timing: None,
            current: None,
            base_dir: None,
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
                "stack": [{"action": "stderr('stack')"}]
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
        };
        let mut guard = LifecycleRunGuard::new(&config,
            &ctx,
            &emitter,
        );

        assert!(guard.record_event_emission(LifecycleSignal::Start));

        let stack_ctx = crate::composition::lifecycle_executor::StackExecutionContext {
            signal: LifecycleSignal::Start,
            frontmatter: &serde_json::Map::new(),
            err: None,
            timing: None,
            current: None,
            base_dir: None,
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
        };
        let mut guard = LifecycleRunGuard::new(
            &config,
            &ctx,
            &emitter,
        );

        let stack_ctx = crate::composition::lifecycle_executor::StackExecutionContext {
            signal: LifecycleSignal::Start,
            frontmatter: &serde_json::Map::new(),
            err: None,
            timing: None,
            current: None,
            base_dir: None,
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
}
