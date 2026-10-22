//! Structured linting for `suggest(...)` metadata.
//!
//! The public entry point [`lint_suggestions`] checks every
//! `suggest(...)` candidate against its target schema and returns one
//! [`SuggestionLintProblem`] per invalid candidate. Invalid candidates are
//! **lint problems, not errors** — schema resolution, validator construction,
//! frontmatter validation, and composition all continue uninterrupted.
//!
//! ## Side-effect freedom
//!
//! Linting performs no filesystem discovery, no composition directive
//! evaluation, no shell execution, and no network access. It validates each
//! candidate against an in-memory target schema fragment built from the
//! SimplifiedSchema AST.
//!
//! ## Span units
//!
//! Every problem's [`SuggestionLintProblem::span`] is a byte range into the
//! original authoring source (frontmatter YAML or standalone schema YAML),
//! including argument quotes. These spans are projected through YAML quoting
//! and escaping before lint results are returned.

use jsonschema::error::ValidationErrorKind;
use serde_json::Value;

use crate::markdown::{schemas::SchemaError, span::SourceSpan};

use super::{
    Constraint, PatternKey, PropertyAtom, PropertyDef, SchemaArm, SchemaShape, SimplifiedSchema,
    SimplifiedType, TypeExpr,
    convert::suggestion_target_schema,
    query::{Alternative, flattened_alternatives},
};

/// Stable category describing why a suggestion candidate failed its target.
///
/// DMLS maps each variant to a human-readable diagnostic message. The set is
/// `#[non_exhaustive]` so future categories can be added without a breaking
/// change.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum SuggestionLintReason {
    /// A number candidate does not use the supported simple-decimal syntax.
    InvalidDecimalSyntax,
    /// A simple decimal cannot round-trip through the supported JSON number
    /// model without loss.
    UnsupportedNumberRepresentation,
    /// A numeric minimum or maximum was violated.
    Range,
    /// A numeric candidate is not an integer.
    Integer,
    /// A string length constraint was violated.
    Length,
    /// A string contains no non-whitespace character.
    NotEmpty,
    /// A string pattern constraint was violated.
    Pattern,
    /// The interpreted value has the wrong JSON type for the target.
    Type,
}

/// One invalid candidate found in authored `suggest(...)` metadata.
///
/// Produced by [`lint_suggestions`]. Each problem carries enough structured
/// data for DMLS (or any consumer) to render a precise diagnostic without
/// re-interpreting the candidate or re-validating the schema.
///
/// Spans are byte ranges into the original authoring source (frontmatter YAML
/// or standalone schema YAML), including argument quotes.
#[derive(Debug, Clone, PartialEq)]
pub struct SuggestionLintProblem {
    /// Dotted property path within its root schema arm (e.g. `"settings.mode"`).
    pub property: String,
    /// Property path segments within the root schema arm.
    ///
    /// This is the unambiguous identity for a property; [`Self::property`] is
    /// retained as a human-readable diagnostic label.
    pub property_path: Vec<String>,
    /// Zero-based root-union arm index, or `None` for a single root shape.
    pub root_arm: Option<usize>,
    /// Zero-based property-union atom index, or `None` for a single atom.
    pub property_arm: Option<usize>,
    /// Argument text after SimplifiedSchema quote and escape processing.
    pub decoded: String,
    /// Target-directed JSON value retained in generated metadata.
    pub interpreted: Value,
    /// Stable failure category.
    pub reason: SuggestionLintReason,
    /// Exact authored argument byte span, including argument quotes.
    pub span: SourceSpan,
}

