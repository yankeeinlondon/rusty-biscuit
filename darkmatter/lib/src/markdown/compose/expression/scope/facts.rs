//! The guard algebra behind [`TypeScope::assume`]: which facts each guard
//! form gives its true and false sides.
//!
//! Every rule reads the AST only. No guard, predicate, or operand is
//! evaluated, and no function handler runs, so `file_exists(x)` touches no
//! filesystem or network. What a predicate proves is the [`NarrowingFact`]
//! its registration declares, read through the registry the way dispatch
//! resolves the name, so a built-in, a test function, and a host function
//! narrow alike.

use serde_json::Value;

use super::shape::{Family, Shape, exact_family};
use super::{GuardOrigin, Polarity, TypeScope};
use crate::markdown::compose::expression::binding::NULL_ROOT;
use crate::markdown::compose::expression::call_binding::{FactKind, NarrowingFact};
use crate::markdown::compose::expression::functions::{
    ShortCircuit, accepts_call_arity, canonical_function_name, inspect_registration, lazy_function,
};
use crate::markdown::compose::expression::{ComparisonOp, Expr, json_number, number_literal};

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
            let args: Vec<&Expr> = args.iter().collect();
            if let Some((_, side)) = lazy_function(name)
                && canonical_function_name(name).is_some_and(|canonical| accepts_call_arity(canonical, args.len()))
            {
                return match side {
                    ShortCircuit::OnFalsy => conjunction(scope, &args, polarity, origin),
                    ShortCircuit::OnTruthy => disjunction(scope, &args, polarity, origin),
                };
            }
            match inspect_registration(name, |registration| registration.narrowing_fact_for(args.len()).cloned()) {
                Some(Some(fact)) => predicate_fact(scope, &fact, &args, polarity, origin),
                _ => scope.clone(),
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

/// The facts a call to a predicate registering `fact` gives its argument,
/// when that argument is a stable subject. A call whose argument is itself a
/// call refines nothing.
fn predicate_fact<'d>(
    scope: &TypeScope<'d>,
    fact: &NarrowingFact,
    args: &[&Expr],
    polarity: Polarity,
    origin: &GuardOrigin,
) -> TypeScope<'d> {
    let Some(argument) = args.get(fact.index()) else {
        return scope.clone();
    };
    let Some(subject) = scope.declared().subject(argument) else {
        return scope.clone();
    };
    let definitions = scope.declared().definitions();
    let target = fact.target();
    match (fact.kind(), polarity) {
        (FactKind::TrueIff, Polarity::True) => match target {
            None => scope.refine(&subject, origin, Shape::without_null),
            Some(target) => match exact_family(target) {
                Some(family) => scope.refine(&subject, origin, |shape| shape.of_family(family, definitions)),
                None => scope.refine(&subject, origin, Shape::only_null),
            },
        },
        (FactKind::TrueIff, Polarity::False) => match target {
            None => scope.refine(&subject, origin, Shape::only_null),
            Some(target) => match exact_family(target) {
                Some(family) => scope.refine(&subject, origin, |shape| shape.without_family(family, definitions)),
                None => scope.refine(&subject, origin, Shape::without_null),
            },
        },
        (FactKind::TrueImplies, Polarity::True) | (FactKind::FalseImplies, Polarity::False) => match target {
            None => scope.refine(&subject, origin, Shape::without_null),
            Some(target) => scope.refine(&subject, origin, |shape| shape.suitable_for(target, definitions)),
        },
        (FactKind::TrueImplies, Polarity::False) | (FactKind::FalseImplies, Polarity::True) => scope.clone(),
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
        (Some(Operand::Container(family)), true) => {
            let definitions = scope.declared().definitions();
            scope.refine(subject, origin, |shape| shape.of_family(family, definitions))
        }
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
