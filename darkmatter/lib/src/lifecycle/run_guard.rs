//! The run guard: one lifecycle run's event accounting.
//!
//! [`LifecycleRunGuard`] owns the order `initialize` → `start` → one terminal
//! event (`success`, `blocked`, `failure`, or `cancelled`) → `finalize`, the
//! single terminal slot, whether the provider launched, and when shells are
//! allowed. The host supplies what the guard cannot know through
//! [`LifecycleRunHost`], and each event's executor configuration through
//! [`GuardedEventStack`].
//!
//! ```mermaid
//! stateDiagram-v2
//!     [*] --> Initialized: initialize
//!     Initialized --> Started: start (shells unlock)
//!     Started --> Terminal: success / blocked / failure / cancelled
//!     Terminal --> Finalized: finalize
//!     Started --> Terminal: drop safety net (blocked or failure)
//!     Finalized --> Started: reset_for_next_iteration
//!     Finalized --> [*]
//! ```

use std::borrow::Cow;
use std::path::Path;

use crate::markdown::compose::ComposeContext;
use crate::markdown::compose::expression::{Expr, ExpressionFinder};

use super::actions::{CommunicationChannel, LifecycleAction, LifecycleActionKind, LifecycleNode};
use super::cancellation::{self, CleanupScope};
use super::context::LifecycleErrorInfo;
use super::effects::{LifecycleEffects, dispatch_communication};
use super::envelope::DiagnosticCatalog;
use super::executor::{DisabledShellRunner, LifecycleEventOutcome, LifecycleRunFailure, ShellRunner, StackControl};
use super::runtime::EvaluationMode;
use super::{LifecycleConfig, LifecycleSignal};

/// What the run guard asks the application that embeds it.
///
/// One host serves one run. Its services are required: the guard cannot emit
/// a safety-net signal or classify a cleanup failure without them.
pub trait LifecycleRunHost {
    /// Held while a terminal event or `finalize` runs, and dropped when it
    /// returns; a host that treats a repeated interrupt differently during
    /// terminal cleanup observes it here.
    type TerminalScope;

    /// The document that owns the lifecycle, for typed diagnostics.
    fn source_path(&self) -> &Path;

    /// The early-binding context snapshot captured when the run began, if
    /// any; see [`LifecycleRunGuard::effective_prepared_context`].
    fn prepared_context(&self) -> Option<&ComposeContext>;

    /// The run's communication service.
    fn effects(&self) -> &dyn LifecycleEffects;

    /// The run's diagnostic catalog.
    fn diagnostics(&self) -> &dyn DiagnosticCatalog;

    /// Whether the user interrupted the run.
    ///
    /// Once it has, the guard's context-free emission keeps only `stderr`
    /// lines: the user still sees the terminal status, but no message,
    /// notification, speech, or sound effect is published while the process
    /// exits.
    fn interrupted(&self) -> bool;

    /// Enter a terminal event or `finalize`.
    fn enter_terminal_event(&self) -> Self::TerminalScope;

    /// Classify a lifecycle failure in the host's catalog.
    fn failure(&self, failure: LifecycleRunFailure) -> LifecycleErrorInfo;
}

/// One event the guard asks a [`GuardedEventStack`] to run.
#[derive(Clone, Copy)]
pub struct GuardedEvent<'e> {
    /// The event to run.
    pub signal: LifecycleSignal,
    /// The lifecycle the guard is driving.
    pub config: &'e LifecycleConfig,
    /// Replaces the configuration's shell runner when `Some`: the guard passes
    /// [`DisabledShellRunner`] until `start` has fired.
    pub shell_runner: Option<&'e dyn ShellRunner>,
    /// Replaces the `err` global when `Some`.
    pub error: Option<&'e LifecycleErrorInfo>,
}

