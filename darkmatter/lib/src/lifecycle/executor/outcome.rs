//! What one lifecycle event reports: the control directive that ended it and
//! the failure that stopped it, if any.

use std::fmt;

use indexmap::IndexMap;
use serde_json::Value;

use super::super::LifecycleSignal;
use super::super::actions::{LifecyclePath, RetryBackoff};
use super::super::context::LifecycleErrorInfo;

/// A resolved lifecycle control action — the runtime-flow effect a stack item
/// requested, with every expression argument already evaluated.
///
/// The parse-time [`LifecycleControlAction`](super::super::actions::LifecycleControlAction)
/// carries unevaluated expression arguments; this is its post-evaluation form,
/// suitable for the host's runtime to act on directly.
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
    /// This is everything *evaluation* can know about a handoff. The host
    /// turns it into its proxy request by adding the one thing the lifecycle
    /// surface does not own: the invocation-wide proxy chain.
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

    /// End the nearest enclosing loop after the interrupted pass's cleanup.
    ///
    /// The executor has already recorded the break with the host
    /// ([`LifecycleHost::record_break`](super::LifecycleHost::record_break)):
    /// this control only ends the event.
    Break {
        /// Evaluated reason, if one was authored.
        reason: Option<String>,
        /// The process exit code when the loop it ends is the root
        /// invocation's.
        code: u8,
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

/// Where in a document's lifecycle frontmatter an action was authored.
///
/// Renders as the action's authored path (`failure[0].else[1].proxy`), the
/// same spelling every lifecycle diagnostic uses, so a proxy diagnostic and a
/// parse or shell diagnostic name the same property the same way. The path
/// is what the author wrote, never an execution position.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActionLocation {
    signal: LifecycleSignal,
    path: LifecyclePath,
}

impl ActionLocation {
    /// Locate the action at `path` in the stack of `signal`.
    pub fn new(signal: LifecycleSignal, path: LifecyclePath) -> Self {
        Self { signal, path }
    }

    #[allow(missing_docs)]
    pub fn signal(&self) -> LifecycleSignal {
        self.signal
    }

    #[allow(missing_docs)]
    pub fn path(&self) -> &LifecyclePath {
        &self.path
    }
}

impl fmt::Display for ActionLocation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.path)
    }
}
