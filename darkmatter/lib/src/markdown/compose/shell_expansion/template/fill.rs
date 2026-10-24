//! Filling a template: each placeholder is evaluated once, its original value
//! is checked against the hole's validator, and the converted text joins the
//! literal runs of its argument, so every authored argument stays exactly one
//! process argument whatever the value holds.
//!
//! The order is fixed: evaluate, refuse a missing value (`null`), a container
//! (array or object), or an embedded NUL, validate the original value without
//! coercion against the declaration and then against the guard refinement
//! the site was approved under, convert it with [`scalar_string`], then
//! concatenate. A value is never parsed for quotes, whitespace, operators, or
//! further placeholders. An expression that fills several holes of one
//! command is evaluated once, and that one checked value fills each of them.
//!
//! [`fill_template`] is shared by every adapter that runs a template:
//! Darkmatter's own `::shell`, `::shell-block`, and frontmatter `$( … )`
//! stages, and a host's runners (for example Claudine's sequence and lifecycle
//! shells). A host that validates late values at a barrier before any command
//! runs calls [`resolve_late_values`] there and hands each site's
//! [`SiteValues`] to [`fill_template`], so no placeholder is evaluated twice.
//! A [`HoleTiming::Guarded`] hole is never part of that barrier: it is read
//! by [`fill_template`] when its guarded command is reached.

use super::{ArgPart, CommandTemplate, FilledChain, FilledSegment, HoleTiming, HoleType, HoleValidator, Placeholder};
use crate::markdown::compose::expression::{EvaluationLookup, ExpressionError, evaluate, parse, scalar_string};
use crate::markdown::compose::shell_expansion::types::{ShellCommandOrigin, ShellExpansionError};
use biscuit_terminal::errors::SourceContext;
use serde_json::Value;
use std::collections::{BTreeMap, HashMap};

/// Evaluates a placeholder's expression against the caller's current state.
///
/// Implemented for any `Fn(&str) -> Result<Value, ExpressionError>`, so a host
/// can pass a closure over its own evaluation, and by [`LookupEvaluator`] for
/// any Darkmatter [`EvaluationLookup`]: an `EvaluationSession` over a host's
/// binding view, or an `EffectiveState` built from a JSON scope with
/// `EffectiveStateBuilder`.
pub trait HoleEvaluator {
    /// The value of `expression`, the trimmed text inside `{{ }}`.
    ///
    /// ## Errors
    ///
    /// The expression's own parse or evaluation error. A read of an absent
    /// property is not an error: it is `Ok(Value::Null)`.
    fn evaluate(&self, expression: &str) -> Result<Value, ExpressionError>;
}

impl<F> HoleEvaluator for F
where
    F: Fn(&str) -> Result<Value, ExpressionError>,
{
    fn evaluate(&self, expression: &str) -> Result<Value, ExpressionError> {
        self(expression)
    }
}

/// Parses each expression and evaluates it with Darkmatter's expression
/// engine against `lookup`.
pub struct LookupEvaluator<'a, L: EvaluationLookup> {
    lookup: &'a L,
}

impl<'a, L: EvaluationLookup> LookupEvaluator<'a, L> {
    pub fn new(lookup: &'a L) -> Self {
        Self { lookup }
    }
}

impl<L: EvaluationLookup> HoleEvaluator for LookupEvaluator<'_, L> {
    fn evaluate(&self, expression: &str) -> Result<Value, ExpressionError> {
        let expr = parse(expression).map_err(|error| ExpressionError::Parse(error.to_string()))?;
        evaluate(&expr, self.lookup)
    }
}

/// Why a placeholder could not fill its argument. None of these carries the
/// value, which may be a secret.
#[derive(Debug, Clone, thiserror::Error)]
pub enum HoleError {
    /// The expression evaluated to `null` (an absent property included).
    #[error("'{expression}' is missing")]
    Missing { expression: String },
    /// The value cannot become one argument of the hole's type.
    #[error("the value of '{expression}' {reason} (declared type: {declared_type})")]
    Rejected {
        expression: String,
        declared_type: String,
        reason: RejectReason,
    },
    /// The expression failed to parse or evaluate.
    #[error("'{expression}' could not be evaluated: {source}")]
    Evaluation {
        expression: String,
        #[source]
        source: Box<ExpressionError>,
    },
}

