//! Typed action model for lifecycle event stacks.
//!
//! Each lifecycle event (`initialize`, `start`, `success`, `blocked`,
//! `failure`, `finalize`, `loop`) may carry a `stack:` of conditional
//! actions. This module defines the parsed, typed form of those actions and
//! the per-event validity matrix that governs where lifecycle control
//! actions may appear.
//!
//! ## Parsing
//!
//! Raw frontmatter values are turned into [`LifecycleStackItem`] values by
//! [`super::lifecycle::parse_lifecycle_config`], which calls into the
//! short-form / long-form parsers in this module. The parsers validate
//! argument shape and the per-event "Where valid" matrix at parse time so a
//! malformed stack never reaches runtime execution.
//!
//! ## Categories
//!
//! The spec groups actions into five categories; this module mirrors that
//! split with [`LifecycleActionKind`] variants:
//!
//! - [`LifecycleControlAction`] — `Stop`/`Skip`/`Error`/`Proxy`/`Retry`/
//!   `Resume`/`Requeue`. At most one per stack item, always last.
//! - [`CommunicationAction`] — `say`/`speak`/`effect`/`message`/`notify`/
//!   `stderr`/`info`/`warn`.
//! - [`ShellAction`] — bespoke shell commands.
//! - [`SideEffectAction`] — Darkmatter side effects by name.
//! - [`ExpressionFunctionAction`] — read-only Darkmatter expression
//!   functions invoked for their result.

use darkmatter::markdown::compose::expression::Expr;

use super::lifecycle::LifecycleSignal;

/// A single stack item: an optional `when:` condition plus one or more
/// ordered actions.
///
/// At parse time the cardinality rule is enforced: at most one action may
/// be a [`LifecycleActionKind::LifecycleControl`], and that action must be
/// the last in `actions` (subsequent actions would be unreachable because
/// a lifecycle control action terminates stack processing for the event).
#[derive(Debug, Clone, PartialEq)]
pub struct LifecycleStackItem {
    /// Darkmatter condition expression evaluated against the lifecycle
    /// execution context. `None` means the item always executes.
    pub when: Option<Expr>,

    /// Ordered actions executed when `when` evaluates truthy (or is omitted).
    pub actions: Vec<LifecycleAction>,
}

/// One parsed action with its `no_error` flag.
#[derive(Debug, Clone, PartialEq)]
pub struct LifecycleAction {
    /// The typed action body.
    pub kind: LifecycleActionKind,

    /// When `true`, an unintentional error from this action is logged but
    /// does not propagate: stack processing continues to the next item and
    /// the composition outcome is unchanged regardless of which event is
    /// processing the stack.
    ///
    /// Defaults to `false`. The spec extends the existing `shell.no_error`
    /// flag to every action category.
    pub no_error: bool,
}

impl LifecycleAction {
    /// Returns `true` when this action is a lifecycle control action.
    ///
    /// Used by the cardinality check (at most one per stack item) and the
    /// "must be last" check.
    pub fn is_lifecycle_control(&self) -> bool {
        matches!(self.kind, LifecycleActionKind::LifecycleControl(_))
    }
}

/// The typed body of a lifecycle action.
#[derive(Debug, Clone, PartialEq)]
pub enum LifecycleActionKind {
    /// Lifecycle control action — terminates stack processing for the event.
    LifecycleControl(LifecycleControlAction),

    /// Communication action (TTS / sound / message / status line).
    Communication(CommunicationAction),

    /// Bespoke shell command.
    Shell(ShellAction),

    /// Darkmatter side-effect invoked by name.
    SideEffect(SideEffectAction),

    /// Read-only Darkmatter expression function invoked for its result.
    ExpressionFunction(ExpressionFunctionAction),
}

/// A lifecycle control action.
///
/// The first one whose `when:` clause matches **terminates stack
/// processing** for the current event. The "Where valid" matrix that
/// governs which events each variant may appear in is encoded in
/// [`LifecycleControlAction::is_valid_for`].
#[derive(Debug, Clone, PartialEq)]
pub enum LifecycleControlAction {
    /// `stop` — end this event's stack cleanly. Composition continues with
    /// the current outcome unchanged. Valid in every event.
    Stop,

    /// `skip` — whole-document opt-out. Valid only in `initialize`.
    Skip,

    /// `error("reason")` — mark this event as failed with a reason. Valid
    /// in every event; the effect depends on the event (see the spec's
    /// explicit-transition table).
    Error {
        /// Optional human-readable reason. Evaluated against the lifecycle
        /// context at runtime.
        reason: Option<Expr>,
    },

    /// `proxy('@foo.md')` — hand off execution to another prompt document.
    /// Valid in `initialize`, `blocked`, `failure`.
    Proxy {
        /// File reference expression resolving to the target prompt path.
        target: Expr,
    },

