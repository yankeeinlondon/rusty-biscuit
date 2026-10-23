//! Type predicates and math: pure handlers over bound arguments.

use serde_json::Value;

use super::{FunctionBinding, bound, json_number, numberlike_value};
use crate::markdown::compose::expression::call_binding::BoundCall;

/// The absence predicates: a direct bare-variable argument to one of these
/// states that the author expects the value may be missing, so it is not an
/// unknown-identifier warning (spec Requirement 4). Membership in this group
/// is the marker; walkers ask [`super::is_absence_predicate`] and never spell
/// the names.
///
/// Interim (spec Resolved Decision 9): the expression type-system successor
/// replaces this with "any parameter whose type admits null".
pub(super) const ABSENCE_PREDICATES: &[FunctionBinding] = &[
    FunctionBinding::pure("is_null", &["isnull"], is_null),
    FunctionBinding::pure("is_empty", &["isempty"], is_empty),
];

pub(super) const BINDINGS: &[FunctionBinding] = &[
    FunctionBinding::pure("is_string", &["isstring"], is_string),
    FunctionBinding::pure("is_number", &["isnumber"], is_number),
    FunctionBinding::pure("is_array", &["isarray"], is_array),
    FunctionBinding::pure("is_object", &["isobject"], is_object),
    FunctionBinding::pure("is_positive", &["ispositive"], is_positive),
    FunctionBinding::pure("is_negative", &["isnegative"], is_negative),
    FunctionBinding::pure("is_integer", &["isinteger"], is_integer),
    FunctionBinding::pure("min", &[], min),
    FunctionBinding::pure("max", &[], max),
    FunctionBinding::pure("abs", &[], abs),
    FunctionBinding::pure("round", &[], round).recovering("x", "fallback"),
];

/// The one `unknown | null` argument every type predicate inspects as sent.
fn inspected<'a>(call: &'a BoundCall<'_>, name: &str) -> Result<&'a Value, String> {
    bound(call.args().get(0), name, 0)
}

fn is_null(call: &BoundCall<'_>) -> Result<Value, String> {
    Ok(Value::Bool(inspected(call, "is_null")?.is_null()))
}

fn is_empty(call: &BoundCall<'_>) -> Result<Value, String> {
    Ok(Value::Bool(super::is_empty(inspected(call, "is_empty")?)))
}

fn is_string(call: &BoundCall<'_>) -> Result<Value, String> {
    Ok(Value::Bool(inspected(call, "is_string")?.is_string()))
}

fn is_number(call: &BoundCall<'_>) -> Result<Value, String> {
    Ok(Value::Bool(inspected(call, "is_number")?.is_number()))
}

fn is_array(call: &BoundCall<'_>) -> Result<Value, String> {
    Ok(Value::Bool(inspected(call, "is_array")?.is_array()))
}

fn is_object(call: &BoundCall<'_>) -> Result<Value, String> {
    Ok(Value::Bool(inspected(call, "is_object")?.is_object()))
}

/// `is_positive(val: number | null)`: null is `false` (predicate family).
fn is_positive(call: &BoundCall<'_>) -> Result<Value, String> {
    Ok(Value::Bool(call.args().number(0).is_some_and(|number| number > 0.0)))
}

/// `is_negative(val: number | null)`: null is `false` (predicate family).
fn is_negative(call: &BoundCall<'_>) -> Result<Value, String> {
    Ok(Value::Bool(call.args().number(0).is_some_and(|number| number < 0.0)))
}

/// `is_integer(val: unknown | null)`: `true` only for a number with no
/// fractional part; numeric text is not an integer.
fn is_integer(call: &BoundCall<'_>) -> Result<Value, String> {
    let whole = inspected(call, "is_integer")?
        .as_f64()
        .is_some_and(|number| number.is_finite() && number.fract() == 0.0);
    Ok(Value::Bool(whole))
}

fn two_numbers(call: &BoundCall<'_>, name: &str) -> Result<(f64, f64), String> {
    let args = call.args();
    Ok((bound(args.number(0), name, 0)?, bound(args.number(1), name, 1)?))
}

fn min(call: &BoundCall<'_>) -> Result<Value, String> {
    let (a, b) = two_numbers(call, "min")?;
    json_number(a.min(b))
}

fn max(call: &BoundCall<'_>) -> Result<Value, String> {
    let (a, b) = two_numbers(call, "max")?;
    json_number(a.max(b))
}

fn abs(call: &BoundCall<'_>) -> Result<Value, String> {
    json_number(bound(call.args().number(0), "abs", 0)?.abs())
}

/// `round(x: numberlike, fallback?: number)`: the selected value (`x`, or
/// the fallback when `x` did not bind) rounded half away from zero.
fn round(call: &BoundCall<'_>) -> Result<Value, String> {
    let selected = bound(call.args().selected(0), "round", 0)?;
    json_number(numberlike_value(selected, "round")?.round())
}