/// A host's per-event executor configuration, as the guard runs it.
pub trait GuardedEventStack {
    /// Run `event.signal`'s stack from `event.config`.
    ///
    /// The implementation derives the executor for `event.signal` from this
    /// configuration. Another signal is another event, so a derived executor
    /// for a different signal must observe `current` afresh rather than reuse
    /// this configuration's observation.
    fn run_guarded(&self, event: GuardedEvent<'_>) -> LifecycleEventOutcome;
}

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
/// Shells are refused until `start` has fired: `start` follows the host's
/// successful preflight, so `initialize` and any catch event reached before it
/// run with [`DisabledShellRunner`]. [`Self::reset_for_proxy`] relocks them for
/// the adopted document.
///
/// ## Drop behavior
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
pub struct LifecycleRunGuard<'a, H: LifecycleRunHost> {
    // `Cow` so a `proxy` hand-off can repoint the guard at the target
    // document's lifecycle (an owned, re-parsed config) without the borrowed
    // original's lifetime. The common case stays a zero-cost borrow.
    config: Cow<'a, LifecycleConfig>,
    host: H,
    accounting: RunAccounting,
    // Set once a `proxy` hand-off adopts a target document. The host's
    // run-start snapshot was captured demand-driven for the *original*
    // document, so it omits groups only the proxied target references — it
    // must be dropped for the target's events.
    proxied: bool,
    // The current run's early-binding context, when the run is not the one the
    // guard was built for: a proxied target's rebuilt context, or a retried or
    // resumed attempt's fresh capture. Owned by the guard so it can outlive
    // the transient borrows that build it, and preferred over both the host's
    // snapshot and the demand-driven fallback.
    run_prepared_context: Option<ComposeContext>,
}

/// One run's event accounting: which slots have fired, the one terminal slot,
/// whether the provider launched, and so whether shells may run.
///
/// [`LifecycleRunGuard`] (direct calls) and [`super::run::Run`]
/// (request/reply) both keep their accounting here, so the two drivers share
/// one set of transitions.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RunAccounting {
    initialize_emitted: bool,
    start_emitted: bool,
    provider_launched: bool,
    terminal_emitted: bool,
    finalize_emitted: bool,
    terminal_signal: Option<LifecycleSignal>,
}

impl RunAccounting {
    /// Record that `signal` is about to run and return whether it may.
    ///
    /// `initialize` and `start` run once; the terminal events share one slot,
    /// the first wins; `finalize` runs once and only after the slot is taken;
    /// `loop` always runs.
    pub fn record_event_emission(&mut self, signal: LifecycleSignal) -> bool {
        match signal {
            LifecycleSignal::Initialize => !std::mem::replace(&mut self.initialize_emitted, true),
            LifecycleSignal::Start => !std::mem::replace(&mut self.start_emitted, true),
            LifecycleSignal::Success
            | LifecycleSignal::Blocked
            | LifecycleSignal::Failure
            | LifecycleSignal::Cancelled => {
                if self.terminal_emitted {
                    return false;
                }
                self.terminal_emitted = true;
                self.terminal_signal = Some(signal);
                true
            }
            LifecycleSignal::Finalize => {
                if self.finalize_emitted || !self.terminal_emitted {
                    return false;
                }
                self.finalize_emitted = true;
                true
            }
            LifecycleSignal::Loop => true,
        }
    }

    /// Re-designate a recorded `success` or `blocked` to `failure`, keeping
    /// the slot taken; `false` (and no change) for any other slot.
    pub fn redesignate_terminal_to_failure(&mut self) -> bool {
        if !matches!(self.terminal_signal, Some(LifecycleSignal::Success | LifecycleSignal::Blocked)) {
            return false;
        }
        self.terminal_signal = Some(LifecycleSignal::Failure);
        true
    }

    /// Record that the provider launched.
    pub fn mark_provider_launched(&mut self) {
        self.provider_launched = true;
    }