    /// `retry` / `retry(N)` — try the current prompt again. Valid in
    /// `blocked`, `failure`.
    Retry {
        /// Number of additional attempts beyond the original. `None` is the
        /// default (one retry).
        max_attempts: Option<Expr>,
        /// Backoff strategy. `None` defaults to fixed.
        backoff: Option<RetryBackoff>,
        /// Delay duration expression. `None` defaults to zero.
        delay: Option<Expr>,
    },

    /// `resume("message")` — resume the agent session with a follow-up
    /// message. Valid only in `failure`.
    Resume {
        /// The follow-up prompt. Required.
        message: Expr,
        /// Number of additional attempts beyond the original. `None` is the
        /// default (one resume).
        max_attempts: Option<Expr>,
    },

    /// `requeue('5m')` — push this prompt onto the deferred-execution
    /// queue. Valid in `blocked`, `failure`.
    Requeue {
        /// Delay duration expression. Required.
        delay: Expr,
        /// Optional human-readable reason.
        reason: Option<Expr>,
    },
}

impl LifecycleControlAction {
    /// Returns the canonical short-form verb for this action.
    pub fn verb(&self) -> &'static str {
        match self {
            Self::Stop => "stop",
            Self::Skip => "skip",
            Self::Error { .. } => "error",
            Self::Proxy { .. } => "proxy",
            Self::Retry { .. } => "retry",
            Self::Resume { .. } => "resume",
            Self::Requeue { .. } => "requeue",
        }
    }

    /// Returns `true` when this control action is permitted in the given
    /// event per the spec's "Where valid" matrix.
    ///
    /// - `Stop` and `Error` are valid in every event.
    /// - `Skip` is valid only in `initialize`.
    /// - `Proxy` is valid in `initialize`, `blocked`, `failure`, `finalize`.
    /// - `Retry` is valid in `blocked`, `failure`, `finalize`.
    /// - `Resume` is valid in `failure`, `finalize`.
    /// - `Requeue` is valid in `blocked`, `failure`, `finalize`.
    ///
    /// `finalize` is the optional-error terminal event, so it doubles as a
    /// last-chance recovery surface: a `finalize.stack` that detects an
    /// unmet contract (typically guarded by `when: "err"`) can `retry`,
    /// `resume`, `requeue`, or `proxy` exactly as `failure` can.
    pub fn is_valid_for(&self, event: LifecycleSignal) -> bool {
        match self {
            Self::Stop | Self::Error { .. } => true,
            Self::Skip => matches!(event, LifecycleSignal::Initialize),
            Self::Proxy { .. } => matches!(
                event,
                LifecycleSignal::Initialize
                    | LifecycleSignal::Blocked
                    | LifecycleSignal::Failure
                    | LifecycleSignal::Finalize
            ),
            Self::Retry { .. } | Self::Requeue { .. } => matches!(
                event,
                LifecycleSignal::Blocked | LifecycleSignal::Failure | LifecycleSignal::Finalize
            ),
            Self::Resume { .. } => {
                matches!(event, LifecycleSignal::Failure | LifecycleSignal::Finalize)
            }
        }
    }
}

/// Backoff strategy for `Retry`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetryBackoff {
    /// Constant delay between attempts (default).
    Fixed,
    /// Delay doubles after each attempt.
    Exponential,
}

impl RetryBackoff {
    /// Parse from the canonical string form used in frontmatter.
    pub fn parse(value: &str) -> Option<Self> {
        match value.trim() {
            "fixed" => Some(Self::Fixed),
            "exponential" => Some(Self::Exponential),
            _ => None,
        }
    }

    /// Canonical string form (matches the frontmatter accepted form).
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Fixed => "fixed",
            Self::Exponential => "exponential",
        }
    }
}

/// A communication action.
#[derive(Debug, Clone, PartialEq)]
pub struct CommunicationAction {
    /// Which channel this action targets.
    pub channel: CommunicationChannel,
    /// The message expression (text to speak, status line, notification
    /// body, etc. — interpreted per channel).
    pub message: Expr,
    /// Optional route pin for messengers; ignored by other channels.
    pub route: Option<Expr>,
}

/// Communication channels available as both top-level properties and
/// discrete stack actions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommunicationChannel {
    /// TTS via host speech provider (after `effect` when both are present).
    Say,
    /// Alias for [`Self::Say`].
    Speak,
    /// Embedded sound effect.
    Effect,
    /// Configured messenger route (Slack, Discord, WhatsApp, Signal, webhooks).
    Message,
    /// OS desktop notification.
    Notify,
    /// Styled status line to stderr.
    Stderr,
    /// Status line rendered with `Status::Info` style.
    Info,
    /// Status line rendered with `Status::Warn` style.
    Warn,
    /// Status line rendered with `Status::Success` style.
    Success,
    /// Plain text written to stdout (no status glyph).
    Stdout,
}

