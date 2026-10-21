//! Typed action model for lifecycle stacks.
//!
//! A **stack** is any place the lifecycle action grammar is accepted: the seven
//! events (`initialize`, `start`, `blocked`, `success`, `failure`,
//! `finalize`, `cancelled`), the `loop:` block's lifecycle concerns, and a sequence task's
//! `setup:`/`teardown:`. Each parses into an ordered list of
//! [`LifecycleNode`]s: a leaf [`LifecycleAction`], or a
//! [`LifecycleConditional`] whose `then`/`else` bodies are themselves ordered
//! node lists. Dictionary bundles are expanded into that list at parse time,
//! so the executor never iterates a mapping to choose order.
//!
//! The model names no host runtime type (settings, providers, messaging,
//! TTS, or sound playback); a host executes it.
//!
//! ## Categories
//!
//! [`LifecycleActionKind`] mirrors the action inventory:
//!
//! - [`LifecycleControlAction`] — the flow-control directives. Every one
//!   except `prep` ends the whole event, so it is the last item of its list;
//!   `prep` returns to the item after it.
//! - [`CommunicationAction`] — `say`/`speak`/`effect`/`message`/`notify`/
//!   `stderr`/`stdout`/`info`/`warn`/`success`.
//! - [`ShellAction`] — bespoke shell commands.
//! - [`SideEffectAction`] — Darkmatter side effects by name.
//! - [`RuntimeSet`] — invocation-local frontmatter mutations.
//! - [`ExpressionFunctionAction`] — read-only Darkmatter expression
//!   functions invoked for their result.

use crate::markdown::compose::expression::{Expr, GuardOrigin, Polarity, TypeScope, WriteSet};
use crate::markdown::compose::shell_expansion::ShellExpansionError;
use crate::markdown::compose::{ResolvedShellValue, check_frontmatter_shell_value};
use indexmap::IndexMap;

use super::LifecycleSignal;
pub use super::source_map::LifecyclePath;

#[path = "signatures.rs"]
mod signatures;
pub use signatures::*;

/// One entry of a parsed stack, in authored order.
#[derive(Debug, Clone, PartialEq)]
pub enum LifecycleNode {
    /// A leaf action.
    Action(LifecycleAction),
    /// A `when` guard selecting one of two ordered bodies.
    Conditional(LifecycleConditional),
}

/// A guarded list item: `when` with `then` (and optionally `else`), or `when`
/// with sibling verbs, whose expansion becomes `then`.
///
/// The guard is evaluated once, when the item is reached, and only the
/// selected body runs. Both bodies were parsed and validated before anything
/// executed.
#[derive(Debug, Clone, PartialEq)]
pub struct LifecycleConditional {
    /// The guard, a Darkmatter condition expression.
    pub when: Expr,
    /// The body run when the guard is truthy. Never empty.
    pub then: Vec<LifecycleNode>,
    /// The body run when the guard is falsy; empty when `else` was omitted
    /// (an explicitly authored `else` is never empty).
    pub otherwise: Vec<LifecycleNode>,
    /// The authored path of the item, e.g. `failure[0]`; the guard is at
    /// `{path}.when`.
    pub path: LifecyclePath,
}

/// Visit every action in `nodes`, in authored order, through both branches of
/// every conditional.
///
/// This is a structural walk: guards are not evaluated, so callers that need
/// every reachable action regardless of state (shell approval, the
/// `initialize` shell prohibition) see each one exactly once.
pub fn for_each_action<'a>(nodes: &'a [LifecycleNode], visit: &mut impl FnMut(&'a LifecycleAction)) {
    for node in nodes {
        match node {
            LifecycleNode::Action(action) => visit(action),
            LifecycleNode::Conditional(conditional) => {
                for_each_action(&conditional.then, visit);
                for_each_action(&conditional.otherwise, visit);
            }
        }
    }
}

