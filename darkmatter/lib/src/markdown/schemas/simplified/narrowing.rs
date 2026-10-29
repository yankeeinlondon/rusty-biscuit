//! Declaration validity of `narrows(...)` clauses.
//!
//! The one set of rules every consumer applies: the grammar (and so the
//! public type parser, standalone schema documents, the `type-definition`
//! and `schema` meta-types, and DMLS authoring), host catalog loading, and
//! runtime registration, which re-checks a programmatically built
//! [`FunctionType`] that never passed through the grammar. This module has no
//! dependency on the runtime registry; lowering a valid declaration to a
//! runtime fact stays with registration.

use serde_json::Value;

use super::serialize::serialize_property_def;
use super::types::{
    Constraint, FunctionType, NarrowsClause, NarrowsKind, PropertyAtom, PropertyDef, SimplifiedType, TypeExpr,
    narrows_target_problem, returns_exact_boolean,
};
use crate::markdown::span::SourceSpan;

/// One runtime value family, as the type predicates test it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Family {
    String,
    Number,
    Boolean,
    Array,
    Object,
}

impl Family {
    /// The bare type of the family: `unknown[]` for arrays.
    pub(crate) fn bare_atom(self) -> PropertyAtom {
        match self {
            Self::String => PropertyAtom::bare(SimplifiedType::String),
            Self::Number => PropertyAtom::bare(SimplifiedType::Number),
            Self::Boolean => PropertyAtom::bare(SimplifiedType::Boolean),
            Self::Object => PropertyAtom::bare(SimplifiedType::Object),
            Self::Array => {
                let mut atom = PropertyAtom::bare(SimplifiedType::Unknown);
                atom.is_array = true;
                atom
            }
        }
    }
}

pub(crate) const ALL_FAMILIES: &[Family] =
    &[Family::String, Family::Number, Family::Boolean, Family::Array, Family::Object];

/// The runtime families an alternative's values may belong to. A type whose
/// accepted values are not pinned to one family here keeps every family.
pub(crate) fn families(atom: &PropertyAtom) -> &'static [Family] {
    if atom.is_array {
        return &[Family::Array];
    }
    match &atom.ty {
        TypeExpr::InlineObject(_) => &[Family::Object],
        TypeExpr::Primitive(ty) => match ty {
            SimplifiedType::Literal => match atom.literal_value() {
                Some(Value::String(_)) => &[Family::String],
                Some(Value::Number(_)) => &[Family::Number],
                Some(Value::Bool(_)) => &[Family::Boolean],
                _ => ALL_FAMILIES,
            },
            SimplifiedType::Number => &[Family::Number],
            SimplifiedType::Boolean => &[Family::Boolean],
            SimplifiedType::Object => &[Family::Object],
            _ if crate::markdown::schemas::is_string_like(atom) => &[Family::String],
            _ => ALL_FAMILIES,
        },
        TypeExpr::Tuple(_) => &[Family::Array],
        // A recursive reference stays a named arm, so narrowing never expands
        // it; without its definition table a guard keeps it on both sides.
        TypeExpr::Imported { .. } | TypeExpr::Union(_) | TypeExpr::Function(_) | TypeExpr::Ref(_) => {
            ALL_FAMILIES
        }
    }
}

pub(crate) fn is_null_type(atom: &PropertyAtom) -> bool {
    !atom.is_array && matches!(atom.ty, TypeExpr::Primitive(SimplifiedType::Null))
}

/// The family whose bare type `atom` is (`string`, `number`, `boolean`,
/// `object`, `unknown[]`), with no constraints beyond markers.
pub(crate) fn exact_family(atom: &PropertyAtom) -> Option<Family> {
    let [family] = families(atom) else {
        return None;
    };
    (type_identity(atom.clone()) == type_identity(family.bare_atom()) && !has_ordinary_constraints(atom))
        .then_some(*family)
}

/// Whether `atom` can be a true-iff fact's type: `null` or the bare type of
/// one family ([`exact_family`]). Only those have a complement narrowing
/// can represent.
pub(crate) fn is_exact_fact_type(atom: &PropertyAtom) -> bool {
    is_null_type(atom) || exact_family(atom).is_some()
}

