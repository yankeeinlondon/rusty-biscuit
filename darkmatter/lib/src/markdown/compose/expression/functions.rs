//! Built-in helper functions for the Darkmatter expression evaluator.
//!
//! This module groups the type predicates, math helpers, collection helpers,
//! string predicates and mutations, and date validators added in the
//! expression-syntax expansion. Functions here all take fully-evaluated
//! `serde_json::Value` arguments and follow the spec's null-propagation /
//! type-mismatch contract: a `Value::Null` argument propagates through the
//! call as `Value::Null`, while a value of the wrong domain returns an
//! evaluator error.

use chrono::{Datelike, Local, NaiveDate, NaiveDateTime, Utc};
use serde_json::Value;

use super::json_number;

/// Tests whether the value is "empty" per the spec: `null`, empty string,
/// empty array, or empty object. Numbers, booleans, and non-empty containers
/// all return `false`.
pub fn is_empty(value: &Value) -> bool {
    match value {
        Value::Null => true,
        Value::String(s) => s.is_empty(),
        Value::Array(arr) => arr.is_empty(),
        Value::Object(obj) => obj.is_empty(),
        Value::Number(_) | Value::Bool(_) => false,
    }
}

fn require_args(name: &str, args: &[Value], expected: usize) -> Result<(), String> {
    if args.len() != expected {
        Err(format!(
            "{name}() requires {expected} argument{}",
            if expected == 1 { "" } else { "s" }
        ))
    } else {
        Ok(())
    }
}

fn any_null(args: &[Value]) -> bool {
    args.iter().any(Value::is_null)
}

fn require_number(name: &str, value: &Value) -> Result<f64, String> {
    match value {
        Value::Number(n) => n
            .as_f64()
            .ok_or_else(|| format!("{name}() received an unrepresentable number")),
        _ => Err(format!("{name}() requires numeric arguments")),
    }
}

fn require_string<'a>(name: &str, value: &'a Value) -> Result<&'a str, String> {
    match value {
        Value::String(s) => Ok(s.as_str()),
        _ => Err(format!("{name}() requires string arguments")),
    }
}

fn require_array<'a>(name: &str, value: &'a Value) -> Result<&'a Vec<Value>, String> {
    match value {
        Value::Array(items) => Ok(items),
        _ => Err(format!("{name}() requires an array argument")),
    }
}

/// Type predicates from the spec.
pub fn is_string(args: &[Value]) -> Result<Value, String> {
    require_args("is_string", args, 1)?;
    Ok(Value::Bool(matches!(args[0], Value::String(_))))
}

pub fn is_number(args: &[Value]) -> Result<Value, String> {
    require_args("is_number", args, 1)?;
    Ok(Value::Bool(matches!(args[0], Value::Number(_))))
}

pub fn is_array(args: &[Value]) -> Result<Value, String> {
    require_args("is_array", args, 1)?;
    Ok(Value::Bool(matches!(args[0], Value::Array(_))))
}

pub fn is_null(args: &[Value]) -> Result<Value, String> {
    require_args("is_null", args, 1)?;
    Ok(Value::Bool(matches!(args[0], Value::Null)))
}

pub fn is_object(args: &[Value]) -> Result<Value, String> {
    require_args("is_object", args, 1)?;
    Ok(Value::Bool(matches!(args[0], Value::Object(_))))
}

pub fn is_empty_fn(args: &[Value]) -> Result<Value, String> {
    require_args("is_empty", args, 1)?;
    Ok(Value::Bool(is_empty(&args[0])))
}

/// `min(a, b)`.
pub fn min_fn(args: &[Value]) -> Result<Value, String> {
    require_args("min", args, 2)?;
    if any_null(args) {
        return Ok(Value::Null);
    }
    let a = require_number("min", &args[0])?;
    let b = require_number("min", &args[1])?;
    json_number(a.min(b))
}

/// `max(a, b)`.
pub fn max_fn(args: &[Value]) -> Result<Value, String> {
    require_args("max", args, 2)?;
    if any_null(args) {
        return Ok(Value::Null);
    }
    let a = require_number("max", &args[0])?;
    let b = require_number("max", &args[1])?;
    json_number(a.max(b))
}

/// `abs(x)`.
pub fn abs_fn(args: &[Value]) -> Result<Value, String> {
    require_args("abs", args, 1)?;
    if any_null(args) {
        return Ok(Value::Null);
    }
    let x = require_number("abs", &args[0])?;
    json_number(x.abs())
}

/// `first(x)` returns the first element of an array (or null when empty).
pub fn first_fn(args: &[Value]) -> Result<Value, String> {
    require_args("first", args, 1)?;
    if any_null(args) {
        return Ok(Value::Null);
    }
    let items = require_array("first", &args[0])?;
    Ok(items.first().cloned().unwrap_or(Value::Null))
}