/// [`for_each_action`] with mutable access, for preflight to fix approved
/// command bytes in place.
pub fn for_each_action_mut(nodes: &mut [LifecycleNode], visit: &mut impl FnMut(&mut LifecycleAction)) {
    for node in nodes {
        match node {
            LifecycleNode::Action(action) => visit(action),
            LifecycleNode::Conditional(conditional) => {
                for_each_action_mut(&mut conditional.then, visit);
                for_each_action_mut(&mut conditional.otherwise, visit);
            }
        }
    }
}

/// Visit every action in `nodes`, in authored order and through both branches
/// of every conditional, with the type scope that holds where it sits;
/// returns the scope that continues after the list.
///
/// A conditional's `then` body sees `scope` assumed on the true side of its
/// `when`, its `else` body the false side, each labelled with the guard's
/// authored path (`success[4].when`). After the conditional, the scope is the
/// [`TypeScope::join`] of the two bodies' ends, so a write in either branch
/// invalidates what came before. An action is visited under the scope that
/// reaches it, and only then do its writes ([`action_writes`]) invalidate
/// facts for the actions after it, so a `set` value is checked before its own
/// write.
///
/// Like [`for_each_action`], the walk is structural: no guard, predicate, or
/// mutation is evaluated.
///
/// ## Errors
///
/// The first error `visit` returns; the walk stops there.
pub fn walk_scoped<'d, E>(
    nodes: &mut [LifecycleNode],
    scope: TypeScope<'d>,
    visit: &mut dyn FnMut(&mut LifecycleAction, &TypeScope<'d>) -> Result<(), E>,
) -> Result<TypeScope<'d>, E> {
    let mut scope = scope;
    for node in nodes {
        match node {
            LifecycleNode::Action(action) => {
                visit(action, &scope)?;
                for writes in action_writes(action) {
                    scope = scope.invalidate(&writes);
                }
            }
            LifecycleNode::Conditional(conditional) => {
                let origin = GuardOrigin::new(conditional.path.field("when").to_string());
                let then_scope = scope.assume_at(&conditional.when, Polarity::True, origin.clone());
                let else_scope = scope.assume_at(&conditional.when, Polarity::False, origin);
                let then_end = walk_scoped(&mut conditional.then, then_scope, visit)?;
                let else_end = walk_scoped(&mut conditional.otherwise, else_scope, visit)?;
                scope = TypeScope::join(&then_end, &else_end);
            }
        }
    }
    Ok(scope)
}

/// The document properties an action may write.
///
/// A `set` writes each destination it names. A Darkmatter side effect may
/// rewrite this document's frontmatter under any key, so it writes every
/// document property. Every other action writes none.
pub fn action_writes(action: &LifecycleAction) -> Vec<WriteSet> {
    match &action.kind {
        LifecycleActionKind::RuntimeSet(set) => set.iter().map(|(key, _)| WriteSet::destination(key)).collect(),
        LifecycleActionKind::SideEffect(_) => vec![WriteSet::AllDocument],
        LifecycleActionKind::ExpressionFunction(function) if is_known_side_effect(&function.function) => {
            vec![WriteSet::AllDocument]
        }
        _ => Vec::new(),
    }
}

/// One parsed action with its `no_error` flag and authored path.
#[derive(Debug, Clone, PartialEq)]
pub struct LifecycleAction {
    /// The typed action body.
    pub kind: LifecycleActionKind,

    /// Suppress dispatch failures and continue the stack. Expression errors
    /// and the prohibition on shells before preflight cannot be suppressed.
    /// Never set on a `set` action.
    pub no_error: bool,

    /// The authored path of the verb key, e.g. `failure[0].else[1].shell` or,
    /// for a root dictionary, `start.message`. A `speak` action keeps `speak`.
    pub path: LifecyclePath,
}

impl LifecycleAction {
    /// Returns `true` when this action is a flow-control directive.
    pub fn is_lifecycle_control(&self) -> bool {
        matches!(self.kind, LifecycleActionKind::LifecycleControl(_))
    }

