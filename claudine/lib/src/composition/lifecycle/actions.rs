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

use darkmatter::markdown::compose::expression::{Expr, ExpressionFinder};
use indexmap::IndexMap;

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
    Proxy {
        /// File reference expression resolving to the target prompt path.
        target: Expr,
        /// Transient top-level frontmatter overlay for the immediate target,
        /// authored as key/value `with:`. Empty when `with:` was omitted or
        /// authored as `{}`.
        with: ProxyWith,
    },

    /// `retry` / `retry(N)` — try the current prompt again. Whether re-entry
    /// re-runs pre-flight or re-invokes the provider is derived at runtime from
    /// whether the provider had launched.
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
    /// message. Needs a live provider session at runtime; pre-launch it
    /// surfaces `ResumeWithoutSession`.
    Resume {
        /// The follow-up prompt. Required.
        message: Expr,
        /// Number of additional attempts beyond the original. `None` is the
        /// default (one resume).
        max_attempts: Option<Expr>,
    },

    /// `defer('5m')` — push this prompt onto the deferred-execution
    /// queue. Parsed but not yet wired to a runtime backend.
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

/// One value inside an authored `proxy.with` mapping, typed at parse time.
///
/// Leaves are [`Expr`] trees produced by the same rule that types every other
/// lifecycle action parameter, recursed through arrays and objects. Holding
/// expressions rather than raw JSON is what lets the handoff preserve a
/// whole-value span's type: `{{ true }}` stays a bool instead of collapsing to
/// the string `"true"`.
#[derive(Debug, Clone, PartialEq)]
pub enum ProxyWithValue {
    /// An authored scalar or interpolation-bearing string.
    Scalar(Expr),
    /// An authored YAML `null`. [`Expr`] has no null literal, and `with:`
    /// values keep their authored types, so null gets its own variant rather
    /// than forcing authors through `{{ null }}`.
    Null,
    /// An authored YAML sequence; elements follow the same rule.
    Array(Vec<ProxyWithValue>),
    /// An authored YAML mapping; values follow the same rule. Its keys are
    /// data, not property names — only the overlay's own top-level keys name
    /// target frontmatter properties, so only those are span-checked.
    Object(IndexMap<String, ProxyWithValue>),
}

/// Why an authored `with:` mapping could not be typed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProxyWithError {
    /// A top-level key carries an interpolation span. Carries the key verbatim.
    DynamicKey(String),
    /// A value could not be typed by the shared action-value rule.
    Value {
        /// Path below `with`, e.g. `metadata.area` or `files[0]`.
        path: String,
        /// The reason from the shared rule.
        message: String,
    },
}

/// The authored `proxy.with` mapping — a transient top-level frontmatter
/// overlay for the immediate proxy target.
///
/// Values are held as typed [`ProxyWithValue`] trees. They are resolved once,
/// at the source handoff, against the source document's lifecycle context;
/// this type is the parse-time carrier, not the evaluated overlay.
///
/// Keys are static YAML strings by construction: [`ProxyWith::new`] is the
/// only constructor and rejects a key carrying an interpolation span, so a
/// downstream consumer never has to re-check for a dynamic key.
///
/// ## Examples
///
/// ```
/// use claudine::composition::lifecycle::actions::{ProxyWith, ProxyWithError};
/// use indexmap::IndexMap;
///
/// let mut authored = IndexMap::new();
/// authored.insert("iteration".to_string(), serde_json::json!("{{ iteration }}"));
/// let with = ProxyWith::new(authored).expect("static key");
/// assert_eq!(with.len(), 1);
///
/// let mut dynamic = IndexMap::new();
/// dynamic.insert("{{ key }}".to_string(), serde_json::json!(1));
/// assert_eq!(
///     ProxyWith::new(dynamic).unwrap_err(),
///     ProxyWithError::DynamicKey("{{ key }}".to_string())
/// );
/// ```
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ProxyWith(IndexMap<String, ProxyWithValue>);

impl ProxyWith {
    /// Type an overlay from an authored mapping.
    ///
    /// ## Errors
    ///
    /// [`ProxyWithError::DynamicKey`] when a top-level key carries a `{{ … }}`
    /// or `$( … )` span — `with:` keys name target frontmatter properties and
    /// are never interpolated, so a span in a key is an authoring error rather
    /// than a value to resolve later. [`ProxyWithError::Value`] when a value
    /// fails the shared action-value rule (e.g. a whole-value span that is not
    /// a parseable expression).
    ///
    /// Keys are checked before values, so an overlay with both faults reports
    /// the key.
    pub fn new(authored: IndexMap<String, serde_json::Value>) -> Result<Self, ProxyWithError> {
        for key in authored.keys() {
            if !ExpressionFinder::find_all_plain(key).is_empty() || key.contains("$(") {
                return Err(ProxyWithError::DynamicKey(key.clone()));
            }
        }
        let mut typed = IndexMap::with_capacity(authored.len());
        for (key, value) in authored {
            let value = type_with_value(&value, &key)?;
            typed.insert(key, value);
        }
        Ok(Self(typed))
    }

    #[allow(missing_docs)]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    #[allow(missing_docs)]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// The typed value for `key`, or `None` when the overlay does not set that
    /// property.
    pub fn get(&self, key: &str) -> Option<&ProxyWithValue> {
        self.0.get(key)
    }

    /// Iterate the overlay's properties.
    ///
    /// Order is deterministic but is **not** the authored order: `serde_json`
    /// is built without `preserve_order`, so frontmatter parsing has already
    /// normalized a nested mapping's keys to sorted order before they reach
    /// here. Overlay semantics are per-key, so no consumer may depend on
    /// order for meaning.
    pub fn iter(&self) -> impl Iterator<Item = (&String, &ProxyWithValue)> {
        self.0.iter()
    }
}

/// Recurse the shared action-value rule through arrays and objects, tagging
/// each failure with its path below `with`.
fn type_with_value(value: &serde_json::Value, path: &str) -> Result<ProxyWithValue, ProxyWithError> {
    match value {
        serde_json::Value::Null => Ok(ProxyWithValue::Null),
        serde_json::Value::Array(items) => items
            .iter()
            .enumerate()
            .map(|(i, item)| type_with_value(item, &format!("{path}[{i}]")))
            .collect::<Result<Vec<_>, _>>()
            .map(ProxyWithValue::Array),
        serde_json::Value::Object(map) => map
            .iter()
            .map(|(k, v)| {
                type_with_value(v, &format!("{path}.{k}")).map(|typed| (k.clone(), typed))
            })
            .collect::<Result<IndexMap<_, _>, _>>()
            .map(ProxyWithValue::Object),
        scalar => super::action_shape::action_value_to_expr(scalar)
            .map(ProxyWithValue::Scalar)
            .map_err(|message| ProxyWithError::Value {
                path: path.to_string(),
                message,
            }),
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
mod tests;