impl CommunicationChannel {
    /// Canonical short-form verb for this channel.
    pub const fn verb(self) -> &'static str {
        match self {
            Self::Say => "say",
            Self::Speak => "speak",
            Self::Effect => "effect",
            Self::Message => "message",
            Self::Notify => "notify",
            Self::Stderr => "stderr",
            Self::Info => "info",
            Self::Warn => "warn",
            Self::Success => "success",
            Self::Stdout => "stdout",
        }
    }

    /// Parse a verb into a channel. Returns `None` for unrecognized verbs.
    pub fn from_verb(verb: &str) -> Option<Self> {
        match verb {
            "say" => Some(Self::Say),
            "speak" => Some(Self::Speak),
            "effect" => Some(Self::Effect),
            "message" => Some(Self::Message),
            "notify" => Some(Self::Notify),
            "stderr" => Some(Self::Stderr),
            "info" => Some(Self::Info),
            "warn" => Some(Self::Warn),
            "success" => Some(Self::Success),
            "stdout" => Some(Self::Stdout),
            _ => None,
        }
    }
}

/// A shell action.
#[derive(Debug, Clone, PartialEq)]
pub struct ShellAction {
    /// The shell command expression. The spec example uses string literals,
    /// but the model accepts any expression so interpolation can compose the
    /// command at runtime.
    pub command: Expr,
    /// Message to emit when the command exits non-zero.
    pub on_error: Option<Expr>,
}

/// A side-effect action — a Darkmatter effect verb invoked by name with
/// positional expression arguments.
///
/// The verb catalog is owned by Darkmatter; this model deliberately does not
/// enumerate it. Phase 4 execution will dispatch by name through the
/// Darkmatter effect engine.
#[derive(Debug, Clone, PartialEq)]
pub struct SideEffectAction {
    /// The Darkmatter effect verb (e.g. `"set_frontmatter"`, `"ensure_file"`).
    pub verb: String,
    /// Positional expression arguments.
    pub args: Vec<Expr>,
}

/// An expression-function action — a read-only Darkmatter expression
/// function invoked for its result (typically logged or fed into a
/// subsequent action).
#[derive(Debug, Clone, PartialEq)]
pub struct ExpressionFunctionAction {
    /// The function name (e.g. `"file_exists"`, `"length"`).
    pub function: String,
    /// Positional expression arguments.
    pub args: Vec<Expr>,
}

/// The positional parameter names for a known Darkmatter side-effect verb,
/// in call order.
///
/// Long-form side-effect actions carry their arguments as named sibling keys
/// (`file:`, `prop:`, `value:`). The parser uses this table to reorder those
/// named parameters into the verb's positional call order so the executor can
/// dispatch positionally. Returns `None` for verbs not in the catalog.
///
/// `ensure_file` lists `content` as its optional second parameter; the
/// executor calls `ensure_file_with_content` when both are present and
/// `ensure_file` otherwise.
///
/// ## Notes
///
/// The Darkmatter side-effect catalog is the authority. This table mirrors
/// the public verb signatures in `darkmatter/lib/src/effects/verbs.rs`; keep
/// the two in sync when the catalog gains or renames a verb.
pub fn side_effect_signature(verb: &str) -> Option<&'static [&'static str]> {
    let sig: &'static [&'static str] = match verb {
        "set_frontmatter" => &["file", "prop", "value"],
        "merge_frontmatter" => &["file", "obj"],
        "delete_frontmatter" => &["file", "prop"],
        "increment_frontmatter" => &["file", "prop"],
        "decrement_frontmatter" => &["file", "prop"],
        "append_frontmatter" => &["file", "prop", "value"],
        "prepend_frontmatter" => &["file", "prop", "value"],
        "ensure_file" => &["file", "content"],
        "ensure_dir" => &["dir"],
        "append_line" => &["file", "text"],
        "append_jsonl" => &["file", "obj"],
        "http_post" => &["url", "body"],
        _ => return None,
    };
    Some(sig)
}