    /// `failure` once the provider launched, `blocked` before.
    pub fn blocked_or_failure(&self) -> LifecycleSignal {
        if self.provider_launched { LifecycleSignal::Failure } else { LifecycleSignal::Blocked }
    }

    /// The terminal event a run that ends now still owes: `start` fired and
    /// nothing took the slot.
    pub fn owed_terminal(&self) -> Option<LifecycleSignal> {
        (self.start_emitted && !self.terminal_emitted).then(|| self.blocked_or_failure())
    }

    /// Close the terminal slot without a terminal event, so `finalize` may
    /// still run once.
    pub fn defuse(&mut self) {
        self.terminal_emitted = true;
    }

    /// Clear everything but `initialize` for the next attempt or iteration;
    /// shells lock until its `start`.
    pub fn reset_for_next_iteration(&mut self) {
        *self = Self { initialize_emitted: self.initialize_emitted, ..Self::default() };
    }

    /// Clear everything, `initialize` included, for an adopted document.
    pub fn reset_for_proxy(&mut self) {
        *self = Self::default();
    }

    #[allow(missing_docs)]
    pub fn initialize_emitted(&self) -> bool {
        self.initialize_emitted
    }

    /// Whether `start` has fired, which is also whether shells may run.
    pub fn start_emitted(&self) -> bool {
        self.start_emitted
    }

    #[allow(missing_docs)]
    pub fn provider_launched(&self) -> bool {
        self.provider_launched
    }

    /// Whether the terminal slot is taken (or closed by [`Self::defuse`]).
    pub fn terminal_emitted(&self) -> bool {
        self.terminal_emitted
    }

    /// The event holding the terminal slot, after any redesignation.
    pub fn terminal_signal(&self) -> Option<LifecycleSignal> {
        self.terminal_signal
    }

    #[allow(missing_docs)]
    pub fn finalize_emitted(&self) -> bool {
        self.finalize_emitted
    }
}

impl<'a, H: LifecycleRunHost> LifecycleRunGuard<'a, H> {
    /// A guard for a run of `config`, before `initialize`.
    pub fn with_host(config: &'a LifecycleConfig, host: H) -> Self {
        Self {
            config: Cow::Borrowed(config),
            host,
            accounting: RunAccounting::default(),
            proxied: false,
            run_prepared_context: None,
        }
    }

    /// The host this guard reports to.
    pub fn host(&self) -> &H {
        &self.host
    }

    /// Source owning this lifecycle, retained for typed runtime diagnostics.
    pub fn source_path(&self) -> &Path {
        self.host.source_path()
    }

    /// Emit the `Initialize` signal (idempotent).
    pub fn emit_initialize_once(&mut self) {
        if self.accounting.record_event_emission(LifecycleSignal::Initialize) {
            self.emit_signal(LifecycleSignal::Initialize);
        }
    }

    /// Emit the `Start` signal (idempotent — only emits on the first call).
    pub fn emit_start_once(&mut self) {
        if self.accounting.record_event_emission(LifecycleSignal::Start) {
            self.emit_signal(LifecycleSignal::Start);
        }
    }

    /// Record that the provider has actually launched.
    ///
    /// Call this once the launch succeeded, not before: it decides whether a
    /// later safety net or [`Self::emit_blocked_or_failure`] reports
    /// `failure` rather than `blocked`.
    pub fn mark_provider_launched(&mut self) {
        self.accounting.mark_provider_launched();
    }

    /// Emit a specific terminal signal and suppress the Drop safety-net.
    ///
    /// Terminal signals (`Success`, `Blocked`, `Failure`) are mutually
    /// exclusive; the first one wins.
    pub fn emit_terminal(&mut self, signal: LifecycleSignal) {
        if self.accounting.terminal_emitted() {
            return;
        }
        self.emit_signal(signal);
        self.accounting.record_event_emission(signal);
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
        self.emit_terminal(self.accounting.blocked_or_failure());
    }