    /// Returns `true` when this action is a directive that ends its event, so
    /// nothing may follow it unconditionally in its list. `prep` is the one
    /// directive that returns.
    pub fn ends_event(&self) -> bool {
        matches!(&self.kind, LifecycleActionKind::LifecycleControl(control) if !control.returns())
    }
}

/// The typed body of a lifecycle action.
#[derive(Debug, Clone, PartialEq)]
pub enum LifecycleActionKind {
    /// Lifecycle control action — a flow-control directive.
    LifecycleControl(LifecycleControlAction),

    /// Communication action (TTS / sound / message / status line).
    Communication(CommunicationAction),

    /// Bespoke shell command.
    Shell(ShellAction),

    /// Darkmatter side-effect invoked by name.
    SideEffect(SideEffectAction),

    /// Invocation-local frontmatter mutations authored as one mapping.
    RuntimeSet(RuntimeSet),

    /// Read-only Darkmatter expression function invoked for its result.
    ExpressionFunction(ExpressionFunctionAction),

    /// A verb the parsing profile does not know, kept opaque under
    /// [`UnknownVerbPolicy::Ignore`](super::UnknownVerbPolicy::Ignore).
    ///
    /// Carries no operand: nothing about it is evaluated, scanned for shells,
    /// or checked for nested spans. A profile that rejects unknown verbs
    /// never produces it.
    Ignored(IgnoredAction),
}

/// An unknown verb kept by name and authored path only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IgnoredAction {
    /// The verb key as authored.
    pub verb: String,
    /// The authored path of the verb key (equal to the owning
    /// [`LifecycleAction::path`]).
    pub path: LifecyclePath,
}

/// A flow-control directive.
///
/// The first one reached **ends the entire event**, wherever it sits — at a
/// stack root or inside a nested `then`/`else` body — except `prep`, a
/// returning call after which the event continues
/// ([`LifecycleControlAction::returns`]). The "Where valid" matrix
/// that governs which events each variant may appear in is encoded in
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
        /// authored as the long-form `with:` parameter. Empty when `with:` was
        /// omitted or authored as `{}`.
        with: WithOverlay,
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
        /// Transient top-level frontmatter overlay for the fresh re-read of the
        /// same document, for that one transition only. Empty when `with:` was
        /// omitted or authored as `{}`.
        with: WithOverlay,
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

    /// `break` / `break: "reason"` — end the nearest enclosing loop.
    ///
    /// Firing outside every loop is a runtime error the host raises.
    Break {
        /// Optional reason recorded as the loop's outcome. An authored `""`
        /// is an explicit empty reason, distinct from `None`.
        reason: Option<Expr>,
        /// Exit code when the loop it ends is the root invocation's; authored
        /// `0..=255`, `0` when omitted.
        code: u8,
    },

    /// `prep: "@target.md"` — run another primitive to completion, then
    /// continue the event with the next item.
    ///
    /// The host runs the target; a target that did not return successfully
    /// fails the event.
    Prep {
        /// File reference expression resolving to the target. A literal is
        /// never empty.
        target: Expr,
        /// Frontmatter overlay for the target, with `proxy.with` semantics.
        /// Empty when `with:` was omitted or authored as `{}`.
        with: WithOverlay,
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
            Self::Break { .. } => "break",
            Self::Prep { .. } => "prep",
        }
    }

    /// Returns `true` for a directive that returns to the next item instead of
    /// ending its event: only `prep`.
    pub fn returns(&self) -> bool {
        matches!(self, Self::Prep { .. })
    }

    /// Returns `true` when this control action is permitted in the given
    /// event per the spec's "Where valid" matrix.
    ///
    /// Cancellation cleanup permits only `Stop`. Other events permit recovery
    /// directives, with `Skip` restricted to initialization. Provider-session
    /// requirements and recovery re-entry points are checked at runtime.
    pub fn is_valid_for(&self, event: LifecycleSignal) -> bool {
        if event == LifecycleSignal::Cancelled {
            return matches!(self, Self::Stop);
        }
        match self {
            Self::Skip => matches!(event, LifecycleSignal::Initialize),
            _ => true,
        }
    }
}