/// Returns `true` when `verb` names a known Darkmatter side-effect.
///
/// Short-form `verb(args)` actions whose verb is not a communication,
/// shell, or lifecycle-control keyword parse as
/// [`ExpressionFunctionAction`]. At execution time the stack executor uses
/// this predicate to route a known side-effect verb (e.g.
/// `ensure_file('@x')`) to the side-effect engine rather than the read-only
/// expression engine.
pub fn is_known_side_effect(verb: &str) -> bool {
    side_effect_signature(verb).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn control_action_validity_matrix() {
        use LifecycleControlAction as A;
        use LifecycleSignal as S;

        // Stop and Error are valid in every event.
        for event in [
            S::Initialize,
            S::Start,
            S::Success,
            S::Blocked,
            S::Failure,
            S::Finalize,
            S::Loop,
        ] {
            assert!(A::Stop.is_valid_for(event), "Stop in {event:?}");
            assert!(
                A::Error { reason: None }.is_valid_for(event),
                "Error in {event:?}"
            );
        }

        // Skip is valid only in Initialize.
        assert!(A::Skip.is_valid_for(S::Initialize));
        for event in [S::Start, S::Success, S::Blocked, S::Failure, S::Finalize, S::Loop] {
            assert!(!A::Skip.is_valid_for(event), "Skip in {event:?}");
        }

        // Proxy: Initialize, Blocked, Failure, Finalize.
        let proxy = A::Proxy {
            target: Expr::StringLiteral("@other.md".into()),
        };
        for event in [S::Initialize, S::Blocked, S::Failure, S::Finalize] {
            assert!(proxy.is_valid_for(event), "Proxy in {event:?}");
        }
        for event in [S::Start, S::Success, S::Loop] {
            assert!(!proxy.is_valid_for(event), "Proxy in {event:?}");
        }

        // Retry: Blocked, Failure, Finalize.
        let retry = A::Retry {
            max_attempts: None,
            backoff: None,
            delay: None,
        };
        for event in [S::Blocked, S::Failure, S::Finalize] {
            assert!(retry.is_valid_for(event), "Retry in {event:?}");
        }
        for event in [S::Initialize, S::Start, S::Success, S::Loop] {
            assert!(!retry.is_valid_for(event), "Retry in {event:?}");
        }

        // Resume: Failure, Finalize.
        let resume = A::Resume {
            message: Expr::StringLiteral("please retry".into()),
            max_attempts: None,
        };
        for event in [S::Failure, S::Finalize] {
            assert!(resume.is_valid_for(event), "Resume in {event:?}");
        }
        for event in [
            S::Initialize,
            S::Start,
            S::Success,
            S::Blocked,
            S::Loop,
        ] {
            assert!(!resume.is_valid_for(event), "Resume in {event:?}");
        }

        // Requeue: Blocked, Failure, Finalize.
        let requeue = A::Requeue {
            delay: Expr::StringLiteral("5m".into()),
            reason: None,
        };
        for event in [S::Blocked, S::Failure, S::Finalize] {
            assert!(requeue.is_valid_for(event), "Requeue in {event:?}");
        }
        for event in [S::Initialize, S::Start, S::Success, S::Loop] {
            assert!(!requeue.is_valid_for(event), "Requeue in {event:?}");
        }
    }

    #[test]
    fn control_action_verb_round_trip() {
        use LifecycleControlAction as A;
        assert_eq!(A::Stop.verb(), "stop");
        assert_eq!(A::Skip.verb(), "skip");
        assert_eq!(A::Error { reason: None }.verb(), "error");
    }

    #[test]
    fn retry_backoff_round_trip() {
        assert_eq!(RetryBackoff::parse("fixed"), Some(RetryBackoff::Fixed));
        assert_eq!(
            RetryBackoff::parse("exponential"),
            Some(RetryBackoff::Exponential)
        );
        assert_eq!(RetryBackoff::parse("bogus"), None);
        assert_eq!(RetryBackoff::Fixed.as_str(), "fixed");
        assert_eq!(RetryBackoff::Exponential.as_str(), "exponential");
    }

    #[test]
    fn communication_channel_verb_round_trip() {
        for channel in [
            CommunicationChannel::Say,
            CommunicationChannel::Speak,
            CommunicationChannel::Effect,
            CommunicationChannel::Message,
            CommunicationChannel::Notify,
            CommunicationChannel::Stderr,
            CommunicationChannel::Info,
            CommunicationChannel::Warn,
            CommunicationChannel::Success,
            CommunicationChannel::Stdout,
        ] {
            let verb = channel.verb();
            assert_eq!(
                CommunicationChannel::from_verb(verb),
                Some(channel),
                "round-trip for {verb}"
            );
        }
        assert_eq!(CommunicationChannel::from_verb("bogus"), None);
    }

    #[test]
    fn is_lifecycle_control_flag() {
        let lc_stop = LifecycleAction {
            kind: LifecycleActionKind::LifecycleControl(LifecycleControlAction::Stop),
            no_error: false,
        };
        let comm = LifecycleAction {
            kind: LifecycleActionKind::Communication(CommunicationAction {
                channel: CommunicationChannel::Say,
                message: Expr::StringLiteral("hi".into()),
                route: None,
            }),
            no_error: false,
        };
        assert!(lc_stop.is_lifecycle_control());
        assert!(!comm.is_lifecycle_control());
    }
}