/// Why a present value was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RejectReason {
    /// An array or an object: never one argument, even under a `string` rule.
    NotScalar,
    /// A string holding a NUL byte, which no process argument can carry.
    EmbeddedNul,
    /// The declared type or one of its constraints rejected the original
    /// value. Nothing is coerced first: a number never passes `alphanumeric`.
    DeclaredType,
    /// The value passes its declaration but not the narrower type an
    /// enclosing guard established, under which the command was approved:
    /// the value changed after the guard, or the guard was not what selected
    /// the command.
    GuardRefinement,
}

impl std::fmt::Display for RejectReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::NotScalar => "is an array or object, not a string, number, or boolean",
            Self::EmbeddedNul => "contains a NUL byte",
            Self::DeclaredType => "does not satisfy its declared type",
            Self::GuardRefinement => "does not satisfy the type its enclosing guard narrowed it to",
        })
    }
}

impl HoleError {
    /// The placeholder expression the error is about.
    pub fn expression(&self) -> &str {
        match self {
            Self::Missing { expression } | Self::Rejected { expression, .. } | Self::Evaluation { expression, .. } => {
                expression
            }
        }
    }

    /// This error as the typed shell error a composition reports for the
    /// command `command` (its authored text) at `origin`.
    ///
    /// A missing value is [`ShellExpansionError::MissingValue`], a refused
    /// value [`ShellExpansionError::TemplateValueRejected`], and an
    /// evaluation failure [`ShellExpansionError::ExpressionEvaluation`] with
    /// the typed cause kept.
    pub fn into_shell_error(self, ctx: &SourceContext, command: &str, origin: &ShellCommandOrigin) -> ShellExpansionError {
        match self {
            Self::Missing { expression } => ShellExpansionError::MissingValue {
                ctx: Box::new(ctx.clone()),
                command: command.to_string(),
                expression,
                origin: origin.clone(),
            },
            Self::Rejected {
                expression,
                declared_type,
                reason,
            } => ShellExpansionError::TemplateValueRejected {
                ctx: Box::new(ctx.clone()),
                command: command.to_string(),
                placeholder: expression.into(),
                declared_type: declared_type.into(),
                reason,
                origin: origin.clone(),
            },
            Self::Evaluation { expression, source } => ShellExpansionError::ExpressionEvaluation {
                ctx: Box::new(ctx.clone()),
                origin: origin.clone(),
                message: format!("placeholder '{expression}' in '{command}': {source}"),
                cause: source,
            },
        }
    }
}

/// The converted text of each late hole of one command site, by the hole's
/// index among the template's placeholders in authored order.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SiteValues {
    holes: BTreeMap<usize, LateHole>,
}

/// One late hole's barrier result.
#[derive(Debug, Clone, PartialEq, Eq)]
struct LateHole {
    text: String,
    /// Whether the value also passed the hole's guard refinement. The barrier
    /// runs whether or not the guarded command is reached, so a failure is
    /// reported only by [`fill_template`], when the command is.
    refinement_holds: bool,
}

impl SiteValues {
    /// The validated, converted text of hole `index`, when it was resolved.
    pub fn get(&self, index: usize) -> Option<&str> {
        self.holes.get(&index).map(|hole| hole.text.as_str())
    }

    pub fn is_empty(&self) -> bool {
        self.holes.is_empty()
    }
}

/// One task execution's document late-value snapshot. Clones share the barrier's
/// values; a new task must allocate a fresh snapshot.
#[derive(Debug, Clone, Default)]
pub struct DocumentShellValues(std::sync::Arc<std::sync::Mutex<BTreeMap<String, SiteValues>>>);

impl DocumentShellValues {
    pub(crate) fn key(candidate: &super::ShellCandidate) -> String {
        format!("{:?}:{:?}:{:?}", candidate.site.source_file, candidate.site.origin, candidate.template)
    }

    /// Retain a host adapter's validated late inputs for the same authored site.
    pub fn insert(&self, candidate: &super::ShellCandidate, values: SiteValues) {
        self.0.lock().expect("document shell values poisoned").insert(Self::key(candidate), values);
    }

