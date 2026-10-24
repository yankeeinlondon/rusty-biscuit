//! Authoring errors the lifecycle parser and its static validators raise.
//!
//! Every variant names the authored path of the offending node so a host can
//! point its diagnostic at the right line. A host that keeps its own error
//! catalog maps each variant onto its twin field by field; the `Display` text
//! is part of that contract and is not reworded casually.

use std::path::PathBuf;

use thiserror::Error;

use crate::markdown::compose::expression::ParseError;

/// Lower-layer cause of an `action_value_to_expr` failure.
///
/// Carried as the optional typed `#[source]` of
/// [`LifecycleError::LifecycleActionInvalidLongForm`] so a handler can recover
/// the concrete [`ParseError`] through [`std::error::Error::source`] instead of
/// re-parsing the flattened message, whether the failing expression was a
/// whole value or one span of mixed text. The
/// [`Invalid`](ActionExprError::Invalid) arm carries the genuinely-prose
/// rejections (a null, an unsupported number, or object/array scalar data)
/// that never had a typed lower cause.
///
/// `Display` is byte-identical to the strings `action_value_to_expr` built
/// before the cause was typed — typing an error is not a licence to reword it.
#[derive(Error, Debug)]
pub enum ActionExprError {
    /// A whole-value `{{ … }}` interpolation span did not parse.
    ///
    /// Boxed to keep `ActionExprError` (and thus the unboxed
    /// `LifecycleActionInvalidLongForm` source slot) small — [`ParseError`]
    /// is the heavy member. The unboxed enum keeps the field downcastable
    /// while the boxed payload keeps host error enums under
    /// `clippy::result_large_err`.
    #[error("whole-value expression is not valid: {0}")]
    Parse(#[from] Box<ParseError>),
    /// A `{{ … }}` span inside mixed text (`"status {{ fm. }}"`) did not
    /// parse. The text stays a literal template; only its spans' syntax is
    /// checked, so nothing is evaluated.
    #[error("interpolation span `{{{{ {expression} }}}}` is not valid: {source}")]
    SpanParse {
        /// The span's inner expression text, as authored.
        expression: String,
        /// The Darkmatter parse failure.
        #[source]
        source: Box<ParseError>,
    },
    /// A scalar shape that cannot become an action expression at all.
    #[error("{0}")]
    Invalid(String),
}

/// A lifecycle authoring error: the stack, an item, or an action does not
/// follow the lifecycle grammar.
///
/// Raised before anything runs. Hosts that keep a diagnostic catalog convert
/// each variant into a same-named twin carrying the same fields and the same
/// `Display` text.
#[derive(Error, Debug)]
pub enum LifecycleError {
    /// A lifecycle frontmatter property has an invalid value.
    #[error("invalid lifecycle property `{property}`: {message}")]
    LifecycleInvalid {
        /// The frontmatter property that failed to parse (e.g. `"success"`).
        property: String,
        /// The underlying deserialization error message.
        message: String,
        /// The source file whose frontmatter contained the invalid property.
        source_file: PathBuf,
        /// The unknown field name extracted from the serde error (e.g. `"speak"`).
        unknown_field: Option<String>,
        /// The valid field names, when the error is an unknown field.
        expected_fields: Vec<String>,
    },

    /// One lifecycle dictionary holds both `say` and its alias `speak`.
    ///
    /// The two keys name the same canonical verb, and a dictionary holds each
    /// canonical verb at most once. Carries the dictionary's authored path.
    #[error("lifecycle dictionary `{0}` has both `say` and `speak`; they are the same verb, so use one")]
    LifecycleSayConflict(String),

    /// A lifecycle `effect` action names an unknown sound effect.
    ///
    /// Carries the action's authored path and the name.
    #[error("lifecycle property `{0}` references unknown sound effect `{1}`")]
    LifecycleUnknownEffect(String, String),