/// Constraints that restrict values beyond the type identity. Markers and
/// annotations (`required`, defaults, suggestions, examples) do not.
pub(crate) fn has_ordinary_constraints(atom: &PropertyAtom) -> bool {
    atom.constraints.iter().chain(&atom.array_constraints).any(|constraint| {
        !matches!(
            constraint,
            Constraint::Required
                | Constraint::Default(_)
                | Constraint::Generated
                | Constraint::Eager
                | Constraint::Suggest(_)
                | Constraint::Example(_)
                | Constraint::Members(_)
                | Constraint::LiteralValue(_)
        )
    })
}

/// `atom` as a member of a value type: its presence markers, timing, and
/// annotations removed, since a shape carries presence in `nullable` and two
/// arms that differ only there are one type. Nested object members keep
/// theirs.
pub(crate) fn type_identity(mut atom: PropertyAtom) -> PropertyAtom {
    let marker = |constraint: &Constraint| {
        matches!(
            constraint,
            Constraint::Required
                | Constraint::Default(_)
                | Constraint::Generated
                | Constraint::Eager
                | Constraint::Suggest(_)
                | Constraint::Example(_)
        )
    };
    atom.constraints.retain(|constraint| !marker(constraint));
    atom.array_constraints.retain(|constraint| !marker(constraint));
    atom.description = None;
    atom
}

/// A target's spelling independent of its description, constraint order, and
/// source location: what two overloads' facts are compared by.
pub(crate) fn canonical_target(atom: &PropertyAtom) -> String {
    let mut atom = atom.clone();
    atom.description = None;
    atom.constraints.sort_by_key(|constraint| format!("{constraint:?}"));
    atom.array_constraints.sort_by_key(|constraint| format!("{constraint:?}"));
    serialize_property_def(&PropertyDef::Single(atom))
}

/// Which declaration a [`NarrowsProblem`] belongs to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ProblemScope {
    /// One overload's clause is invalid on its own; the 0-based overload.
    Overload(usize),
    /// Two overloads disagree on the fact; 0-based, `first` before `later`.
    Overloads { first: usize, later: usize },
}

/// Why a `narrows(...)` declaration is invalid.
///
/// `span` is a byte range in the type-expression text overload `located`
/// was parsed from: for [`ProblemScope::Overloads`], the clause of the
/// overload that disagrees with the first declaring one, or the declaring
/// clause when the other declares none. `related` is then the other
/// overload. A programmatically built declaration carries whatever spans it
/// was given, which no consumer maps to a file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NarrowsProblem {
    pub scope: ProblemScope,
    pub message: String,
    /// The overload (index as in [`ProblemScope`]) whose text `span` is in.
    pub located: usize,
    pub span: SourceSpan,
    /// For [`ProblemScope::Overloads`], the overload `located` conflicts
    /// with.
    pub related: Option<RelatedOverload>,
}

/// The second declaration of a [`ProblemScope::Overloads`] problem.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RelatedOverload {
    pub overload: usize,
    /// The overload's `narrows` clause; `None` when it declares none, so
    /// only the consumer that knows where each overload was written can
    /// locate it.
    pub span: Option<SourceSpan>,
    pub message: String,
}

impl NarrowsProblem {
    /// The message with its overload numbering, for a consumer reporting
    /// the whole overload set at once (registration, catalog loading).
    pub(crate) fn overload_message(&self) -> String {
        match self.scope {
            ProblemScope::Overload(index) => format!("overload {}: {}", index + 1, self.message),
            ProblemScope::Overloads { .. } => self.message.clone(),
        }
    }
}

/// What is wrong with `function`'s own `narrows` clause, if anything: the
/// return type, the named parameter, and the target. `None` without a
/// clause.
pub(crate) fn clause_problem(function: &FunctionType) -> Option<(String, SourceSpan)> {
    let clause = function.narrows.as_ref()?;
    if !returns_exact_boolean(&function.returns) {
        return Some((
            "`narrows` requires `returns(boolean)`; `boolean | null`, `unknown`, `boolish`, \
             unions, and arrays do not qualify"
                .into(),
            clause.clause_span.clone(),
        ));
    }
    match function.parameters.as_slice().iter().find(|parameter| parameter.name == clause.parameter) {
        None => {
            return Some((
                format!("`narrows` names parameter `{}`, which this function does not declare", clause.parameter),
                clause.parameter_span.clone(),
            ));
        }
        Some(parameter) if parameter.rest => {
            return Some((
                format!("`narrows` names parameter `{}`, which is a rest parameter", clause.parameter),
                clause.parameter_span.clone(),
            ));
        }
        Some(_) => {}
    }
    let target = clause.target.as_ref()?;
    if let Some(problem) = narrows_target_problem(target) {
        return Some((problem, clause.target_span.clone()));
    }
    // A grouped array such as `(string | number)[]` is one array type, not a
    // union: only a top-level union is rejected.
    if matches!(target.ty, TypeExpr::Union(_)) && !target.is_array {
        return Some(("a narrowing target must be a single type, not a union".into(), clause.target_span.clone()));
    }
    if clause.kind == NarrowsKind::TrueIff && !is_exact_fact_type(target) {
        return Some((
            format!(
                "a `true-iff` fact needs `null` or the bare type of one value family (`string`, `number`, \
                 `boolean`, `object`, `unknown[]`), not `{}`; use `true-implies` for a refinement",
                canonical_target(target)
            ),
            clause.target_span.clone(),
        ));
    }
    None
}