/// `last(x)` returns the last element of an array (or null when empty).
pub fn last_fn(args: &[Value]) -> Result<Value, String> {
    require_args("last", args, 1)?;
    if any_null(args) {
        return Ok(Value::Null);
    }
    let items = require_array("last", &args[0])?;
    Ok(items.last().cloned().unwrap_or(Value::Null))
}

/// `starts_with(x, find)` — case-sensitive prefix test.
pub fn starts_with(args: &[Value]) -> Result<Value, String> {
    require_args("starts_with", args, 2)?;
    if any_null(args) {
        return Ok(Value::Null);
    }
    let haystack = require_string("starts_with", &args[0])?;
    let needle = require_string("starts_with", &args[1])?;
    Ok(Value::Bool(haystack.starts_with(needle)))
}

/// `ends_with(x, find)` — case-sensitive suffix test.
pub fn ends_with(args: &[Value]) -> Result<Value, String> {
    require_args("ends_with", args, 2)?;
    if any_null(args) {
        return Ok(Value::Null);
    }
    let haystack = require_string("ends_with", &args[0])?;
    let needle = require_string("ends_with", &args[1])?;
    Ok(Value::Bool(haystack.ends_with(needle)))
}

fn string_mutation<F>(name: &str, args: &[Value], f: F) -> Result<Value, String>
where
    F: Fn(&str) -> String,
{
    require_args(name, args, 1)?;
    if any_null(args) {
        return Ok(Value::Null);
    }
    let s = require_string(name, &args[0])?;
    Ok(Value::String(f(s)))
}

pub fn lower(args: &[Value]) -> Result<Value, String> {
    string_mutation("lower", args, |s| s.to_lowercase())
}

pub fn upper(args: &[Value]) -> Result<Value, String> {
    string_mutation("upper", args, |s| s.to_uppercase())
}

pub fn capitalize(args: &[Value]) -> Result<Value, String> {
    string_mutation("capitalize", args, |s| {
        let mut chars = s.chars();
        match chars.next() {
            Some(first) => first.to_uppercase().chain(chars).collect(),
            None => String::new(),
        }
    })
}

/// Splits a string into "words" suitable for case-conversion. Words are
/// runs of letters or digits, with boundaries at any non-alphanumeric
/// character or transitions like `aB` (camel boundary) and `IDFoo`
/// (acronym boundary). The output is lowercase tokens.
fn split_words(input: &str) -> Vec<String> {
    let mut words: Vec<String> = Vec::new();
    let mut current = String::new();
    let chars: Vec<char> = input.chars().collect();

    let flush = |current: &mut String, words: &mut Vec<String>| {
        if !current.is_empty() {
            words.push(current.to_lowercase());
            current.clear();
        }
    };

    for i in 0..chars.len() {
        let c = chars[i];
        if !c.is_alphanumeric() {
            flush(&mut current, &mut words);
            continue;
        }

        if !current.is_empty() {
            let prev = chars[i - 1];
            // Word boundaries:
            // - lower→Upper transition (e.g., "fooBar" → "foo|Bar")
            // - digit↔letter transition (e.g., "v2foo" → "v|2|foo")
            // - acronym boundary: "FOOBar" → "FOO|Bar"
            let lower_to_upper = prev.is_lowercase() && c.is_uppercase();
            let alpha_digit_flip = prev.is_alphabetic() != c.is_alphabetic();
            let acronym_break = prev.is_uppercase()
                && c.is_uppercase()
                && i + 1 < chars.len()
                && chars[i + 1].is_lowercase();
            if lower_to_upper || alpha_digit_flip || acronym_break {
                flush(&mut current, &mut words);
            }
        }
        current.push(c);
    }
    flush(&mut current, &mut words);
    words
}

pub fn kebab_case(args: &[Value]) -> Result<Value, String> {
    string_mutation("kebab_case", args, |s| split_words(s).join("-"))
}

pub fn snake_case(args: &[Value]) -> Result<Value, String> {
    string_mutation("snake_case", args, |s| split_words(s).join("_"))
}

pub fn camel_case(args: &[Value]) -> Result<Value, String> {
    string_mutation("camel_case", args, |s| {
        let words = split_words(s);
        let mut iter = words.into_iter();
        let mut out = String::new();
        if let Some(first) = iter.next() {
            out.push_str(&first);
        }
        for word in iter {
            let mut chars = word.chars();
            if let Some(c) = chars.next() {
                out.extend(c.to_uppercase());
                out.extend(chars);
            }
        }
        out
    })
}