    /// A single-pass lifecycle expression authors a `{{ … }}` span inside a
    /// quoted string literal.
    ///
    /// A whole-value communication field, a `when`/`while`/`until` predicate, a
    /// stack action operand, and a `proxy … with` value are each evaluated
    /// exactly once, so the nested span can never interpolate: it would reach
    /// the side effect as raw text.
    #[error(
        "lifecycle property `{property}` nests `{nested}` inside the string literal {literal} ({source_path})",
        source_path = biscuit_file::to_portable_string(source_path)
    )]
    LifecycleNestedSpanInLiteral {
        /// The prompt file whose lifecycle frontmatter holds the literal.
        source_path: PathBuf,
        /// Authored lifecycle path, e.g. `success[0].say.text` or
        /// `finalize[0].when`.
        property: String,
        /// The authored literal, quotes included.
        literal: String,
        /// The nested span as authored, e.g. `{{ctx.repo_name}}`.
        nested: String,
        /// The complete bare-expression rewrite from Darkmatter's lint, when
        /// one can be proven equivalent. Illustrative: it carries no YAML
        /// quoting and no `{{ }}` wrapper.
        suggestion: Option<String>,
    },

    /// A lifecycle expression makes a call the design-time check proves
    /// wrong (category 5): the same verdict that rejects the call in the body,
    /// before any lifecycle event or expression runs, in every branch.
    #[error(
        "lifecycle `{property}` calls a function with an argument that can never bind ({source_path}): {source}",
        source_path = biscuit_file::to_portable_string(source_path)
    )]
    LifecycleCallRejected {
        /// The prompt file whose lifecycle frontmatter holds the call.
        source_path: PathBuf,
        /// Authored lifecycle path of the surface, e.g. `success[0].say.text`
        /// or `finalize[0].when`.
        property: String,
        /// The expression holding the call, as authored.
        expression: String,
        /// Always [`ExpressionError::CallCheck`](crate::markdown::compose::expression::ExpressionError::CallCheck).
        #[source]
        source: Box<crate::markdown::compose::expression::ExpressionError>,
    },

    /// A frontmatter document uses removed lifecycle grammar.
    ///
    /// Covers the retired spellings the lifecycle parser sees: the `stack:`
    /// key, the `action:` item aggregator (including the `{action: verb, …}`
    /// discriminator map), `say_first`, and the communication
    /// `message`/`sound`/`route` parameters. There is no acceptance period:
    /// the old spelling never runs.
    #[error(
        "`{path}` uses the removed `{removed}` grammar ({source_path}); {replacement}",
        source_path = biscuit_file::to_portable_string(source_path)
    )]
    RemovedGrammar {
        /// The document that holds the removed spelling.
        source_path: PathBuf,
        /// The authored path of the removed spelling, e.g. `start.stack`,
        /// `failure[0].say.message`, or `loop.while`.
        path: String,
        /// The removed spelling, e.g. `stack`, `say_first`, `_loop_count`.
        removed: String,
        /// What to write instead.
        replacement: String,
    },

    /// A lifecycle stack, item, or body has an invalid shape (wrong type, an
    /// empty explicit branch, an orphan `then`/`else`, a sixth nesting level,
    /// a misplaced `no_error`, and so on).
    #[error(
        "invalid lifecycle stack item in `{property}` ({source_path}): {message}",
        source_path = biscuit_file::to_portable_string(source_path)
    )]
    LifecycleStackInvalidShape {
        /// The prompt file whose lifecycle frontmatter held the malformed stack.
        source_path: PathBuf,
        /// The authored path of the offending node (e.g. `start[0].then`).
        property: String,
        /// Human-readable description of the shape problem.
        message: String,
    },

    /// A lifecycle `when:` condition failed to parse.
    ///
    /// The typed sibling of [`LifecycleStackInvalidShape`]: it shares that
    /// variant's `Display` (`message` keeps the prose) and adds the
    /// recoverable source.
    ///
    /// [`LifecycleStackInvalidShape`]: LifecycleError::LifecycleStackInvalidShape
    #[error(
        "invalid lifecycle stack item in `{property}` ({source_path}): {message}",
        source_path = biscuit_file::to_portable_string(source_path)
    )]
    LifecycleWhenExpressionInvalid {
        /// The prompt file whose lifecycle frontmatter held the condition.
        source_path: PathBuf,
        /// The authored path of the offending action or node (e.g. `start[1].shell`).
        property: String,
        /// Human-readable description of the parse failure.
        message: String,
        /// The typed expression parse failure.
        #[source]
        source: ParseError,
    },

    /// A bare lifecycle action list item failed to parse (an empty verb).
    #[error(
        "invalid lifecycle action `{raw}` in `{property}` ({source_path}): {message}",
        source_path = biscuit_file::to_portable_string(source_path)
    )]
    LifecycleActionInvalidShortForm {
        /// The prompt file whose lifecycle frontmatter held the action.
        source_path: PathBuf,
        /// The authored path of the offending action or node (e.g. `start[1].shell`).
        property: String,
        /// The raw short-form string exactly as authored.
        raw: String,
        /// Human-readable parse failure description.
        message: String,
    },

    /// A long-form lifecycle action supplied an unknown field, wrong value
    /// type, or missing required parameter.
    #[error(
        "invalid lifecycle action `{action}` in `{property}` ({source_path}): {message}",
        source_path = biscuit_file::to_portable_string(source_path)
    )]
    LifecycleActionInvalidLongForm {
        /// The prompt file whose lifecycle frontmatter held the action.
        source_path: PathBuf,
        /// The authored path of the offending action or node (e.g. `start[1].shell`).
        property: String,
        /// The action verb (e.g. `"set_frontmatter"`, `"shell"`).
        action: String,
        /// Human-readable description of the long-form problem.
        message: String,
        /// Typed cause when the failure came from converting an action value
        /// into an expression (`action_value_to_expr`). `None` for the
        /// shape failures — missing parameter, unknown key, wrong value type —
        /// that never had a lower cause.
        #[source]
        source: Option<ActionExprError>,
    },

    /// A lifecycle action verb is not recognized at parse time.
    #[error(
        "unknown lifecycle action `{verb}` in `{property}` ({source_path}){rewrite}",
        source_path = biscuit_file::to_portable_string(source_path)
    )]
    LifecycleUnknownVerb {
        /// The prompt file whose lifecycle frontmatter held the action.
        source_path: PathBuf,
        /// The authored path of the offending action or node (e.g. `start[1].shell`).
        property: String,
        /// The unrecognized verb.
        verb: String,
        /// Did-you-mean rewrite, empty when no suggestion is available.
        rewrite: String,
    },

    /// A positional lifecycle action received a direct YAML map value.
    ///
    /// Object-valued side-effect arguments must be passed through a whole-value
    /// `{{ … }}` interpolation span; direct nested YAML objects are rejected.
    #[error(
        "lifecycle action `{verb}` in `{property}` received an object value where a scalar or array is expected; \
         pass object data through a whole-value `{{{{ ... }}}}` interpolation ({source_path})",
        source_path = biscuit_file::to_portable_string(source_path)
    )]
    LifecycleObjectDataThroughInterpolationPositional {
        /// The prompt file whose lifecycle frontmatter held the action.
        source_path: PathBuf,
        /// The authored path of the offending action or node (e.g. `start[1].shell`).
        property: String,
        /// The action verb.
        verb: String,
    },

    /// The removed positional lifecycle `set: [key, value]` form was used.
    #[error(
        "lifecycle `{property}.{path}` uses the removed positional `set` form ({source_path}); \
         use `set: {{property: value}}`",
        source_path = biscuit_file::to_portable_string(source_path)
    )]
    LifecycleSetPositionalRemoved {
        /// The prompt file whose lifecycle frontmatter held the action.
        source_path: PathBuf,
        /// The authored path of the node that holds the `set` verb (e.g.
        /// `start[0]`, `start` for a root dictionary, or a task's
        /// `tasks[0].side_effect`).
        property: String,
        /// The `set`-rooted path below `property`.
        path: String,
    },

    /// Lifecycle `set:` received a non-mapping payload.
    #[error(
        "lifecycle `{property}.{path}` must be a mapping, got {actual} ({source_path}); \
         use `set: {{property: value}}`",
        source_path = biscuit_file::to_portable_string(source_path)
    )]
    LifecycleSetNotMapping {
        /// The prompt file whose lifecycle frontmatter held the action.
        source_path: PathBuf,
        /// The authored path of the node that holds the `set` verb (e.g.
        /// `start[0]`, `start` for a root dictionary, or a task's
        /// `tasks[0].side_effect`).
        property: String,
        /// The `set`-rooted path below `property`.
        path: String,
        /// The authored payload type or named unsupported whole-value form.
        actual: String,
    },

    /// A lifecycle `set:` destination key is empty or dynamic.
    #[error(
        "lifecycle `{property}.{path}` has invalid destination key `{key}`: {message} \
         ({source_path}); use literal non-empty keys in `set: {{property: value}}`",
        source_path = biscuit_file::to_portable_string(source_path)
    )]
    LifecycleSetInvalidKey {
        /// The prompt file whose lifecycle frontmatter held the action.
        source_path: PathBuf,
        /// The authored path of the node that holds the `set` verb.
        property: String,
        /// The deepest representable `set` path.
        path: String,
        /// The invalid key, verbatim.
        key: String,
        /// Why the key is invalid.
        message: String,
    },

    /// A key/value lifecycle action parameter received a direct YAML map value.
    ///
    /// Object-valued side-effect arguments must be passed through a whole-value
    /// `{{ … }}` interpolation span; direct nested YAML objects are rejected.
    #[error(
        "lifecycle action `{verb}` parameter `{param}` in `{property}` received an object value where a scalar is expected; \
         pass object data through a whole-value `{{{{ ... }}}}` interpolation ({source_path})",
        source_path = biscuit_file::to_portable_string(source_path)
    )]
    LifecycleObjectDataThroughInterpolationParameter {
        /// The prompt file whose lifecycle frontmatter held the action.
        source_path: PathBuf,
        /// The authored path of the offending action or node (e.g. `start[1].shell`).
        property: String,
        /// The action verb.
        verb: String,
        /// The parameter name.
        param: String,
    },

    /// A `with:` — a task's, or a `proxy`, `retry`, or `prep` directive's —
    /// was authored with something other than a YAML mapping (including
    /// `null`).
    #[error(
        "`{at}` must be a mapping, got {actual} ({source_path})",
        source_path = biscuit_file::to_portable_string(source_path),
        at = with_property_path(property, path)
    )]
    LifecycleWithNotMapping {
        /// The file that authored the `with`.
        source_path: PathBuf,
        /// The authored path of the `with`'s owner: the action (e.g.
        /// `failure[0].retry`) or the task (`sequence[1]`; empty for the task
        /// an external `kind: task` file is).
        property: String,
        /// The `with`-rooted path below the action (`with`). Joined to
        /// `property` for the rendered property path.
        path: String,
        /// The JSON type name of the authored value.
        actual: String,
    },

    /// A `with` overlay was supplied as a single whole-mapping interpolation
    /// (`with: "{{ payload }}"`) rather than explicit keys.
    #[error(
        "`{at}` cannot be supplied as a whole-mapping interpolation `{raw}` ({source_path})",
        source_path = biscuit_file::to_portable_string(source_path),
        at = with_property_path(property, path)
    )]
    LifecycleWithWholeMapping {
        /// The file that authored the `with`.
        source_path: PathBuf,
        /// The authored path of the `with`'s owner (action or task).
        property: String,
        /// The `with`-rooted path below the action.
        path: String,
        /// The authored string, verbatim.
        raw: String,
    },

    /// A `with` overlay key carries an interpolation span. Keys name target
    /// frontmatter properties and are never interpolated.
    #[error(
        "`{at}` has a dynamic key `{key}`; `with:` keys must be static strings ({source_path})",
        source_path = biscuit_file::to_portable_string(source_path),
        at = with_property_path(property, path)
    )]
    LifecycleWithDynamicKey {
        /// The file that authored the `with`.
        source_path: PathBuf,
        /// The authored path of the `with`'s owner (action or task).
        property: String,
        /// The `with`-rooted path below the action. It gains a trailing
        /// `.{key}` segment only when the key is a safe path segment — a
        /// dotted path built from an unrepresentable key would point at a
        /// property that does not exist.
        path: String,
        /// The offending key, verbatim.
        key: String,
    },

    /// A `with` overlay was authored on an action that does not take one.
    ///
    /// Only `proxy`, `retry`, and `prep` accept `with`.
    #[error(
        "lifecycle action `{verb}` in `{property}` does not accept a `with` overlay; \
         `with` is only valid on `proxy`, `retry`, and `prep` ({source_path})",
        source_path = biscuit_file::to_portable_string(source_path)
    )]
    LifecycleWithUnsupported {
        /// The prompt file whose lifecycle frontmatter held the action.
        source_path: PathBuf,
        /// The authored path of the action (e.g. `failure[0].resume`).
        property: String,
        /// The action verb that received the overlay.
        verb: String,
    },

    /// A positional lifecycle action received the wrong number of arguments.
    #[error(
        "lifecycle action `{verb}` in `{property}` has the wrong arity ({source_path}): {message}",
        source_path = biscuit_file::to_portable_string(source_path)
    )]
    LifecycleWrongArity {
        /// The prompt file whose lifecycle frontmatter held the action.
        source_path: PathBuf,
        /// The authored path of the offending action or node (e.g. `start[1].shell`).
        property: String,
        /// The action verb.
        verb: String,
        /// Human-readable arity description.
        message: String,
    },

    /// The short-form lifecycle action grammar `verb(args)` has been removed.
    ///
    /// The `rewrite` field carries the `verb: value` equivalent for the
    /// did-you-mean message.
    #[error(
        "short-form lifecycle action `{raw}` in `{property}` has been removed ({source_path}).\n\n\
         Use the short form (`verb: value`) or the long form instead: {rewrite}",
        source_path = biscuit_file::to_portable_string(source_path)
    )]
    LifecycleShortFormRemoved {
        /// The prompt file whose lifecycle frontmatter held the action.
        source_path: PathBuf,
        /// The authored path of the offending action or node (e.g. `start[1].shell`).
        property: String,
        /// The raw short-form string exactly as authored.
        raw: String,
        /// Positional-form did-you-mean rewrite.
        rewrite: String,
    },

    /// A variadic expression-function action was authored in long form.
    ///
    /// Variadic functions (`and`, `or`) have no deterministic named-parameter
    /// mapping, so they reject the `verb: { param: value }` shape.
    #[error(
        "lifecycle expression function `{verb}` in `{property}` does not support long form; \
         use the positional array form (`{verb}: [...]`) ({source_path})",
        source_path = biscuit_file::to_portable_string(source_path)
    )]
    LifecycleExpressionFunctionKeyValueUnsupported {
        /// The prompt file whose frontmatter held the action.
        source_path: PathBuf,
        /// The authored path of the offending action or node (e.g. `start[1].shell`).
        property: String,
        /// The variadic expression-function verb.
        verb: String,
    },

    /// An action was used in a forbidden event: a shell in `initialize`, or
    /// a lifecycle control outside its placement matrix.
    #[error(
        "lifecycle action `{action}` is not valid in the `{event}` event ({source_path})",
        source_path = biscuit_file::to_portable_string(source_path)
    )]
    LifecycleActionPlacement {
        /// The prompt file whose lifecycle frontmatter held the action.
        source_path: PathBuf,
        /// The authored path of the offending action or node (e.g. `start[1].shell`).
        property: String,
        /// The rejected action verb (e.g. `"skip"`, `"retry"`).
        action: String,
        /// The event name the action was placed in (e.g. `"start"`).
        event: String,
    },

    /// A document run by a host that renamed a Darkmatter event authors the
    /// Darkmatter spelling (`composed:` where the host says `start:`).
    #[error(
        "lifecycle event `{authored}` is named `{replacement}` by this host; write `{replacement}:` instead ({source_path})",
        source_path = biscuit_file::to_portable_string(source_path)
    )]
    LifecycleRenamedEvent {
        /// The prompt file whose frontmatter holds the key.
        source_path: PathBuf,
        /// The authored Darkmatter spelling (e.g. `"composed"`).
        authored: String,
        /// The host's spelling of the same event (e.g. `"start"`).
        replacement: String,
    },

    /// One lifecycle dictionary holds more than one flow-control directive.
    #[error(
        "lifecycle dictionary `{property}` has more than one flow-control directive; \
         at most one is allowed per dictionary ({source_path})",
        source_path = biscuit_file::to_portable_string(source_path)
    )]
    LifecycleMultipleLifecycleActions {
        /// The prompt file whose lifecycle frontmatter held the dictionary.
        source_path: PathBuf,
        /// The authored path of the dictionary (e.g. `start[0]`).
        property: String,
    },

    /// An unconditional flow-control directive is followed by another item in
    /// the same list.
    #[error(
        "flow-control directive `{property}` must be the last item in its list ({source_path})",
        source_path = biscuit_file::to_portable_string(source_path)
    )]
    LifecycleActionOrder {
        /// The prompt file whose lifecycle frontmatter held the list.
        source_path: PathBuf,
        /// The authored path of the directive (e.g. `success[0].stop`).
        property: String,
    },

    /// A lifecycle action received the wrong number or type of arguments,
    /// such as a negative `retry` count or a `proxy` without its target.
    #[error(
        "lifecycle action `{action}` in `{property}` has invalid arguments ({source_path}): {message}",
        source_path = biscuit_file::to_portable_string(source_path)
    )]
    LifecycleInvalidArgs {
        /// The prompt file whose lifecycle frontmatter held the action.
        source_path: PathBuf,
        /// The authored path of the offending action or node (e.g. `start[1].shell`).
        property: String,
        /// The action verb (e.g. `"retry"`, `"proxy"`).
        action: String,
        /// Human-readable description of the argument problem.
        message: String,
    },
}

