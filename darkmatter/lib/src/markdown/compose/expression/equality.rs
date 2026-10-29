//! Strict typed equality for the `==` and `!=` operators.
//!
//! Two values are equal only when they have the same runtime type and the
//! same value, recursively through arrays and objects. Integer and float
//! storage are one runtime number type and compare by exact represented
//! value. This is the single comparison the evaluator and the static guard
//! analysis share; see `darkmatter/docs/topics/darkmatter-expressions.md`.

use serde_json::{Number, Value};

/// Whether `left` and `right` are strictly equal.
///
/// Stops at the first mismatch and allocates nothing.
pub(crate) fn values_equal(left: &Value, right: &Value) -> bool {
    match (left, right) {
        (Value::Null, Value::Null) => true,
        (Value::Bool(left), Value::Bool(right)) => left == right,
        (Value::Number(left), Value::Number(right)) => numbers_equal(left, right),
        (Value::String(left), Value::String(right)) => left == right,
        (Value::Array(left), Value::Array(right)) => {
            left.len() == right.len()
                && left.iter().zip(right).all(|(left, right)| values_equal(left, right))
        }
        (Value::Object(left), Value::Object(right)) => {
            // Equal sizes plus every left key present on the right means the
            // key sets match, so a missing key never equals a `null` entry.
            left.len() == right.len()
                && left.iter().all(|(key, value)| {
                    right.get(key).is_some_and(|other| values_equal(value, other))
                })
        }
        _ => false,
    }
}

/// Exact numeric value equality across `i64`, `u64`, and `f64` storage.
///
/// Never routes both sides through `f64`: `9007199254740992` and
/// `9007199254740993` are distinct integers that share one `f64`.
fn numbers_equal(left: &Number, right: &Number) -> bool {
    match (integer_of(left), integer_of(right)) {
        (Some(left), Some(right)) => left == right,
        (Some(integer), None) => float_is_integer(right.as_f64(), integer),
        (None, Some(integer)) => float_is_integer(left.as_f64(), integer),
        // `0.0 == -0.0` holds under IEEE comparison, as required.
        (None, None) => left.as_f64() == right.as_f64(),
    }
}

/// The integer a number is stored as, widened losslessly to `i128`; `None`
/// for float storage.
fn integer_of(number: &Number) -> Option<i128> {
    number
        .as_i64()
        .map(i128::from)
        .or_else(|| number.as_u64().map(i128::from))
}

/// Whether `float` represents exactly `integer`.
fn float_is_integer(float: Option<f64>, integer: i128) -> bool {
    // Every float in (-2^127, 2^127) with no fraction converts to `i128`
    // exactly; outside that range no `i64`/`u64` can match anyway.
    const LIMIT: f64 = 170_141_183_460_469_231_731_687_303_715_884_105_728.0; // 2^127
    float.is_some_and(|float| {
        float.fract() == 0.0 && float > -LIMIT && float < LIMIT && float as i128 == integer
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn integer_and_float_storage_share_one_number_type() {
        assert!(values_equal(&json!(5), &json!(5.0)));
        assert!(values_equal(&json!(0), &json!(-0.0)));
        assert!(!values_equal(&json!(5), &json!(5.5)));
    }

    #[test]
    fn signed_and_unsigned_storage_compare_without_loss() {
        let above_i64 = Value::Number(Number::from(u64::MAX));
        assert!(values_equal(&above_i64, &Value::Number(Number::from(u64::MAX))));
        assert!(!values_equal(&above_i64, &json!(-1)));
        assert!(values_equal(&json!(i64::MAX), &Value::Number(Number::from(i64::MAX as u64))));
    }

    #[test]
    fn integers_beyond_two_to_the_fifty_three_stay_distinct() {
        let low = json!(9_007_199_254_740_992_i64);
        let high = json!(9_007_199_254_740_993_i64);
        assert!(!values_equal(&low, &high));
        // The float 2^53 is exactly the lower integer, never the upper one.
        assert!(values_equal(&low, &json!(9_007_199_254_740_992.0)));
        assert!(!values_equal(&high, &json!(9_007_199_254_740_992.0)));
    }
}