/// Lints every `suggest(...)` candidate in declaration order.
///
/// Walks all properties (including inline-object descendants), pattern keys,
/// property-union atoms, and root-union arms. Each candidate is checked
/// against its target schema — the non-null scalar fragment or array-item
/// fragment, with `x-darkmatter-suggest` excluded — using the existing JSON
/// Schema validator so numeric and string constraints are single-sourced.
/// In a union the target is the property's effective type at the candidate's
/// insertion site: a candidate fails only when no alternative there (scalar
/// alternatives, or element alternatives for an array form) accepts it, since
/// a property's one `suggest(...)` list sits on a single alternative.
/// Candidates on a named-type reference (`Name(suggest(…))`) are skipped
/// until the schema is resolved; lint the resolved schema to judge them
/// against the referenced type.
///
/// ## Errors versus lint
///
/// Invalid candidates are returned as [`SuggestionLintProblem`] data and
/// **never** become a [`SchemaError`]. Errors are reserved for an invalid
/// schema fragment or a validator that cannot be constructed — in those cases
/// the function returns `Err`.
///
/// ## Ordering
///
/// Problems are returned in declaration order: properties first, then pattern
/// keys, then inline-object descendants, then root-union arms.
///
/// ## Side-effect freedom
///
/// No filesystem discovery, shell execution, composition directive evaluation,
/// or network access occurs.
pub fn lint_suggestions(
    schema: &SimplifiedSchema,
) -> Result<Vec<SuggestionLintProblem>, SchemaError> {
    let mut problems = Vec::new();
    match schema {
        SimplifiedSchema::Single(shape) => lint_shape(shape, None, &[], &mut problems)?,
        SimplifiedSchema::Union(arms) => {
            for (root_arm, arm) in arms.iter().enumerate() {
                if let SchemaArm::Inline(shape) = arm {
                    lint_shape(shape, Some(root_arm), &[], &mut problems)?;
                }
            }
        }
    }
    Ok(problems)
}

fn lint_shape(
    shape: &SchemaShape,
    root_arm: Option<usize>,
    parent: &[String],
    problems: &mut Vec<SuggestionLintProblem>,
) -> Result<(), SchemaError> {
    for (name, def) in &shape.properties {
        let mut property_path = parent.to_vec();
        property_path.push(name.clone());
        lint_property(def, root_arm, &property_path, problems)?;
    }
    for pattern in &shape.pattern_keys {
        let name = match &pattern.key {
            PatternKey::CatchAll => "<string>".to_string(),
            PatternKey::Starting(prefix) => format!("<starting::{prefix}>"),
            PatternKey::Ending(suffix) => format!("<ending::{suffix}>"),
            PatternKey::Pattern(pattern) => format!("<pattern::{pattern}>"),
        };
        let mut property_path = parent.to_vec();
        property_path.push(name);
        lint_property(&pattern.def, root_arm, &property_path, problems)?;
    }
    Ok(())
}

fn lint_property(
    def: &PropertyDef,
    root_arm: Option<usize>,
    property_path: &[String],
    problems: &mut Vec<SuggestionLintProblem>,
) -> Result<(), SchemaError> {
    let alternatives = flattened_alternatives(def);
    for alternative in &alternatives {
        match &alternative.atom.ty {
            TypeExpr::InlineObject(shape) => lint_shape(shape, root_arm, property_path, problems)?,
            // A named reference's target is known only after resolution, which a
            // passive lint never performs; linting the resolved schema judges its
            // candidates against the resolved target.
            TypeExpr::Imported { .. } => {}
            // Flattening never yields a union atom; tuple positions and
            // function signatures carry no suggestions to lint.
            TypeExpr::Union(_) | TypeExpr::Tuple(_) | TypeExpr::Function(_) => {}
            TypeExpr::Primitive(_) => {
                lint_candidates(alternative, &alternatives, root_arm, property_path, problems)?;
            }
        }
    }
    Ok(())
}