    /// Record that `signal` is about to be emitted and return whether the
    /// caller should actually run the event.
    ///
    /// This is split from [`Self::run_event_stack`] so a host can build an
    /// event configuration that borrows from the guard without conflicting
    /// with the mutable state update.
    pub fn record_event_emission(&mut self, signal: LifecycleSignal) -> bool {
        self.accounting.record_event_emission(signal)
    }

    /// Re-designate an already-recorded `Success`/`Blocked` terminal signal to
    /// `Failure` so a routed-to `failure` event (and the subsequent `finalize`)
    /// can still fire.
    ///
    /// A `success`/`blocked` stack that ends in an explicit `error(...)` action
    /// downgrades the run to `failure`. The actions before the directive in the
    /// success/blocked stack have **already** run and their emissions must
    /// remain — so the terminal slot was already taken by `Success`/`Blocked`.
    /// This overwrites only the recorded terminal signal while the slot stays
    /// taken, letting the caller run the `failure` event's stack directly
    /// (without [`Self::record_event_emission`], which would refuse the taken
    /// slot) and keeping [`Self::emit_finalize_once`] enabled.
    ///
    /// No-op (returns `false`) unless the currently recorded terminal signal is
    /// `Success` or `Blocked`; the existing slot is otherwise authoritative.
    pub fn redesignate_terminal_to_failure(&mut self) -> bool {
        self.accounting.redesignate_terminal_to_failure()
    }

    /// Run the typed stack for `signal` through `stack`.
    ///
    /// Callers must first call [`Self::record_event_emission`] and only run
    /// the stack when it returns `true`.
    pub fn run_event_stack<S: GuardedEventStack + ?Sized>(
        &self,
        signal: LifecycleSignal,
        stack: &S,
    ) -> LifecycleEventOutcome {
        self.run_guarded(signal, stack, None)
    }

    /// Execute the lifecycle event's stack for `signal`, tracking the guard's
    /// state machine.
    ///
    /// Convenience wrapper around [`Self::record_event_emission`] +
    /// [`Self::run_event_stack`]. Prefer the split methods when the context
    /// borrows from the guard to avoid borrow-checker conflicts.
    pub fn execute_event<S: GuardedEventStack + ?Sized>(
        &mut self,
        signal: LifecycleSignal,
        stack: &S,
    ) -> LifecycleEventOutcome {
        if !self.record_event_emission(signal) {
            return LifecycleEventOutcome::default();
        }
        self.run_event_stack(signal, stack)
    }

    fn run_guarded<S: GuardedEventStack + ?Sized>(
        &self,
        signal: LifecycleSignal,
        stack: &S,
        error: Option<&LifecycleErrorInfo>,
    ) -> LifecycleEventOutcome {
        let cleanup_event = matches!(
            signal,
            LifecycleSignal::Success
                | LifecycleSignal::Blocked
                | LifecycleSignal::Failure
                | LifecycleSignal::Cancelled
                | LifecycleSignal::Finalize
        );
        let _terminal_scope = cleanup_event.then(|| self.host.enter_terminal_event());
        let _cleanup = cleanup_event.then(CleanupScope::enter);
        stack.run_guarded(GuardedEvent {
            signal,
            config: &self.config,
            shell_runner: (!self.accounting.start_emitted()).then_some(&DisabledShellRunner as &dyn ShellRunner),
            error,
        })
    }

    /// Emit the `Finalize` signal once, after a terminal signal has fired.
    ///
    /// If no terminal signal has been emitted yet, this is a no-op (the
    /// caller should resolve the terminal state first).
    pub fn emit_finalize_once(&mut self) {
        if self.accounting.record_event_emission(LifecycleSignal::Finalize) {
            self.emit_signal(LifecycleSignal::Finalize);
        }
    }

    /// Reset per-iteration state so the guard can drive the next loop
    /// iteration without double-emitting terminal or finalize events.
    ///
    /// Preserves `initialize_emitted` so `initialize` fires exactly once
    /// across the whole loop run.
    pub fn reset_for_next_iteration(&mut self) {
        self.accounting.reset_for_next_iteration();
    }