/// The first problem with the `narrows` declarations of an overload set,
/// checking each overload's clause and then their agreement: when any
/// overload declares a fact, every overload declares the same parameter at
/// the same position, the same implication, and the same canonical target
/// (or none).
pub(crate) fn declaration_problem(overloads: &[FunctionType]) -> Option<NarrowsProblem> {
    let indexed: Vec<(usize, &FunctionType)> = overloads.iter().enumerate().collect();
    declaration_problem_indexed(&indexed)
}

/// [`declaration_problem`] over the overloads among a union's options: its
/// non-array `function` arms. Indexes in the problem are into `arms`.
pub(crate) fn overload_arms_problem(arms: &[PropertyAtom]) -> Option<NarrowsProblem> {
    let indexed: Vec<(usize, &FunctionType)> = arms
        .iter()
        .enumerate()
        .filter_map(|(index, arm)| match &arm.ty {
            TypeExpr::Function(function) if !arm.is_array => Some((index, &**function)),
            _ => None,
        })
        .collect();
    declaration_problem_indexed(&indexed)
}

fn declaration_problem_indexed(overloads: &[(usize, &FunctionType)]) -> Option<NarrowsProblem> {
    for &(index, function) in overloads {
        if let Some((message, span)) = clause_problem(function) {
            return Some(NarrowsProblem {
                scope: ProblemScope::Overload(index),
                message,
                located: index,
                span,
                related: None,
            });
        }
    }
    let (first, declaring, declared) = overloads
        .iter()
        .find_map(|&(index, function)| function.narrows.as_ref().map(|clause| (index, function, clause)))?;
    let expected = effective_fact(declaring);
    for &(other, function) in overloads {
        if other == first {
            continue;
        }
        // A missing fact is located at the declaring clause, the only clause
        // there is; disagreeing facts at the later overload's clause.
        let (message, located, span, related) = match (&function.narrows, effective_fact(function)) {
            (None, _) => (
                format!(
                    "overload {} declares `narrows` but overload {} does not; every overload must declare the same fact",
                    first + 1,
                    other + 1
                ),
                first,
                declared.clause_span.clone(),
                RelatedOverload {
                    overload: other,
                    span: None,
                    message: format!("overload {} declares no `narrows` fact", other + 1),
                },
            ),
            (Some(clause), fact) if fact != expected => (
                format!(
                    "overloads {} and {} declare different `narrows` facts; every overload must declare the same one",
                    first + 1,
                    other + 1
                ),
                other,
                clause.clause_span.clone(),
                RelatedOverload {
                    overload: first,
                    span: Some(declared.clause_span.clone()),
                    message: format!("overload {} declares its `narrows` fact here", first + 1),
                },
            ),
            _ => continue,
        };
        let scope = ProblemScope::Overloads { first: first.min(other), later: first.max(other) };
        return Some(NarrowsProblem { scope, message, located, span, related: Some(related) });
    }
    None
}

/// What an overload's clause means, for agreement: parameter name and
/// position, implication, and canonical target (`None` for presence).
fn effective_fact(function: &FunctionType) -> Option<(&str, Option<usize>, NarrowsKind, Option<String>)> {
    let NarrowsClause { parameter, target, kind, .. } = function.narrows.as_ref()?;
    let position = function.parameters.as_slice().iter().position(|declared| &declared.name == parameter);
    Some((parameter.as_str(), position, *kind, target.as_ref().map(canonical_target)))
}