impl LifecycleError {
    /// The design-time verdict of a call this error rejects, when it is a
    /// [`LifecycleError::LifecycleCallRejected`].
    pub fn rejected_call(&self) -> Option<&crate::markdown::compose::expression::CallCheck> {
        match self {
            Self::LifecycleCallRejected { source, .. } => match source.as_ref() {
                crate::markdown::compose::expression::ExpressionError::CallCheck(check) => Some(check),
                _ => None,
            },
            _ => None,
        }
    }
}

/// The rendered property path of a `with`: `{property}.{path}`, or `path`
/// alone for an owner with no path of its own.
#[doc(hidden)]
pub fn with_property_path(property: &str, path: &str) -> String {
    if property.is_empty() { path.to_string() } else { format!("{property}.{path}") }
}

/// Why an event-time lifecycle expression failed.
///
/// The `Err` type of an executor's expression layer: `when:` guards,
/// interpolation, control-argument evaluation, and side-effect argument
/// evaluation all fail through it. Every arm reaches the envelope boundary
/// ([`LifecycleErrorInfo::from_expr_error_with`]), so the typed value is
/// projected once rather than pre-flattened into prose at each raise site.
///
/// The lower-layer arms are `transparent`: they *replace* the concrete error
/// in the chain rather than adding a hop to it, so a handler recovers the stage
/// by downcasting to this enum and matching its arm.
///
/// The heavy members are boxed to keep the enum (and the `Result`s carrying it)
/// small — `clippy::result_large_err` fires on these hot helpers otherwise.
///
/// [`LifecycleErrorInfo::from_expr_error_with`]: super::context::LifecycleErrorInfo::from_expr_error_with
#[derive(Debug, Error)]
pub enum LifecycleExprError {
    /// A parsed expression could not be evaluated.
    #[error(transparent)]
    Evaluate(#[from] Box<crate::markdown::compose::expression::ExpressionError>),

    /// Event-time interpolation (subtree compose) raised.
    #[error(transparent)]
    Compose(#[from] Box<crate::markdown::MarkdownError>),

    /// A failure the expression layer describes itself, with no lower-layer
    /// error in hand: a control argument of the wrong shape, or template syntax
    /// in a `proxy.with` value that becomes the target's lifecycle
    /// configuration.
    #[error("{0}")]
    Prose(String),

    /// The request's file-resolution context failed validation.
    #[error(transparent)]
    Context(#[from] crate::markdown::compose::ContextBuildError),
}

impl LifecycleExprError {
    /// Build a [`Self::Prose`] arm from anything string-shaped.
    pub fn prose(message: impl Into<String>) -> Self {
        LifecycleExprError::Prose(message.into())
    }
}