/// One value inside an authored `with:` or `set:` mapping, typed at parse time.
///
/// Leaves are [`Expr`] trees produced by the same rule that types every other
/// lifecycle action parameter, recursed through arrays and objects. Holding
/// expressions rather than raw JSON is what lets the handoff preserve a
/// whole-value span's type: `{{ true }}` stays a bool instead of collapsing to
/// the string `"true"`.
#[derive(Debug, Clone, PartialEq)]
pub enum WithValue {
    /// An authored scalar or interpolation-bearing string.
    Scalar(Expr),
    /// An authored YAML `null`. [`Expr`] has no null literal, and `with:`
    /// values keep their authored types, so null gets its own variant rather
    /// than forcing authors through `{{ null }}`.
    Null,
    /// An authored YAML sequence; elements follow the same rule.
    Array(Vec<WithValue>),
    /// An authored YAML mapping; values follow the same rule. Its keys are
    /// data, not property names — only the overlay's own top-level keys name
    /// target frontmatter properties, so only those are span-checked.
    Object(IndexMap<String, WithValue>),
    /// A whole-value `$( … )` assigned by a lifecycle `set`. Only a `set`
    /// mapping's top-level values take this form; a nested or mixed string,
    /// and every `with:` value, stays data.
    Shell(Box<SetShellValue>),
}

/// A lifecycle `set` value that runs a command when its action executes.
///
/// The command bytes are fixed once, at preflight, from early-binding values;
/// the result is produced each time the action executes. See
/// [`ResolvedShellValue`] for the two lifetimes. A value approved by its shape
/// ([`Self::template`]) runs its template instead, reading its placeholders
/// when the action executes.
#[derive(Debug, Clone, PartialEq)]
pub struct SetShellValue {
    /// The value as authored (trimmed), suffixes included.
    pub authored: String,
    /// The approved command bytes; `None` until preflight resolves them. An
    /// unresolved value never runs.
    pub resolved: Option<ResolvedShellValue>,
    /// The authored command as a typed template, recorded by preflight when
    /// it holds a placeholder. Its approval decides whether the value runs
    /// its bytes or its template.
    pub template: Option<super::shell_template::LifecycleShellTemplate>,
}

/// The message of a shell-value parse error, without the directive location
/// prefix the lifecycle diagnostic supplies itself.
pub fn shell_value_error_message(error: &ShellExpansionError) -> String {
    match error {
        ShellExpansionError::ParseDirective { message, .. }
        | ShellExpansionError::ExpressionEvaluation { message, .. } => message.clone(),
        other => other.to_string(),
    }
}

/// Why an authored `with:` mapping could not be typed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WithOverlayError {
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

/// Why an authored lifecycle `set:` mapping could not be typed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeSetError {
    /// A destination key is empty.
    EmptyKey,
    /// A destination key carries an interpolation span.
    DynamicKey(String),
    /// A value could not be typed by the shared action-value rule.
    Value {
        /// Path below `set`, e.g. `metadata.area` or `files[0]`.
        path: String,
        /// The reason from the shared rule.
        message: String,
    },
}

/// One lifecycle runtime mutation action.
///
/// The mapping is retained as one action rather than lowered into independent
/// writes. Values use the same recursive typed tree as `proxy.with`, preserving
/// native YAML values and whole-value expression types.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct RuntimeSet(IndexMap<String, WithValue>);

impl RuntimeSet {
    /// Type a lifecycle `set:` mapping.
    pub fn new(authored: IndexMap<String, serde_json::Value>) -> Result<Self, RuntimeSetError> {
        Self::new_with_order(authored, None)
    }