pub fn pascal_case(args: &[Value]) -> Result<Value, String> {
    string_mutation("pascal_case", args, |s| {
        let mut out = String::new();
        for word in split_words(s) {
            let mut chars = word.chars();
            if let Some(c) = chars.next() {
                out.extend(c.to_uppercase());
                out.extend(chars);
            }
        }
        out
    })
}

pub fn title_case(args: &[Value]) -> Result<Value, String> {
    string_mutation("title_case", args, |s| {
        let mut parts: Vec<String> = Vec::new();
        for word in split_words(s) {
            let mut chars = word.chars();
            if let Some(c) = chars.next() {
                let mut buf: String = c.to_uppercase().collect();
                buf.extend(chars);
                parts.push(buf);
            }
        }
        parts.join(" ")
    })
}

/// Parses a strict ISO date `YYYY-MM-DD`.
pub fn parse_iso_date(s: &str) -> Option<NaiveDate> {
    if s.len() != 10 {
        return None;
    }
    NaiveDate::parse_from_str(s, "%Y-%m-%d").ok()
}

/// Parses a string as either an ISO date or an ISO datetime, returning the
/// date portion. Datetime strings without an offset are interpreted in the
/// requested timezone (`assume_utc == true` → UTC, else Local).
pub fn parse_date_or_datetime(s: &str, assume_utc: bool) -> Option<NaiveDate> {
    if let Some(date) = parse_iso_date(s) {
        return Some(date);
    }
    if let Some(date) = parse_iso_datetime_to_date(s, assume_utc) {
        return Some(date);
    }
    None
}

/// Validates an ISO datetime string.
///
/// Accepts:
/// - `YYYY-MM-DDTHH:MM:SS` (naive)
/// - `YYYY-MM-DDTHH:MM:SS.fff` (naive with fractional seconds)
/// - `YYYY-MM-DDTHH:MM:SSZ` (UTC offset shorthand)
/// - `YYYY-MM-DDTHH:MM:SS±HH:MM` (RFC3339 offset)
pub fn parse_iso_datetime(s: &str) -> bool {
    if chrono::DateTime::parse_from_rfc3339(s).is_ok() {
        return true;
    }
    NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S").is_ok()
        || NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S%.f").is_ok()
}

