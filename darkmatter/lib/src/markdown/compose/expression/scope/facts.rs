//! The guard algebra behind [`TypeScope::assume`]: which facts each guard
//! form gives its true and false sides.
//!
//! Every rule reads the AST only. No guard, predicate, or operand is
//! evaluated, and no function handler runs, so `file_exists(x)` touches no
//! filesystem or network.

use serde_json::Value;

use super::shape::{Family, Shape};
use super::{GuardOrigin, Polarity, TypeScope};
use crate::markdown::compose::expression::binding::NULL_ROOT;
use crate::markdown::compose::expression::functions::{accepts_call_arity, canonical_function_name};
use crate::markdown::compose::expression::{ComparisonOp, Expr, json_number, number_literal};

/// What a registered one-argument predicate proves about its original
/// argument. Keyed by canonical registry name; aliases and case resolve
/// through the registry, never here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum PredicateFact {
    /// True-iff a runtime family: `is_string`, `is_number`, ….
    Family(Family),
    /// True-iff `null`.
    Null,
    /// True for `null` and empty values, so only the false side learns
    /// presence.
    Empty,
    /// True implies a present string: today's handler rejects every other
    /// original type, and returns false for `null`.
    FileExists,
}

/// The predicates whose handlers justify a fact.
pub(super) const PREDICATE_FACTS: &[(&str, PredicateFact)] = &[
    ("is_string", PredicateFact::Family(Family::String)),
    ("is_number", PredicateFact::Family(Family::Number)),
    ("is_array", PredicateFact::Family(Family::Array)),
    ("is_object", PredicateFact::Family(Family::Object)),
    ("is_null", PredicateFact::Null),
    ("is_empty", PredicateFact::Empty),
    ("file_exists", PredicateFact::FileExists),
];

pub(super) fn assume<'d>(scope: &TypeScope<'d>, guard: &Expr, polarity: Polarity, origin: &GuardOrigin) -> TypeScope<'d> {
    match guard {
        Expr::Paren(inner) => assume(scope, inner, polarity, origin),
        Expr::UnaryNot(inner) => assume(scope, inner, polarity.negate(), origin),
        Expr::Fallback { primary, fallback } => {
            // As a condition, `a || b` is truthy exactly when `or(a, b)` is.
            disjunction(scope, &[primary.as_ref(), fallback.as_ref()], polarity, origin)
        }
        Expr::Ternary {
            condition,
            then_branch,
            else_branch,
        } => {
            let chosen = assume(&assume(scope, condition, Polarity::True, origin), then_branch, polarity, origin);
            let other = assume(&assume(scope, condition, Polarity::False, origin), else_branch, polarity, origin);
            TypeScope::join(&chosen, &other)
        }
        Expr::Comparison { left, op, right } => comparison(scope, left, *op, right, polarity, origin),
        Expr::FunctionCall { name, args } => {
            let Some(canonical) = canonical_function_name(name).filter(|canonical| accepts_call_arity(canonical, args.len()))
            else {
                return scope.clone();
            };
            let args: Vec<&Expr> = args.iter().collect();
            match canonical {
                "and" => conjunction(scope, &args, polarity, origin),
                "or" => disjunction(scope, &args, polarity, origin),
                predicate => predicate_fact(scope, predicate, &args, polarity, origin),
            }
        }
        subject => match (scope.declared().subject(subject), polarity) {
            (Some(subject), Polarity::True) => scope.refine(&subject, origin, Shape::truthy),
            _ => scope.clone(),
        },
    }
}

/// `and(a, b, …)`: each operand is checked under the earlier operands' true
/// facts. True means every operand held; false joins the route where each
/// operand first failed.
fn conjunction<'d>(scope: &TypeScope<'d>, operands: &[&Expr], polarity: Polarity, origin: &GuardOrigin) -> TypeScope<'d> {
    short_circuit(scope, operands, polarity, Polarity::True, origin)
}

/// `or(a, b, …)` and `a || b`: each operand is checked under the earlier
/// operands' false facts. False means every operand failed; true joins the
/// route where each operand first held.
fn disjunction<'d>(scope: &TypeScope<'d>, operands: &[&Expr], polarity: Polarity, origin: &GuardOrigin) -> TypeScope<'d> {
    short_circuit(scope, operands, polarity, Polarity::False, origin)
}

/// The shared short-circuit rule: `continues` is the operand outcome that
/// moves evaluation to the next operand.
fn short_circuit<'d>(
    scope: &TypeScope<'d>,
    operands: &[&Expr],
    polarity: Polarity,
    continues: Polarity,
    origin: &GuardOrigin,
) -> TypeScope<'d> {
    if polarity == continues {
        return operands
            .iter()
            .fold(scope.clone(), |scope, operand| assume(&scope, operand, continues, origin));
    }
    let mut prefix = scope.clone();
    let mut routes: Option<TypeScope<'d>> = None;
    for operand in operands {
        let route = assume(&prefix, operand, polarity, origin);
        routes = Some(match routes {
            Some(joined) => TypeScope::join(&joined, &route),
            None => route,
        });
        prefix = assume(&prefix, operand, continues, origin);
    }
    routes.unwrap_or_else(|| scope.clone())
}