/// Lints the candidates of one primitive alternative against the property's
/// effective type.
///
/// A candidate is offered wherever the property's value (or, for an array
/// alternative, one of its elements) is completed, so it is valid when **any**
/// alternative at that insertion site accepts it, not only the alternative that
/// stores the `suggest(...)` list. A union keeps one shared list on one arm, so
/// judging that arm alone would reject every candidate meant for its siblings.
/// The reported reason is the storing alternative's own failure.
fn lint_candidates(
    alternative: &Alternative<'_>,
    alternatives: &[Alternative<'_>],
    root_arm: Option<usize>,
    property_path: &[String],
    problems: &mut Vec<SuggestionLintProblem>,
) -> Result<(), SchemaError> {
    let atom = alternative.atom;
    let Some(candidates) = atom.constraints.iter().find_map(|constraint| match constraint {
        Constraint::Suggest(candidates) => Some(candidates),
        _ => None,
    }) else {
        return Ok(());
    };

    let property = property_path.join(".");
    let target = suggestion_target_schema(&property, atom)?;
    let validator = super::super::validate::build_structural_validator(&target)?;
    let is_number = matches!(atom.ty, TypeExpr::Primitive(SimplifiedType::Number));
    let mut siblings: Option<Vec<jsonschema::Validator>> = None;

    for candidate in candidates {
        let reason = if is_number && candidate.canonical_decimal.is_none() {
            Some(SuggestionLintReason::InvalidDecimalSyntax)
        } else if is_number && candidate.interpreted.is_string() {
            Some(SuggestionLintReason::UnsupportedNumberRepresentation)
        } else {
            validator
                .iter_errors(&candidate.interpreted)
                .next()
                .map(|error| classify_validation_error(error.kind(), atom, &candidate.interpreted))
                .filter(|_| {
                    let siblings = siblings.get_or_insert_with(|| {
                        sibling_validators(alternative, alternatives, &property)
                    });
                    !siblings
                        .iter()
                        .any(|sibling| sibling.is_valid(&candidate.interpreted))
                })
        };
        if let Some(reason) = reason {
            problems.push(SuggestionLintProblem {
                property: property.clone(),
                property_path: property_path.to_vec(),
                root_arm,
                property_arm: alternative.arm,
                decoded: candidate.decoded.clone(),
                interpreted: candidate.interpreted.clone(),
                reason,
                span: candidate.span.clone(),
            });
        }
    }
    Ok(())
}

/// Validators for every other alternative at `alternative`'s insertion site
/// (scalar value or array element).
///
/// An unresolved named reference has no known target and is left out, as is a
/// sibling whose fragment cannot be built; neither can vouch for a candidate.
fn sibling_validators(
    alternative: &Alternative<'_>,
    alternatives: &[Alternative<'_>],
    property: &str,
) -> Vec<jsonschema::Validator> {
    alternatives
        .iter()
        .filter(|sibling| {
            !std::ptr::eq(sibling.atom, alternative.atom)
                && sibling.is_array == alternative.is_array
                && !matches!(sibling.atom.ty, TypeExpr::Imported { .. } | TypeExpr::Union(_))
        })
        .filter_map(|sibling| {
            let target = suggestion_target_schema(property, sibling.atom).ok()?;
            super::super::validate::build_structural_validator(&target).ok()
        })
        .collect()
}

fn classify_validation_error(
    kind: &ValidationErrorKind,
    atom: &PropertyAtom,
    value: &Value,
) -> SuggestionLintReason {
    match kind {
        ValidationErrorKind::Minimum { .. }
        | ValidationErrorKind::Maximum { .. }
        | ValidationErrorKind::ExclusiveMinimum { .. }
        | ValidationErrorKind::ExclusiveMaximum { .. } => SuggestionLintReason::Range,
        ValidationErrorKind::MinLength { .. } | ValidationErrorKind::MaxLength { .. } => {
            SuggestionLintReason::Length
        }
        ValidationErrorKind::Pattern { .. }
            if atom
                .constraints
                .iter()
                .any(|constraint| matches!(constraint, Constraint::NotEmpty))
                && value.as_str().is_some_and(|text| text.trim().is_empty()) =>
        {
            SuggestionLintReason::NotEmpty
        }
        ValidationErrorKind::Pattern { .. } => SuggestionLintReason::Pattern,
        ValidationErrorKind::Type { .. }
            if atom
                .constraints
                .iter()
                .any(|constraint| matches!(constraint, Constraint::Integer)) =>
        {
            SuggestionLintReason::Integer
        }
        _ => SuggestionLintReason::Type,
    }
}
