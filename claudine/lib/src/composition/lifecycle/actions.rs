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
//!   `Resume`/`Defer`. At most one per stack item, always last.
//! - [`CommunicationAction`] — `say`/`speak`/`effect`/`message`/`notify`/
//!   `stderr`/`info`/`warn`.
//! - [`ShellAction`] — bespoke shell commands.
//! - [`SideEffectAction`] — Darkmatter side effects by name.
//! - [`ExpressionFunctionAction`] — read-only Darkmatter expression
//!   functions invoked for their result.

use darkmatter::markdown::compose::expression::Expr;

use super::LifecycleSignal;

#[path = "signatures.rs"]
mod signatures;
pub use signatures::*;

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

    /// `defer('5m')` — push this prompt onto the deferred-execution
    /// queue. Valid in `blocked`, `failure`.
    Defer {
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
            Self::Defer { .. } => "defer",
        }
    }

    /// Returns `true` when this control action is permitted in the given
    /// event per the spec's "Where valid" matrix.
    ///
    /// Flow control is **universal**: `Stop`, `Error`, `Retry`, `Resume`,
    /// `Defer`, and `Proxy` are valid in **every** event. Flow control reacts
    /// to *state* — an error, a missing file, an `env` value, frontmatter — and
    /// an error is just one kind of state (e.g. a `success` stack may `resume`
    /// the agent because an expected artifact was not produced).
    ///
    /// `Skip` is the **one** placement-restricted action: a whole-document
    /// opt-out is only coherent at `initialize`, before anything has run.
    ///
    /// Differences that look event-specific are **runtime capability**, not
    /// placement, and are checked at runtime rather than here: `Resume` needs a
    /// live provider session (pre-launch it surfaces `ResumeWithoutSession`),
    /// and `Retry`'s re-entry point (re-run pre-flight vs. re-invoke the
    /// provider) is derived from whether the provider had launched.
    pub fn is_valid_for(&self, event: LifecycleSignal) -> bool {
        match self {
            Self::Skip => matches!(event, LifecycleSignal::Initialize),
            _ => true,
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


/// Produce a did-you-mean rewrite from a removed short-form action to its
/// positional form.
///
/// Examples:
/// - `success("x")` → `success: "x"`
/// - `set_frontmatter('a','b','c')` → `set_frontmatter: ["a","b","c"]`
/// - `stop()` → `stop: []`
pub fn rewrite_to_positional(raw: &str) -> String {
    let Some(open) = raw.find('(') else {
        return format!("{raw}: []");
    };
    let close = raw.rfind(')').unwrap_or(raw.len());
    let verb = raw[..open].trim();
    let args_raw = raw[open + 1..close].trim();

    if args_raw.is_empty() {
        return format!("{verb}: []");
    }

    let args = split_short_form_args(args_raw);
    if args.is_empty() {
        return format!("{verb}: []");
    }
    if args.len() == 1 {
        return format!("{verb}: {}", yaml_like_arg(args[0]));
    }
    let formatted: Vec<String> = args.iter().map(|a| yaml_like_arg(a)).collect();
    format!("{verb}: [{}]", formatted.join(", "))
}

/// Split a short-form argument list on commas at the top level.
fn split_short_form_args(s: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut start = 0;
    let mut depth = 0i32;
    let mut quote: Option<char> = None;
    for (i, ch) in s.char_indices() {
        if let Some(qc) = quote {
            if ch == qc {
                quote = None;
            }
            continue;
        }
        match ch {
            '\'' | '"' => quote = Some(ch),
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => depth -= 1,
            ',' if depth == 0 => {
                out.push(&s[start..i]);
                start = i + 1;
            }
            _ => {}
        }
    }
    if start <= s.len() {
        out.push(&s[start..]);
    }
    out
}

/// Render a short-form argument as a YAML-like scalar for the did-you-mean
/// rewrite. This is a best-effort visual aid, not a full YAML serializer.
fn yaml_like_arg(arg: &str) -> String {
    let trimmed = arg.trim();
    if trimmed.is_empty() {
        return "\"\"".to_string();
    }
    // Strip one layer of matching quotes so 'a' becomes a, "a" becomes a.
    let unquoted = unwrap_matching_quotes(trimmed);
    // If the unquoted value looks like a bare scalar (no special chars), show
    // it as a quoted string so the rewrite is unambiguously YAML.
    format!("\"{}\"", unquoted.replace('"', "\\\""))
}

fn unwrap_matching_quotes(s: &str) -> &str {
    let bytes = s.as_bytes();
    if bytes.len() >= 2 {
        let q = bytes[0];
        if (q == b'\'' || q == b'"') && bytes[bytes.len() - 1] == q {
            let inner = &s[1..s.len() - 1];
            if !inner.as_bytes().contains(&q) {
                return inner;
            }
        }
    }
    s
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

        // Flow control is universal: Proxy/Retry/Resume/Defer are valid in
        // every event (placement is not error-gated).
        let every = [
            S::Initialize,
            S::Start,
            S::Success,
            S::Blocked,
            S::Failure,
            S::Finalize,
            S::Loop,
        ];
        let proxy = A::Proxy {
            target: Expr::StringLiteral("@other.md".into()),
        };
        let retry = A::Retry {
            max_attempts: None,
            backoff: None,
            delay: None,
        };
        let resume = A::Resume {
            message: Expr::StringLiteral("please retry".into()),
            max_attempts: None,
        };
        let requeue = A::Defer {
            delay: Expr::StringLiteral("5m".into()),
            reason: None,
        };
        for event in every {
            assert!(proxy.is_valid_for(event), "Proxy in {event:?}");
            assert!(retry.is_valid_for(event), "Retry in {event:?}");
            assert!(resume.is_valid_for(event), "Resume in {event:?}");
            assert!(requeue.is_valid_for(event), "Defer in {event:?}");
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

    // -------------------------------------------------------------------------
    // Signature parser
    // -------------------------------------------------------------------------

    #[test]
    fn parse_signature_fixed_positional() {
        let sig = parse_signature("set_frontmatter(file, prop, value)").unwrap();
        assert_eq!(sig.verb, "set_frontmatter");
        assert_eq!(sig.params, vec!["file", "prop", "value"]);
        assert_eq!(sig.optional_tail, 0);
        assert!(!sig.variadic);
        assert_eq!(sig.required_count(), 3);
        assert_eq!(sig.max_count(), Some(3));
    }

    #[test]
    fn parse_signature_optional_tail() {
        let sig = parse_signature("ensure_file(file, content?)").unwrap();
        assert_eq!(sig.verb, "ensure_file");
        assert_eq!(sig.params, vec!["file", "content"]);
        assert_eq!(sig.optional_tail, 1);
        assert!(!sig.variadic);
        assert_eq!(sig.required_count(), 1);
        assert_eq!(sig.max_count(), Some(2));
    }

    #[test]
    fn parse_signature_bracket_optional_tail() {
        // Darkmatter's catalog form for an optional trailing parameter.
        let sig = parse_signature("number(x, [default])").unwrap();
        assert_eq!(sig.verb, "number");
        assert_eq!(sig.params, vec!["x", "default"]);
        assert_eq!(sig.optional_tail, 1);
        assert!(!sig.variadic);
        assert_eq!(sig.required_count(), 1);
        assert_eq!(sig.max_count(), Some(2));
    }

    #[test]
    fn parse_signature_variadic() {
        let sig = parse_signature("and(...)").unwrap();
        assert_eq!(sig.verb, "and");
        assert!(sig.params.is_empty());
        assert_eq!(sig.optional_tail, 0);
        assert!(sig.variadic);
        assert_eq!(sig.required_count(), 0);
        assert_eq!(sig.max_count(), None);
    }

    #[test]
    fn parse_signature_zero_arg() {
        let sig = parse_signature("stop()").unwrap();
        assert_eq!(sig.verb, "stop");
        assert!(sig.params.is_empty());
        assert!(!sig.variadic);
        assert_eq!(sig.required_count(), 0);
        assert_eq!(sig.max_count(), Some(0));
    }

    #[test]
    fn parse_signature_malformed() {
        assert!(parse_signature("no_parens").is_none());
        assert!(parse_signature("(file)").is_none());
        assert!(parse_signature("verb(").is_none());
        assert!(parse_signature("verb(file").is_none());
        assert!(parse_signature("verb(a, ?)").is_none());
        assert!(parse_signature("verb(a?, b)").is_none());
    }

    // -------------------------------------------------------------------------
    // Side-effect signature derivation
    // -------------------------------------------------------------------------

    #[test]
    fn side_effect_signature_derives_from_descriptors() {
        let set = side_effect_signature("set_frontmatter").unwrap();
        assert_eq!(set.params, vec!["file", "prop", "value"]);
        assert_eq!(set.optional_tail, 0);

        let ensure = side_effect_signature("ensure_file").unwrap();
        assert_eq!(ensure.params, vec!["file", "content"]);
        assert_eq!(ensure.optional_tail, 1);

        assert!(side_effect_signature("not_a_verb").is_none());
    }

    // -------------------------------------------------------------------------
    // Expression-function signature derivation
    // -------------------------------------------------------------------------

    #[test]
    fn expression_function_signature_bracket_optional() {
        // `number(x, [default])` — the bracketed param is optional, so the
        // one-argument form `number("{{ value }}")` is valid arity.
        let sig = expression_function_signature("number").unwrap();
        assert_eq!(sig.params, vec!["x", "default"]);
        assert_eq!(sig.optional_tail, 1);
        assert_eq!(sig.required_count(), 1);
        assert_eq!(sig.max_count(), Some(2));

        let round = expression_function_signature("round").unwrap();
        assert_eq!(round.required_count(), 1);
        assert_eq!(round.max_count(), Some(2));
    }

    #[test]
    fn expression_function_signature_merges_overloads() {
        // Overloaded functions: the shorter overload's missing parameters are
        // optional, so the one-argument positional form is valid arity.
        let frontmatter = expression_function_signature("frontmatter").unwrap();
        assert_eq!(frontmatter.params, vec!["file", "prop"]);
        assert_eq!(frontmatter.optional_tail, 1);
        assert_eq!(frontmatter.required_count(), 1);
        assert_eq!(frontmatter.max_count(), Some(2));

        let link = expression_function_signature("link").unwrap();
        assert_eq!(link.required_count(), 1);
        assert_eq!(link.max_count(), Some(2));

        let validate = expression_function_signature("validate_schema").unwrap();
        assert_eq!(validate.required_count(), 1);
        assert_eq!(validate.max_count(), Some(2));
    }

    #[test]
    fn expression_function_signature_variadic_preserved() {
        let and = expression_function_signature("and").unwrap();
        assert!(and.variadic);
        assert_eq!(and.required_count(), 0);
        assert_eq!(and.max_count(), None);

        assert!(expression_function_signature("not_a_function").is_none());
    }

    // -------------------------------------------------------------------------
    // Known-verb predicate
    // -------------------------------------------------------------------------

    #[test]
    fn is_known_lifecycle_verb_unions_families() {
        // Communication
        assert!(is_known_lifecycle_verb("success"));
        assert!(is_known_lifecycle_verb("message"));
        assert!(is_known_lifecycle_verb("stderr"));

        // Shell
        assert!(is_known_lifecycle_verb("shell"));

        // Control
        assert!(is_known_lifecycle_verb("stop"));
        assert!(is_known_lifecycle_verb("retry"));
        assert!(is_known_lifecycle_verb("defer"));

        // Side-effect
        assert!(is_known_lifecycle_verb("set_frontmatter"));
        assert!(is_known_lifecycle_verb("ensure_file"));

        // Expression function
        assert!(is_known_lifecycle_verb("length"));
        assert!(is_known_lifecycle_verb("and"));
        assert!(is_known_lifecycle_verb("or"));
    }

    #[test]
    fn is_known_lifecycle_verb_rejects_unknown() {
        assert!(!is_known_lifecycle_verb("sucess"));
        assert!(!is_known_lifecycle_verb("nope"));
        assert!(!is_known_lifecycle_verb(""));
    }

    // -------------------------------------------------------------------------
    // Short-form rewrite helper
    // -------------------------------------------------------------------------

    #[test]
    fn rewrite_to_positional_communication() {
        assert_eq!(rewrite_to_positional("success(\"x\")"), "success: \"x\"");
    }

    #[test]
    fn rewrite_to_positional_multi_arg() {
        assert_eq!(
            rewrite_to_positional("set_frontmatter('a','b','c')"),
            "set_frontmatter: [\"a\", \"b\", \"c\"]"
        );
    }

    #[test]
    fn rewrite_to_positional_zero_arg() {
        assert_eq!(rewrite_to_positional("stop()"), "stop: []");
    }

    #[test]
    fn rewrite_to_positional_bare_verb() {
        assert_eq!(rewrite_to_positional("stop"), "stop: []");
    }
}