fn predicate_fact<'d>(
    scope: &TypeScope<'d>,
    canonical: &str,
    args: &[&Expr],
    polarity: Polarity,
    origin: &GuardOrigin,
) -> TypeScope<'d> {
    let Some((_, fact)) = PREDICATE_FACTS.iter().find(|(name, _)| *name == canonical) else {
        return scope.clone();
    };
    let [argument] = args else {
        return scope.clone();
    };
    let Some(subject) = scope.declared().subject(argument) else {
        return scope.clone();
    };
    match (fact, polarity) {
        (PredicateFact::Family(family), Polarity::True) => scope.refine(&subject, origin, |shape| shape.of_family(*family)),
        (PredicateFact::Family(family), Polarity::False) => {
            scope.refine(&subject, origin, |shape| shape.without_family(*family))
        }
        (PredicateFact::Null, Polarity::True) => scope.refine(&subject, origin, Shape::only_null),
        (PredicateFact::Null | PredicateFact::Empty, Polarity::False) => {
            scope.refine(&subject, origin, Shape::without_null)
        }
        (PredicateFact::FileExists, Polarity::True) => {
            scope.refine(&subject, origin, |shape| shape.of_family(Family::String))
        }
        (PredicateFact::Empty, Polarity::True) | (PredicateFact::FileExists, Polarity::False) => scope.clone(),
    }
}

/// A comparison operand the analysis can read without evaluating anything.
enum Operand {
    Literal(Value),
    /// An array or object literal, whose elements may be computed.
    Container(Family),
}

/// The value the evaluator would produce for a literal operand.
fn literal_operand(expr: &Expr) -> Option<Operand> {
    match expr {
        Expr::Paren(inner) => literal_operand(inner),
        Expr::StringLiteral(text) => Some(Operand::Literal(Value::String(text.clone()))),
        Expr::BoolLiteral(value) => Some(Operand::Literal(Value::Bool(*value))),
        Expr::NumberLiteral(number) => number_literal(*number).map(Operand::Literal),
        // The evaluator negates the literal's value, not its spelling.
        Expr::UnaryMinus(inner) => match inner.as_ref() {
            Expr::NumberLiteral(number) => {
                let negated = -number_literal(*number)?.as_f64()?;
                json_number(negated).ok().map(Operand::Literal)
            }
            _ => None,
        },
        Expr::Variable(path) if path == NULL_ROOT => Some(Operand::Literal(Value::Null)),
        Expr::ArrayLiteral(_) => Some(Operand::Container(Family::Array)),
        Expr::ObjectLiteral(_) => Some(Operand::Container(Family::Object)),
        _ => None,
    }
}

/// `==` and `!=`, as the evaluator decides them: strict typed equality
/// (`values_equal`), under which `null` equals only `null` and the operators
/// are exact complements.
fn comparison<'d>(
    scope: &TypeScope<'d>,
    left: &Expr,
    op: ComparisonOp,
    right: &Expr,
    polarity: Polarity,
    origin: &GuardOrigin,
) -> TypeScope<'d> {
    let equal = match op {
        ComparisonOp::Equal => true,
        ComparisonOp::NotEqual => false,
        _ => return scope.clone(),
    };
    // Whether the guard, as it holds on this side, says the operands are equal.
    let equal = equal == (polarity == Polarity::True);
    let declared = scope.declared();
    match (declared.subject(left), declared.subject(right)) {
        (Some(subject), None) => against_operand(scope, &subject, literal_operand(right), equal, origin),
        (None, Some(subject)) => against_operand(scope, &subject, literal_operand(left), equal, origin),
        (Some(left), Some(right)) => {
            let left_values = scope.type_of(&left).shape().finite_values();
            let right_values = scope.type_of(&right).shape().finite_values();
            let scope = against_values(scope, &left, right_values, equal, origin);
            against_values(&scope, &right, left_values, equal, origin)
        }
        (None, None) => scope.clone(),
    }
}

fn against_operand<'d>(
    scope: &TypeScope<'d>,
    subject: &super::Subject,
    operand: Option<Operand>,
    equal: bool,
    origin: &GuardOrigin,
) -> TypeScope<'d> {
    match (operand, equal) {
        (Some(Operand::Literal(value)), true) => scope.refine(subject, origin, |shape| shape.equal_to(&value)),
        (Some(Operand::Literal(value)), false) => scope.refine(subject, origin, |shape| shape.differs_from(&value)),
        // Equal to a container is present and of its family; its elements
        // may be computed, so the value itself is not pinned.
        (Some(Operand::Container(family)), true) => scope.refine(subject, origin, |shape| shape.of_family(family)),
        (Some(Operand::Container(_)), false) | (None, _) => scope.clone(),
    }
}

/// Narrows `subject` by comparison with another subject whose possible
/// values are `values`, when that set is finite (`null` included). The equal
/// side keeps every alternative that may strictly equal one of them; the
/// unequal side narrows only against a single value.
fn against_values<'d>(
    scope: &TypeScope<'d>,
    subject: &super::Subject,
    values: Option<Vec<Value>>,
    equal: bool,
    origin: &GuardOrigin,
) -> TypeScope<'d> {
    let Some(values) = values else {
        return scope.clone();
    };
    if equal {
        return scope.refine(subject, origin, |shape| {
            values
                .iter()
                .map(|value| shape.equal_to(value))
                .reduce(|joined, route| joined.join(&route))
                .unwrap_or_else(|| shape.clone())
        });
    }
    match values.as_slice() {
        [value] => scope.refine(subject, origin, |shape| shape.differs_from(value)),
        _ => scope.clone(),
    }
}