    /// Reset **all** per-run state, including `initialize_emitted`, so the
    /// guard can drive a fresh document run from `initialize`.
    ///
    /// Unlike [`Self::reset_for_next_iteration`] (which preserves
    /// `initialize_emitted` because loop iterations re-enter at `start`), a
    /// `proxy` hand-off replaces the running document with another one that
    /// must run its own `initialize`/preflight. The proxied document is a
    /// distinct run, so every signal — including `initialize` — fires again,
    /// and shells stay refused until its `start`.
    pub fn reset_for_proxy(&mut self) {
        self.accounting.reset_for_proxy();
        self.proxied = true;
        // A previous hand-off's rebuilt context belongs to the document being
        // replaced. Drop it so the newly adopted target's events fall back to
        // the demand-driven capture until its own launch rebuild installs a
        // fresh one.
        self.run_prepared_context = None;
    }

    /// Install the early-binding context of the run now active.
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
    pub fn set_run_prepared_context(&mut self, context: ComposeContext) {
        self.run_prepared_context = Some(context);
    }

    /// The early-binding `ctx.*`/`env.*` snapshot lifecycle events should use,
    /// or `None` to force a demand-driven per-expression re-capture.
    ///
    /// Returns the host's run-start snapshot for a normally-run document.
    /// Once a later run has installed its own snapshot through
    /// [`Self::set_run_prepared_context`] — a proxied target's rebuild, which
    /// carries the target's own resolved provider/model identity, or a retried
    /// or resumed attempt's fresh capture — that snapshot wins. Before a proxied
    /// target's rebuild installs one (or if none is installed) a proxied guard
    /// returns `None`: the *original* document's snapshot omits any `ctx.*`
    /// group only the target references, so it is dropped and the executor
    /// re-captures, exactly as the target's own body composition already does.
    pub fn effective_prepared_context(&self) -> Option<&ComposeContext> {
        if let Some(context) = self.run_prepared_context.as_ref() {
            return Some(context);
        }
        if self.proxied {
            None
        } else {
            self.host.prepared_context()
        }
    }

    /// Suppress the Drop emission without emitting any signal.
    ///
    /// Use when transferring lifecycle responsibility elsewhere.
    pub fn defuse(&mut self) {
        self.accounting.defuse();
    }

    /// The run's event accounting.
    pub fn accounting(&self) -> &RunAccounting {
        &self.accounting
    }

    #[allow(missing_docs)]
    pub fn initialize_emitted(&self) -> bool {
        self.accounting.initialize_emitted()
    }

    /// Whether `start` has fired, which is also whether shells may run.
    pub fn start_emitted(&self) -> bool {
        self.accounting.start_emitted()
    }

    #[allow(missing_docs)]
    pub fn provider_launched(&self) -> bool {
        self.accounting.provider_launched()
    }

    /// The event holding the terminal slot, after any redesignation.
    pub fn terminal_signal(&self) -> Option<LifecycleSignal> {
        self.accounting.terminal_signal()
    }

    #[allow(missing_docs)]
    pub fn finalize_emitted(&self) -> bool {
        self.accounting.finalize_emitted()
    }

    /// The lifecycle configuration this guard is driving.
    pub fn config(&self) -> &LifecycleConfig {
        &self.config
    }

    /// Repoint the guard at a different lifecycle config (owned).
    ///
    /// A `proxy` hand-off replaces the running document with a different
    /// one. The target document is a distinct run with its own lifecycle
    /// blocks, so the guard must drive **its** events, not the proxying
    /// document's. Pair this with [`Self::reset_for_proxy`] so every signal —
    /// including `initialize` — fires again for the target.
    pub fn set_config(&mut self, config: LifecycleConfig) {
        self.config = Cow::Owned(config);
    }