    /// Type a lifecycle `set:` mapping with order retained by the frontmatter parser.
    ///
    /// Entries are put in authored order before any check runs, so the error
    /// returned for a mapping with several invalid keys or values names the
    /// first authored one rather than the first in `authored`'s own order.
    pub fn new_with_order(
        mut authored: IndexMap<String, serde_json::Value>,
        authored_order: Option<&[String]>,
    ) -> Result<Self, RuntimeSetError> {
        let mut ordered: Vec<_> = authored_order
            .into_iter()
            .flatten()
            .filter_map(|key| authored.shift_remove_entry(key))
            .collect();
        ordered.extend(authored);
        for (key, _) in &ordered {
            if key.is_empty() {
                return Err(RuntimeSetError::EmptyKey);
            }
            if super::parse::is_dynamic_with_key(key) {
                return Err(RuntimeSetError::DynamicKey(key.clone()));
            }
        }
        let mut typed = IndexMap::with_capacity(ordered.len());
        for (key, value) in ordered {
            if let serde_json::Value::String(text) = &value
                && text.trim_start().starts_with("$(")
            {
                check_frontmatter_shell_value(&key, text).map_err(|error| RuntimeSetError::Value {
                    path: key.clone(),
                    message: shell_value_error_message(&error),
                })?;
                typed.insert(
                    key,
                    WithValue::Shell(Box::new(SetShellValue {
                        authored: text.trim().to_string(),
                        resolved: None,
                        template: None,
                    })),
                );
                continue;
            }
            let value = type_with_value(&value, &key).map_err(|error| match error {
                WithOverlayError::DynamicKey(_) => unreachable!("value typing does not inspect keys"),
                WithOverlayError::Value { path, message } => {
                    RuntimeSetError::Value { path, message }
                }
            })?;
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

    /// The typed value assigned to `key`.
    pub fn get(&self, key: &str) -> Option<&WithValue> {
        self.0.get(key)
    }

    /// Iterate destination keys and typed values in deterministic order.
    pub fn iter(&self) -> impl Iterator<Item = (&String, &WithValue)> {
        self.0.iter()
    }

    /// The whole-value shell assignments, in order.
    pub fn shell_values(&self) -> impl Iterator<Item = (&String, &SetShellValue)> {
        self.0.iter().filter_map(|(key, value)| match value {
            WithValue::Shell(shell) => Some((key, shell.as_ref())),
            _ => None,
        })
    }

    /// Mutable access to the whole-value shell assignments, for preflight to
    /// fix their command bytes.
    pub fn shell_values_mut(&mut self) -> impl Iterator<Item = (&String, &mut SetShellValue)> {
        self.0.iter_mut().filter_map(|(key, value)| match value {
            WithValue::Shell(shell) => Some((key, shell.as_mut())),
            _ => None,
        })
    }

    /// Whether any value runs a command.
    pub fn has_shell_values(&self) -> bool {
        self.shell_values().next().is_some()
    }
}

/// An authored `with:` mapping on `proxy` or `retry` — a transient top-level
/// frontmatter overlay for the transition's target (the proxy target, or the
/// fresh re-read of the same document for `retry`).
///
/// Values are held as typed [`WithValue`] trees. They are resolved once, when
/// the directive fires, against the source document's lifecycle context; this
/// type is the parse-time carrier, not the evaluated overlay.
///
/// Keys are static YAML strings by construction: [`WithOverlay::new`] is the
/// only constructor and rejects a key carrying an interpolation span, so a
/// downstream consumer never has to re-check for a dynamic key.
///
/// ## Examples
///
/// ```
/// use darkmatter::lifecycle::actions::{WithOverlay, WithOverlayError};
/// use indexmap::IndexMap;
///
/// let mut authored = IndexMap::new();
/// authored.insert("iteration".to_string(), serde_json::json!("{{ iteration }}"));
/// let with = WithOverlay::new(authored).expect("static key");
/// assert_eq!(with.len(), 1);
///
/// let mut dynamic = IndexMap::new();
/// dynamic.insert("{{ key }}".to_string(), serde_json::json!(1));
/// assert_eq!(
///     WithOverlay::new(dynamic).unwrap_err(),
///     WithOverlayError::DynamicKey("{{ key }}".to_string())
/// );
/// ```
#[derive(Debug, Clone, Default, PartialEq)]
pub struct WithOverlay(IndexMap<String, WithValue>);

impl WithOverlay {
    /// Type an overlay from an authored mapping.
    ///
    /// ## Errors
    ///
    /// [`WithOverlayError::DynamicKey`] when a top-level key carries a `{{ … }}`
    /// or `$( … )` span — `with:` keys name target frontmatter properties and
    /// are never interpolated, so a span in a key is an authoring error rather
    /// than a value to resolve later. [`WithOverlayError::Value`] when a value
    /// fails the shared action-value rule (e.g. a whole-value span that is not
    /// a parseable expression).
    ///
    /// Keys are checked before values, so an overlay with both faults reports
    /// the key.
    pub fn new(authored: IndexMap<String, serde_json::Value>) -> Result<Self, WithOverlayError> {
        for key in authored.keys() {
            if super::parse::is_dynamic_with_key(key) {
                return Err(WithOverlayError::DynamicKey(key.clone()));
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
    pub fn get(&self, key: &str) -> Option<&WithValue> {
        self.0.get(key)
    }

    /// Iterate the overlay's properties.
    ///
    /// Order is deterministic but is **not** the authored order: `serde_json`
    /// is built without `preserve_order`, so frontmatter parsing has already
    /// normalized a nested mapping's keys to sorted order before they reach
    /// here. Overlay semantics are per-key, so no consumer may depend on
    /// order for meaning.
    pub fn iter(&self) -> impl Iterator<Item = (&String, &WithValue)> {
        self.0.iter()
    }
}

/// Recurse the shared action-value rule through arrays and objects, tagging
/// each failure with its path below `with`.
fn type_with_value(value: &serde_json::Value, path: &str) -> Result<WithValue, WithOverlayError> {
    match value {
        serde_json::Value::Null => Ok(WithValue::Null),
        serde_json::Value::Array(items) => items
            .iter()
            .enumerate()
            .map(|(i, item)| type_with_value(item, &format!("{path}[{i}]")))
            .collect::<Result<Vec<_>, _>>()
            .map(WithValue::Array),
        serde_json::Value::Object(map) => map
            .iter()
            .map(|(k, v)| {
                type_with_value(v, &format!("{path}.{k}")).map(|typed| (k.clone(), typed))
            })
            .collect::<Result<IndexMap<_, _>, _>>()
            .map(WithValue::Object),
        scalar => super::action_shape::action_value_to_expr(scalar)
            .map(WithValue::Scalar)
            .map_err(|message| WithOverlayError::Value {
                path: path.to_string(),
                message: message.to_string(),
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
    /// The payload (`text`): words to speak, an effect name, a status line, a
    /// notification title — interpreted per channel.
    pub text: Expr,
}

/// Communication channels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommunicationChannel {
    /// TTS via host speech provider.
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
    /// Authored bytes retained for approval eligibility after interpolation.
    pub authored_command: Option<String>,
    /// The shell command expression. The spec example uses string literals,
    /// but the model accepts any expression so interpolation can compose the
    /// command at runtime.
    pub command: Expr,
    /// Message to emit when the command exits non-zero.
    pub on_error: Option<Expr>,
    /// `true` once pre-flight (C3) has stamped `command` and `on_error` with
    /// their resolved bytes. A command approved as those bytes runs them as
    /// they stand: anything the resolution inserted is data and is never
    /// interpolated again at event time. A command whose guarded value was
    /// missing at pre-flight keeps its authored text instead; its `template`
    /// has no bytes, so it runs only as that template.
    pub pre_resolved: bool,
    /// The authored `command` as a template typed in the scope of its
    /// enclosing guards, recorded by pre-flight when it holds a placeholder.
    /// Its approval decides whether the action runs the stamped bytes or the
    /// template.
    pub template: Option<super::shell_template::LifecycleShellTemplate>,
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