fn parse_iso_datetime_to_date(s: &str, assume_utc: bool) -> Option<NaiveDate> {
    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(s) {
        return Some(if assume_utc {
            dt.with_timezone(&Utc).date_naive()
        } else {
            dt.with_timezone(&Local).date_naive()
        });
    }
    // Naive datetimes carry no offset; the date portion is the same regardless
    // of whether we treat the wall-clock time as local or UTC.
    NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S")
        .ok()
        .or_else(|| NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S%.f").ok())
        .map(|naive| naive.date())
}

/// `is_date(x)` — strict `YYYY-MM-DD` validator. Strings only.
pub fn is_date(args: &[Value]) -> Result<Value, String> {
    require_args("is_date", args, 1)?;
    let ok = matches!(&args[0], Value::String(s) if parse_iso_date(s).is_some());
    Ok(Value::Bool(ok))
}

/// `is_date_utc(x)` — same as `is_date` (the format itself is timezone-agnostic).
pub fn is_date_utc(args: &[Value]) -> Result<Value, String> {
    require_args("is_date_utc", args, 1)?;
    let ok = matches!(&args[0], Value::String(s) if parse_iso_date(s).is_some());
    Ok(Value::Bool(ok))
}

/// `is_date_time(x)` — strict ISO datetime validator. Strings only.
pub fn is_datetime(args: &[Value]) -> Result<Value, String> {
    require_args("is_date_time", args, 1)?;
    let ok = matches!(&args[0], Value::String(s) if parse_iso_datetime(s));
    Ok(Value::Bool(ok))
}

/// `is_date_time_utc(x)` — same parse contract as `is_date_time`.
pub fn is_datetime_utc(args: &[Value]) -> Result<Value, String> {
    require_args("is_date_time_utc", args, 1)?;
    let ok = matches!(&args[0], Value::String(s) if parse_iso_datetime(s));
    Ok(Value::Bool(ok))
}

/// Pure helper for relative date predicates with an injectable reference
/// date and an explicit timezone hint for naive datetime interpretation.
pub fn is_today_with(value: &Value, today: NaiveDate, assume_utc: bool) -> bool {
    let Value::String(s) = value else {
        return false;
    };
    parse_date_or_datetime(s, assume_utc)
        .map(|d| d == today)
        .unwrap_or(false)
}

pub fn is_yesterday_with(value: &Value, today: NaiveDate, assume_utc: bool) -> bool {
    let Value::String(s) = value else {
        return false;
    };
    let yesterday = today.pred_opt();
    parse_date_or_datetime(s, assume_utc)
        .and_then(|d| yesterday.map(|y| d == y))
        .unwrap_or(false)
}

pub fn is_tomorrow_with(value: &Value, today: NaiveDate, assume_utc: bool) -> bool {
    let Value::String(s) = value else {
        return false;
    };
    let tomorrow = today.succ_opt();
    parse_date_or_datetime(s, assume_utc)
        .and_then(|d| tomorrow.map(|t| d == t))
        .unwrap_or(false)
}

pub fn is_this_month_with(value: &Value, today: NaiveDate, assume_utc: bool) -> bool {
    let Value::String(s) = value else {
        return false;
    };
    parse_date_or_datetime(s, assume_utc)
        .map(|d| d.year() == today.year() && d.month() == today.month())
        .unwrap_or(false)
}

pub fn is_this_year_with(value: &Value, today: NaiveDate, assume_utc: bool) -> bool {
    let Value::String(s) = value else {
        return false;
    };
    parse_date_or_datetime(s, assume_utc)
        .map(|d| d.year() == today.year())
        .unwrap_or(false)
}

fn today_local() -> NaiveDate {
    Local::now().date_naive()
}

fn today_utc() -> NaiveDate {
    Utc::now().date_naive()
}

pub fn is_today(args: &[Value]) -> Result<Value, String> {
    require_args("is_today", args, 1)?;
    Ok(Value::Bool(is_today_with(&args[0], today_local(), false)))
}

pub fn is_today_utc(args: &[Value]) -> Result<Value, String> {
    require_args("is_today_utc", args, 1)?;
    Ok(Value::Bool(is_today_with(&args[0], today_utc(), true)))
}

pub fn is_yesterday(args: &[Value]) -> Result<Value, String> {
    require_args("is_yesterday", args, 1)?;
    Ok(Value::Bool(is_yesterday_with(
        &args[0],
        today_local(),
        false,
    )))
}

pub fn is_yesterday_utc(args: &[Value]) -> Result<Value, String> {
    require_args("is_yesterday_utc", args, 1)?;
    Ok(Value::Bool(is_yesterday_with(&args[0], today_utc(), true)))
}

pub fn is_tomorrow(args: &[Value]) -> Result<Value, String> {
    require_args("is_tomorrow", args, 1)?;
    Ok(Value::Bool(is_tomorrow_with(
        &args[0],
        today_local(),
        false,
    )))
}

pub fn is_tomorrow_utc(args: &[Value]) -> Result<Value, String> {
    require_args("is_tomorrow_utc", args, 1)?;
    Ok(Value::Bool(is_tomorrow_with(&args[0], today_utc(), true)))
}

pub fn is_this_month(args: &[Value]) -> Result<Value, String> {
    require_args("is_this_month", args, 1)?;
    Ok(Value::Bool(is_this_month_with(
        &args[0],
        today_local(),
        false,
    )))
}

pub fn is_this_month_utc(args: &[Value]) -> Result<Value, String> {
    require_args("is_this_month_utc", args, 1)?;
    Ok(Value::Bool(is_this_month_with(&args[0], today_utc(), true)))
}

pub fn is_this_year(args: &[Value]) -> Result<Value, String> {
    require_args("is_this_year", args, 1)?;
    Ok(Value::Bool(is_this_year_with(
        &args[0],
        today_local(),
        false,
    )))
}

pub fn is_this_year_utc(args: &[Value]) -> Result<Value, String> {
    require_args("is_this_year_utc", args, 1)?;
    Ok(Value::Bool(is_this_year_with(&args[0], today_utc(), true)))
}

/// Dispatches an evaluated function call against the new helper library.
///
/// Returns `Some(result)` when the name matches a helper in this module,
/// or `None` to let the outer dispatcher fall through to its own handlers
/// (`Unknown function: ...`).
pub fn dispatch(name: &str, args: &[Value]) -> Option<Result<Value, String>> {
    let f = match name {
        // Type predicates
        "isstring" | "is_string" => is_string as fn(&[Value]) -> Result<Value, String>,
        "isnumber" | "is_number" => is_number,
        "isarray" | "is_array" => is_array,
        "isnull" | "is_null" => is_null,
        "isobject" | "is_object" => is_object,
        "isempty" | "is_empty" => is_empty_fn,
        // Math helpers
        "min" => min_fn,
        "max" => max_fn,
        "abs" => abs_fn,
        // Collection helpers
        "first" => first_fn,
        "last" => last_fn,
        // String predicates
        "startswith" | "starts_with" => starts_with,
        "endswith" | "ends_with" => ends_with,
        // String mutations
        "lower" => lower,
        "upper" => upper,
        "capitalize" => capitalize,
        "kebabcase" | "kebab_case" => kebab_case,
        "snakecase" | "snake_case" => snake_case,
        "camelcase" | "camel_case" => camel_case,
        "pascalcase" | "pascal_case" => pascal_case,
        "titlecase" | "title_case" => title_case,
        // Strict date validators
        "isdate" | "is_date" => is_date,
        "isdateutc" | "is_date_utc" => is_date_utc,
        "isdatetime" | "is_datetime" | "is_date_time" => is_datetime,
        "isdatetimeutc" | "is_datetime_utc" | "is_date_time_utc" => is_datetime_utc,
        // Relative date validators
        "istoday" | "is_today" => is_today,
        "istodayutc" | "is_today_utc" => is_today_utc,
        "isyesterday" | "is_yesterday" => is_yesterday,
        "isyesterdayutc" | "is_yesterday_utc" => is_yesterday_utc,
        "istomorrow" | "is_tomorrow" => is_tomorrow,
        "istomorrowutc" | "is_tomorrow_utc" => is_tomorrow_utc,
        "isthismonth" | "is_this_month" => is_this_month,
        "isthismonthutc" | "is_this_month_utc" => is_this_month_utc,
        "isthisyear" | "is_this_year" => is_this_year,
        "isthisyearutc" | "is_this_year_utc" => is_this_year_utc,
        _ => return None,
    };
    Some(f(args))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn v(value: Value) -> Vec<Value> {
        vec![value]
    }

    fn vv(a: Value, b: Value) -> Vec<Value> {
        vec![a, b]
    }

    mod fn_type_predicates {
        use super::*;

        #[test]
        fn is_string_true_for_strings_only() {
            assert_eq!(is_string(&v(json!("hi"))).unwrap(), json!(true));
            assert_eq!(is_string(&v(json!(""))).unwrap(), json!(true));
            assert_eq!(is_string(&v(json!(0))).unwrap(), json!(false));
            assert_eq!(is_string(&v(json!(null))).unwrap(), json!(false));
            assert_eq!(is_string(&v(json!([]))).unwrap(), json!(false));
            assert_eq!(is_string(&v(json!({}))).unwrap(), json!(false));
            assert_eq!(is_string(&v(json!(true))).unwrap(), json!(false));
        }

        #[test]
        fn is_number_true_for_numbers_only() {
            assert_eq!(is_number(&v(json!(0))).unwrap(), json!(true));
            assert_eq!(is_number(&v(json!(1.5))).unwrap(), json!(true));
            assert_eq!(is_number(&v(json!("1"))).unwrap(), json!(false));
            assert_eq!(is_number(&v(json!(true))).unwrap(), json!(false));
            assert_eq!(is_number(&v(json!(null))).unwrap(), json!(false));
        }

        #[test]
        fn is_array_true_for_arrays_only() {
            assert_eq!(is_array(&v(json!([]))).unwrap(), json!(true));
            assert_eq!(is_array(&v(json!([1, 2]))).unwrap(), json!(true));
            assert_eq!(is_array(&v(json!({}))).unwrap(), json!(false));
            assert_eq!(is_array(&v(json!("a"))).unwrap(), json!(false));
        }

        #[test]
        fn is_null_true_for_null_only() {
            assert_eq!(is_null(&v(json!(null))).unwrap(), json!(true));
            assert_eq!(is_null(&v(json!(""))).unwrap(), json!(false));
            assert_eq!(is_null(&v(json!(0))).unwrap(), json!(false));
            assert_eq!(is_null(&v(json!(false))).unwrap(), json!(false));
        }

        #[test]
        fn is_object_true_for_objects_only() {
            assert_eq!(is_object(&v(json!({}))).unwrap(), json!(true));
            assert_eq!(is_object(&v(json!({"a": 1}))).unwrap(), json!(true));
            assert_eq!(is_object(&v(json!([]))).unwrap(), json!(false));
            assert_eq!(is_object(&v(json!("a"))).unwrap(), json!(false));
        }

        #[test]
        fn is_empty_true_for_null_empty_string_empty_array_empty_object() {
            assert_eq!(is_empty_fn(&v(json!(null))).unwrap(), json!(true));
            assert_eq!(is_empty_fn(&v(json!(""))).unwrap(), json!(true));
            assert_eq!(is_empty_fn(&v(json!([]))).unwrap(), json!(true));
            assert_eq!(is_empty_fn(&v(json!({}))).unwrap(), json!(true));
        }

        #[test]
        fn is_empty_false_for_numbers_booleans_and_non_empty_containers() {
            assert_eq!(is_empty_fn(&v(json!(0))).unwrap(), json!(false));
            assert_eq!(is_empty_fn(&v(json!(0.0))).unwrap(), json!(false));
            assert_eq!(is_empty_fn(&v(json!(false))).unwrap(), json!(false));
            assert_eq!(is_empty_fn(&v(json!(true))).unwrap(), json!(false));
            assert_eq!(is_empty_fn(&v(json!("a"))).unwrap(), json!(false));
            assert_eq!(is_empty_fn(&v(json!([0]))).unwrap(), json!(false));
            assert_eq!(is_empty_fn(&v(json!({"a": 1}))).unwrap(), json!(false));
        }
    }

    mod fn_math {
        use super::*;

        #[test]
        fn min_max_abs_basic() {
            assert_eq!(min_fn(&vv(json!(2), json!(5))).unwrap(), json!(2));
            assert_eq!(max_fn(&vv(json!(2), json!(5))).unwrap(), json!(5));
            assert_eq!(abs_fn(&v(json!(-3))).unwrap(), json!(3));
            assert_eq!(abs_fn(&v(json!(3.5))).unwrap(), json!(3.5));
        }

        #[test]
        fn math_null_propagates() {
            assert_eq!(min_fn(&vv(json!(null), json!(5))).unwrap(), json!(null));
            assert_eq!(max_fn(&vv(json!(2), json!(null))).unwrap(), json!(null));
            assert_eq!(abs_fn(&v(json!(null))).unwrap(), json!(null));
        }

        #[test]
        fn math_type_mismatch_errors() {
            let err = min_fn(&vv(json!("foo"), json!(5))).unwrap_err();
            assert!(err.contains("min"));
            let err = max_fn(&vv(json!([]), json!(5))).unwrap_err();
            assert!(err.contains("max"));
            let err = abs_fn(&v(json!("nope"))).unwrap_err();
            assert!(err.contains("abs"));
        }

        #[test]
        fn math_with_boolean_returns_error() {
            let err = min_fn(&vv(json!(true), json!(5))).unwrap_err();
            assert!(err.contains("min"), "got: {err}");
            let err = max_fn(&vv(json!(false), json!(5))).unwrap_err();
            assert!(err.contains("max"), "got: {err}");
            let err = abs_fn(&v(json!(true))).unwrap_err();
            assert!(err.contains("abs"), "got: {err}");
        }

        #[test]
        fn math_with_null_propagates() {
            assert_eq!(min_fn(&vv(json!(null), json!(5))).unwrap(), json!(null));
            assert_eq!(max_fn(&vv(json!(2), json!(null))).unwrap(), json!(null));
            assert_eq!(abs_fn(&v(json!(null))).unwrap(), json!(null));
        }
    }

    mod fn_collection {
        use super::*;

        #[test]
        fn first_last_basic() {
            assert_eq!(first_fn(&v(json!([1, 2, 3]))).unwrap(), json!(1));
            assert_eq!(last_fn(&v(json!([1, 2, 3]))).unwrap(), json!(3));
        }

        #[test]
        fn first_last_empty_returns_null() {
            assert_eq!(first_fn(&v(json!([]))).unwrap(), json!(null));
            assert_eq!(last_fn(&v(json!([]))).unwrap(), json!(null));
        }

        #[test]
        fn first_last_null_propagates() {
            assert_eq!(first_fn(&v(json!(null))).unwrap(), json!(null));
            assert_eq!(last_fn(&v(json!(null))).unwrap(), json!(null));
        }

        #[test]
        fn first_last_type_mismatch_errors() {
            assert!(first_fn(&v(json!("hi"))).is_err());
            assert!(last_fn(&v(json!({"a": 1}))).is_err());
        }
    }

    mod fn_string_predicates {
        use super::*;

        #[test]
        fn startswith_endswith_basic() {
            assert_eq!(
                starts_with(&vv(json!("foobar"), json!("foo"))).unwrap(),
                json!(true)
            );
            assert_eq!(
                starts_with(&vv(json!("foobar"), json!("bar"))).unwrap(),
                json!(false)
            );
            assert_eq!(
                ends_with(&vv(json!("foobar"), json!("bar"))).unwrap(),
                json!(true)
            );
            assert_eq!(
                ends_with(&vv(json!("foobar"), json!("foo"))).unwrap(),
                json!(false)
            );
        }

        #[test]
        fn startswith_case_sensitive() {
            assert_eq!(
                starts_with(&vv(json!("Foobar"), json!("foo"))).unwrap(),
                json!(false)
            );
        }

        #[test]
        fn startswith_endswith_null_propagates() {
            assert_eq!(
                starts_with(&vv(json!(null), json!("foo"))).unwrap(),
                json!(null)
            );
            assert_eq!(
                ends_with(&vv(json!("hi"), json!(null))).unwrap(),
                json!(null)
            );
        }

        #[test]
        fn startswith_endswith_type_mismatch_errors() {
            assert!(starts_with(&vv(json!(5), json!("foo"))).is_err());
            assert!(ends_with(&vv(json!("hi"), json!(5))).is_err());
        }
    }

    mod fn_string_mutations {
        use super::*;

        #[test]
        fn lower_upper_capitalize() {
            assert_eq!(lower(&v(json!("FoO"))).unwrap(), json!("foo"));
            assert_eq!(upper(&v(json!("FoO"))).unwrap(), json!("FOO"));
            assert_eq!(capitalize(&v(json!("hello"))).unwrap(), json!("Hello"));
            assert_eq!(capitalize(&v(json!(""))).unwrap(), json!(""));
        }

        #[test]
        fn case_conversions_handle_whitespace_and_punctuation() {
            assert_eq!(
                kebab_case(&v(json!("Hello World!"))).unwrap(),
                json!("hello-world")
            );
            assert_eq!(
                snake_case(&v(json!("Hello World!"))).unwrap(),
                json!("hello_world")
            );
            assert_eq!(
                camel_case(&v(json!("hello world"))).unwrap(),
                json!("helloWorld")
            );
            assert_eq!(
                pascal_case(&v(json!("hello world"))).unwrap(),
                json!("HelloWorld")
            );
            assert_eq!(
                title_case(&v(json!("hello world"))).unwrap(),
                json!("Hello World")
            );
        }

        #[test]
        fn case_conversions_handle_existing_separators() {
            assert_eq!(
                kebab_case(&v(json!("foo_bar-baz"))).unwrap(),
                json!("foo-bar-baz")
            );
            assert_eq!(
                snake_case(&v(json!("fooBarBaz"))).unwrap(),
                json!("foo_bar_baz")
            );
            assert_eq!(
                camel_case(&v(json!("foo-bar_baz"))).unwrap(),
                json!("fooBarBaz")
            );
            assert_eq!(
                pascal_case(&v(json!("foo-bar_baz"))).unwrap(),
                json!("FooBarBaz")
            );
        }

        #[test]
        fn case_conversions_handle_acronyms() {
            assert_eq!(
                snake_case(&v(json!("XMLHttpRequest"))).unwrap(),
                json!("xml_http_request")
            );
            assert_eq!(
                kebab_case(&v(json!("XMLHttpRequest"))).unwrap(),
                json!("xml-http-request")
            );
        }

        #[test]
        fn case_conversions_empty_string() {
            assert_eq!(kebab_case(&v(json!(""))).unwrap(), json!(""));
            assert_eq!(snake_case(&v(json!(""))).unwrap(), json!(""));
            assert_eq!(camel_case(&v(json!(""))).unwrap(), json!(""));
            assert_eq!(pascal_case(&v(json!(""))).unwrap(), json!(""));
            assert_eq!(title_case(&v(json!(""))).unwrap(), json!(""));
        }

        #[test]
        fn mutations_null_propagates() {
            assert_eq!(lower(&v(json!(null))).unwrap(), json!(null));
            assert_eq!(upper(&v(json!(null))).unwrap(), json!(null));
            assert_eq!(capitalize(&v(json!(null))).unwrap(), json!(null));
            assert_eq!(kebab_case(&v(json!(null))).unwrap(), json!(null));
            assert_eq!(snake_case(&v(json!(null))).unwrap(), json!(null));
            assert_eq!(camel_case(&v(json!(null))).unwrap(), json!(null));
            assert_eq!(pascal_case(&v(json!(null))).unwrap(), json!(null));
            assert_eq!(title_case(&v(json!(null))).unwrap(), json!(null));
        }

        #[test]
        fn mutations_type_mismatch_errors() {
            assert!(lower(&v(json!(5))).is_err());
            assert!(upper(&v(json!(true))).is_err());
            assert!(kebab_case(&v(json!([]))).is_err());
        }
    }

    mod fn_isdate_strict {
        use super::*;

        #[test]
        fn isdate_accepts_iso_dates() {
            assert_eq!(is_date(&v(json!("2024-06-15"))).unwrap(), json!(true));
            assert_eq!(is_date_utc(&v(json!("2024-06-15"))).unwrap(), json!(true));
        }

        #[test]
        fn isdate_rejects_bad_formats() {
            assert_eq!(is_date(&v(json!("2024/06/15"))).unwrap(), json!(false));
            assert_eq!(is_date(&v(json!("06-15-2024"))).unwrap(), json!(false));
            assert_eq!(is_date(&v(json!("2024-13-01"))).unwrap(), json!(false));
            assert_eq!(
                is_date(&v(json!("2024-06-15T00:00:00"))).unwrap(),
                json!(false)
            );
        }

        #[test]
        fn isdate_rejects_non_strings_including_null() {
            assert_eq!(is_date(&v(json!(null))).unwrap(), json!(false));
            assert_eq!(is_date(&v(json!(20240615))).unwrap(), json!(false));
            assert_eq!(is_date(&v(json!(true))).unwrap(), json!(false));
            assert_eq!(is_date_utc(&v(json!(null))).unwrap(), json!(false));
        }

        #[test]
        fn isdatetime_accepts_iso_datetimes() {
            assert_eq!(
                is_datetime(&v(json!("2024-06-15T12:30:00"))).unwrap(),
                json!(true)
            );
            assert_eq!(
                is_datetime(&v(json!("2024-06-15T12:30:00Z"))).unwrap(),
                json!(true)
            );
            assert_eq!(
                is_datetime(&v(json!("2024-06-15T12:30:00+02:00"))).unwrap(),
                json!(true)
            );
            assert_eq!(
                is_datetime(&v(json!("2024-06-15T12:30:00.123"))).unwrap(),
                json!(true)
            );
            assert_eq!(
                is_datetime_utc(&v(json!("2024-06-15T12:30:00Z"))).unwrap(),
                json!(true)
            );
        }

        #[test]
        fn isdatetime_rejects_invalid_inputs() {
            assert_eq!(is_datetime(&v(json!("2024-06-15"))).unwrap(), json!(false));
            assert_eq!(is_datetime(&v(json!("nope"))).unwrap(), json!(false));
            assert_eq!(is_datetime(&v(json!(null))).unwrap(), json!(false));
            assert_eq!(is_datetime(&v(json!(123))).unwrap(), json!(false));
        }
    }

    mod fn_istoday_relative {
        use super::*;

        const TODAY: NaiveDate = match NaiveDate::from_ymd_opt(2024, 6, 15) {
            Some(d) => d,
            None => panic!(),
        };

        #[test]
        fn istoday_with_matches_today_date() {
            assert!(is_today_with(&json!("2024-06-15"), TODAY, false));
            assert!(!is_today_with(&json!("2024-06-14"), TODAY, false));
        }

        #[test]
        fn istoday_with_accepts_datetime_strings() {
            assert!(is_today_with(&json!("2024-06-15T08:00:00"), TODAY, false));
            assert!(is_today_with(&json!("2024-06-15T08:00:00Z"), TODAY, true));
        }

        #[test]
        fn istoday_with_rejects_non_strings_and_invalid() {
            assert!(!is_today_with(&json!(null), TODAY, false));
            assert!(!is_today_with(&json!(20240615), TODAY, false));
            assert!(!is_today_with(&json!("not-a-date"), TODAY, false));
        }

        #[test]
        fn isyesterday_with_matches_prev_day() {
            assert!(is_yesterday_with(&json!("2024-06-14"), TODAY, false));
            assert!(!is_yesterday_with(&json!("2024-06-15"), TODAY, false));
            assert!(!is_yesterday_with(&json!(null), TODAY, false));
        }

        #[test]
        fn istomorrow_with_matches_next_day() {
            assert!(is_tomorrow_with(&json!("2024-06-16"), TODAY, false));
            assert!(!is_tomorrow_with(&json!("2024-06-15"), TODAY, false));
        }

        #[test]
        fn isthismonth_with_matches_same_month_year() {
            assert!(is_this_month_with(&json!("2024-06-01"), TODAY, false));
            assert!(is_this_month_with(&json!("2024-06-30"), TODAY, false));
            assert!(!is_this_month_with(&json!("2024-07-01"), TODAY, false));
            assert!(!is_this_month_with(&json!("2023-06-15"), TODAY, false));
        }

        #[test]
        fn isthisyear_with_matches_same_year() {
            assert!(is_this_year_with(&json!("2024-01-01"), TODAY, false));
            assert!(is_this_year_with(&json!("2024-12-31"), TODAY, false));
            assert!(!is_this_year_with(&json!("2023-12-31"), TODAY, false));
        }

        #[test]
        fn relative_validators_accept_datetime_strings() {
            // "today" datetime in UTC.
            assert!(is_today_with(&json!("2024-06-15T23:59:00Z"), TODAY, true));
            // Datetime carrying an explicit offset that lands on a
            // different UTC date when assume_utc=true.
            assert!(is_today_with(
                &json!("2024-06-16T01:00:00+03:00"),
                TODAY,
                true
            ));
        }
    }
}