    /// Complete cancellation cleanup without allowing recovery to replace its
    /// cause, and return whether cancellation was requested.
    ///
    /// Runs `cancelled` then `finalize` (each at most once, as
    /// [`Self::execute_event`] does) with the cancellation as `err`, in
    /// [`EvaluationMode::CancellationCleanup`]: a failure or a recovery
    /// directive in either stack is reported as a warning and changes nothing.
    /// Before `start`, both events stay shell-free.
    pub fn cancel_if_requested<S: GuardedEventStack + ?Sized>(&mut self, stack: &S) -> bool {
        let Some(cause) = cancellation::cause_with(|| self.host.interrupted()) else {
            return false;
        };
        let error = cause.error_with(self.host.diagnostics());
        for signal in [LifecycleSignal::Cancelled, LifecycleSignal::Finalize] {
            let cleanup = if self.record_event_emission(signal) {
                self.run_guarded(signal, stack, Some(&error))
            } else {
                LifecycleEventOutcome::default()
            };
            for warning in self.cleanup_warnings(EvaluationMode::CancellationCleanup, &cleanup) {
                self.host.effects().warn(&warning.msg);
            }
        }
        true
    }

    /// The diagnostics a cleanup event's outcome leaves without changing the
    /// run's terminal outcome: its own failures, a refused recovery directive,
    /// and an explicit `error(...)`.
    pub(crate) fn cleanup_warnings(
        &self,
        mode: EvaluationMode,
        outcome: &LifecycleEventOutcome,
    ) -> Vec<LifecycleErrorInfo> {
        let mut warnings: Vec<_> =
            outcome.evaluation_error.iter().chain(outcome.action_error.iter()).cloned().collect();
        if let Some(control) = outcome.control.as_ref() {
            if let Some(action) = super::policy::refused_recovery(mode, control) {
                warnings.push(self.host.failure(LifecycleRunFailure::CancellationRecovery {
                    source_path: self.host.source_path().to_path_buf(),
                    action: action.to_string(),
                }));
            }
            if let StackControl::Error { reason } = control {
                warnings.push(LifecycleErrorInfo::from_action_failure_with(
                    self.host.diagnostics(),
                    "error",
                    reason.clone().unwrap_or_else(|| "cancellation cleanup failed".into()),
                ));
            }
        }
        warnings
    }

    /// Emit a signal's unguarded, literal root communication without an
    /// event configuration.
    ///
    /// This is the drop safety net's path (and the `emit_*_once` helpers'):
    /// with no [`GuardedEventStack`] it cannot evaluate a guard, interpolate,
    /// or run anything but communication. It emits, in stack order (a root
    /// dictionary's canonical order), each root-level `stderr`, `message`,
    /// `notify`, `say`/`speak`, and `effect` whose text contains no `{{ … }}`
    /// span. Guarded items, templated text, and every other verb are skipped
    /// rather than emitted unevaluated. Ordinary runs go through
    /// [`Self::run_event_stack`] instead.
    ///
    /// After a user interrupt ([`LifecycleRunHost::interrupted`]) only the
    /// `stderr` lines emit.
    fn emit_signal(&self, signal: LifecycleSignal) {
        let effects = self.host.effects();
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
                effects.stderr(signal, text);
                continue;
            }
            if self.host.interrupted() {
                continue;
            }
            match comm.channel {
                CommunicationChannel::Message
                | CommunicationChannel::Notify
                | CommunicationChannel::Say
                | CommunicationChannel::Speak
                | CommunicationChannel::Effect => {
                    dispatch_communication(effects, comm.channel, signal, text);
                }
                _ => {}
            }
        }
    }
}

impl<H: LifecycleRunHost> Drop for LifecycleRunGuard<'_, H> {
    fn drop(&mut self) {
        if let Some(signal) = self.accounting.owed_terminal() {
            self.emit_signal(signal);
        }
    }
}

#[cfg(test)]
mod tests;