    /// Read the barrier inputs for the same authored site.
    pub fn get(&self, candidate: &super::ShellCandidate) -> Option<SiteValues> {
        self.0.lock().expect("document shell values poisoned").get(&Self::key(candidate)).cloned()
    }

    /// Whether this barrier retained any deferred shell placeholder values.
    pub fn is_empty(&self) -> bool {
        self.0.lock().expect("document shell values poisoned").values().all(SiteValues::is_empty)
    }

    pub(crate) fn identity(&self) -> String {
        format!("{:?}", *self.0.lock().expect("document shell values poisoned"))
    }
}

/// Late values validated at a barrier, by host-chosen site id.
///
/// Only [`HoleTiming::Deferred`] holes are kept: every other hole is still
/// evaluated by [`fill_template`] immediately before its command, against the
/// state current then.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LateValues {
    sites: HashMap<String, SiteValues>,
}

impl LateValues {
    /// The values resolved for site `id`.
    pub fn site(&self, id: &str) -> Option<&SiteValues> {
        self.sites.get(id)
    }

    pub fn is_empty(&self) -> bool {
        self.sites.values().all(SiteValues::is_empty)
    }
}

/// A late value that failed at the barrier, with the site it belongs to.
#[derive(Debug, Clone, thiserror::Error)]
#[error("late value for shell command site '{site}': {error}")]
pub struct LateValueError {
    pub site: String,
    #[source]
    pub error: HoleError,
}

/// Evaluates and validates every [`HoleTiming::Deferred`] hole of `sites`
/// (each a host-chosen site id and its typed template), so a step fails
/// before any of its commands runs when one of its late values is missing or
/// invalid.
///
/// A [`HoleTiming::Guarded`] hole is skipped: its guard may not select its
/// command. A deferred hole's guard refinement is likewise checked only when
/// [`fill_template`] uses the value.
///
/// ## Errors
///
/// The first failing hole, in site order then authored order.
pub fn resolve_late_values<'a, I>(sites: I, evaluator: &dyn HoleEvaluator) -> Result<LateValues, LateValueError>
where
    I: IntoIterator<Item = (&'a str, &'a CommandTemplate)>,
{
    let mut late = LateValues::default();
    for (id, template) in sites {
        let mut values = SiteValues::default();
        for (index, placeholder) in template.placeholders().enumerate() {
            if !matches!(placeholder.timing, HoleTiming::Deferred { .. }) {
                continue;
            }
            let checked = check_hole(placeholder, evaluator).map_err(|error| LateValueError {
                site: id.to_string(),
                error,
            })?;
            values.holes.insert(
                index,
                LateHole {
                    text: scalar_string(&checked.value),
                    refinement_holds: checked.refinement_holds,
                },
            );
        }
        late.sites.insert(id.to_string(), values);
    }
    Ok(late)
}

/// Fills every hole of `template`, producing one string per argument.
///
/// A hole whose index `late` holds takes that already validated text; every
/// other hole is evaluated through `evaluator`, now, once per distinct
/// expression. Every value is checked against the hole's declaration and its
/// guard refinement. Every hole of every segment is filled before this
/// returns, so a chain whose `&&`/`||` would skip a segment still has that
/// segment's values checked.
///
/// The returned chain has not been checked against any blacklist; call
/// [`check_filled_chain`](crate::markdown::compose::shell_expansion::check_filled_chain)
/// before the first segment starts.
///
/// ## Errors
///
/// The first hole, in authored order, that is missing, refused, or fails to
/// evaluate. A value that passes its declaration but not its guard
/// refinement is [`RejectReason::GuardRefinement`].
///
/// ## Examples
///
/// ```
/// use darkmatter::markdown::compose::expression::ExpressionError;
/// use darkmatter::markdown::compose::shell_expansion::template::{fill_template, parse_authored};
/// use serde_json::{Value, json};
///
/// let template = parse_authored("tool --out {{ out }}/build {{ name }}").unwrap();
/// let scope = json!({ "out": "a b", "name": "x; rm -rf ~" });
/// let evaluate = |expression: &str| -> Result<Value, ExpressionError> {
///     Ok(scope.get(expression).cloned().unwrap_or(Value::Null))
/// };
/// let chain = fill_template(&template, &evaluate, None).unwrap();
/// assert_eq!(chain.segments[0].args, ["--out", "a b/build", "x; rm -rf ~"]);
/// ```
pub fn fill_template(
    template: &CommandTemplate,
    evaluator: &dyn HoleEvaluator,
    late: Option<&SiteValues>,
) -> Result<FilledChain, HoleError> {
    let mut index = 0;
    let mut resolved: Vec<(&str, String)> = Vec::new();
    let mut segments = Vec::with_capacity(template.segments.len());
    for segment in &template.segments {
        let mut args = Vec::with_capacity(segment.args.len());
        for arg in &segment.args {
            let mut text = String::new();
            for part in &arg.parts {
                match part {
                    ArgPart::Literal(literal) => text.push_str(literal),
                    ArgPart::Hole(placeholder) => {
                        match late.and_then(|late| late.holes.get(&index)) {
                            Some(hole) if !hole.refinement_holds => {
                                return Err(refinement_rejected(placeholder));
                            }
                            Some(hole) => text.push_str(&hole.text),
                            None => match resolved.iter().find(|(expression, _)| *expression == placeholder.expression) {
                                Some((_, value)) => text.push_str(value),
                                None => {
                                    let value = resolve_hole(placeholder, evaluator)?;
                                    text.push_str(&value);
                                    resolved.push((&placeholder.expression, value));
                                }
                            },
                        }
                        index += 1;
                    }
                }
            }
            args.push(text);
        }
        segments.push(FilledSegment {
            operator: segment.operator,
            program: segment.program.clone(),
            args,
            redirection: super::redirection_config(&segment.redirects),
        });
    }
    Ok(FilledChain { segments })
}

/// Evaluates one placeholder, checks it against its declaration and guard
/// refinement, and turns its value into argument text.
fn resolve_hole(placeholder: &Placeholder, evaluator: &dyn HoleEvaluator) -> Result<String, HoleError> {
    let checked = check_hole(placeholder, evaluator)?;
    if !checked.refinement_holds {
        return Err(refinement_rejected(placeholder));
    }
    Ok(scalar_string(&checked.value))
}

/// A value that passed its declaration, and whether it passed the hole's
/// guard refinement too.
struct CheckedValue {
    value: Value,
    refinement_holds: bool,
}

/// Evaluates one placeholder and checks the original value against its
/// declaration. The guard refinement's verdict is returned, not enforced.
fn check_hole(placeholder: &Placeholder, evaluator: &dyn HoleEvaluator) -> Result<CheckedValue, HoleError> {
    let expression = placeholder.expression.clone();
    let value = evaluator.evaluate(&placeholder.expression).map_err(|source| HoleError::Evaluation {
        expression: expression.clone(),
        source: Box::new(source),
    })?;
    let rejected = |reason| HoleError::Rejected {
        expression: expression.clone(),
        declared_type: placeholder.typing.declared_text(),
        reason,
    };
    match &value {
        Value::Null => return Err(HoleError::Missing { expression }),
        Value::Array(_) | Value::Object(_) => return Err(rejected(RejectReason::NotScalar)),
        Value::String(text) if text.contains('\0') => return Err(rejected(RejectReason::EmbeddedNul)),
        _ => {}
    }
    if !placeholder.typing.validator.accepts(&value) {
        return Err(rejected(RejectReason::DeclaredType));
    }
    let refinement_holds = placeholder
        .typing
        .refinement
        .as_ref()
        .is_none_or(|refinement| refinement.accepts(&value));
    Ok(CheckedValue { value, refinement_holds })
}

/// The refusal of a value its guard refinement rejects.
fn refinement_rejected(placeholder: &Placeholder) -> HoleError {
    let narrowed = placeholder
        .typing
        .refinement
        .as_ref()
        .map_or_else(String::new, |refinement| refinement.text());
    HoleError::Rejected {
        expression: placeholder.expression.clone(),
        declared_type: format!("{} narrowed to {narrowed}", placeholder.typing.declared_text()),
        reason: RejectReason::GuardRefinement,
    }
}

impl HoleType {
    /// The type a value is validated against, as written in a schema: the
    /// declaration with its constraints, or `string` for the scalar fallback.
    pub fn declared_text(&self) -> String {
        match &self.validator {
            HoleValidator::FallbackScalar => "string".to_string(),
            HoleValidator::Declared { definition, .. } => super::typing::declaration_text(definition),
        }
    }
}
