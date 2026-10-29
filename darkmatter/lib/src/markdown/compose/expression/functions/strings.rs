//! String predicates and mutations: pure handlers over bound arguments.
//! Every text parameter declares `| null`; a transform passes null through
//! and a predicate answers `false`.

use serde_json::Value;

use super::{FunctionBinding, remove_date_substrings, scalar_string};
use crate::markdown::compose::expression::call_binding::BoundCall;
use crate::markdown::numeric;

pub(super) const BINDINGS: &[FunctionBinding] = &[
    FunctionBinding::pure("starts_with", &["startswith"], starts_with),
    FunctionBinding::pure("ends_with", &["endswith"], ends_with),
    FunctionBinding::pure("lower", &[], lower),
    FunctionBinding::pure("upper", &[], upper),
    FunctionBinding::pure("capitalize", &[], capitalize),
    FunctionBinding::pure("kebab_case", &["kebabcase"], kebab_case),
    FunctionBinding::pure("snake_case", &["snakecase"], snake_case),
    FunctionBinding::pure("camel_case", &["camelcase"], camel_case),
    FunctionBinding::pure("pascal_case", &["pascalcase"], pascal_case),
    FunctionBinding::pure("title_case", &["titlecase"], title_case),
    FunctionBinding::pure("without_date", &["withoutdate"], without_date),
    FunctionBinding::pure("ensure_leading", &["ensureleading"], ensure_leading),
    FunctionBinding::pure("ensure_trailing", &["ensuretrailing"], ensure_trailing),
    FunctionBinding::pure("replace", &[], replace),
    FunctionBinding::pure("replace_first", &["replacefirst"], replace_first),
    FunctionBinding::pure("replace_last", &["replacelast"], replace_last),
    FunctionBinding::pure("trim", &[], trim_fn),
];

/// A case-sensitive test of the two bound texts; null in either is `false`.
fn text_predicate(call: &BoundCall<'_>, test: fn(&str, &str) -> bool) -> Result<Value, String> {
    let args = call.args();
    Ok(Value::Bool(args.text(0).zip(args.text(1)).is_some_and(|(text, find)| test(text, find))))
}

fn starts_with(call: &BoundCall<'_>) -> Result<Value, String> {
    text_predicate(call, |text, find| text.starts_with(find))
}

fn ends_with(call: &BoundCall<'_>) -> Result<Value, String> {
    text_predicate(call, |text, find| text.ends_with(find))
}

/// Applies `transform` to the one bound text; null passes through.
fn transform(call: &BoundCall<'_>, transform: impl FnOnce(&str) -> String) -> Result<Value, String> {
    Ok(call.args().text(0).map_or(Value::Null, |text| Value::String(transform(text))))
}

fn lower(call: &BoundCall<'_>) -> Result<Value, String> {
    transform(call, str::to_lowercase)
}

fn upper(call: &BoundCall<'_>) -> Result<Value, String> {
    transform(call, str::to_uppercase)
}

/// `trim(text: string | null): string | null`: the text without leading and
/// trailing whitespace; null stays null.
pub(super) fn trim_fn(call: &BoundCall<'_>) -> Result<Value, String> {
    transform(call, |text| text.trim().to_string())
}

fn capitalize(call: &BoundCall<'_>) -> Result<Value, String> {
    transform(call, capitalized)
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
            let acronym_break =
                prev.is_uppercase() && c.is_uppercase() && i + 1 < chars.len() && chars[i + 1].is_lowercase();
            if lower_to_upper || alpha_digit_flip || acronym_break {
                flush(&mut current, &mut words);
            }
        }
        current.push(c);
    }
    flush(&mut current, &mut words);
    words
}

/// `word` with its first character uppercased.
fn capitalized(word: &str) -> String {
    let mut chars = word.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

fn kebab_case(call: &BoundCall<'_>) -> Result<Value, String> {
    transform(call, |text| split_words(text).join("-"))
}

fn snake_case(call: &BoundCall<'_>) -> Result<Value, String> {
    transform(call, |text| split_words(text).join("_"))
}

fn camel_case(call: &BoundCall<'_>) -> Result<Value, String> {
    transform(call, |text| {
        let mut words = split_words(text).into_iter();
        let mut out = words.next().unwrap_or_default();
        out.extend(words.map(|word| capitalized(&word)));
        out
    })
}

fn pascal_case(call: &BoundCall<'_>) -> Result<Value, String> {
    transform(call, |text| split_words(text).iter().map(|word| capitalized(word)).collect())
}

fn title_case(call: &BoundCall<'_>) -> Result<Value, String> {
    transform(call, |text| split_words(text).iter().map(|word| capitalized(word)).collect::<Vec<_>>().join(" "))
}

/// `without_date(string)`: removes strict `YYYY-MM-DD` substrings that parse
/// as real calendar dates.
fn without_date(call: &BoundCall<'_>) -> Result<Value, String> {
    transform(call, remove_date_substrings)
}

/// Where [`ensure_affix`] adds the affix.
#[derive(Clone, Copy)]
enum Side {
    Leading,
    Trailing,
}

/// `ensure_leading` / `ensure_trailing` over `string | number | null`
/// operands: compares text forms, returns `var` unchanged when the affix is
/// already there, and otherwise adds it. A number given a number (or text
/// spelling a number) stays a number when the joined text is itself an
/// admissible number spelling; every other result is text. Null in either
/// operand passes through.
fn ensure_affix(call: &BoundCall<'_>, side: Side) -> Result<Value, String> {
    let args = call.args();
    let (Some(var), Some(affix)) = (args.get(0), args.get(1)) else {
        return Ok(Value::Null);
    };
    if var.is_null() || affix.is_null() {
        return Ok(Value::Null);
    }
    let (var_text, affix_text) = (scalar_string(var), scalar_string(affix));
    let present = match side {
        Side::Leading => var_text.starts_with(&affix_text),
        Side::Trailing => var_text.ends_with(&affix_text),
    };
    if present {
        return Ok(var.clone());
    }
    let joined = match side {
        Side::Leading => format!("{affix_text}{var_text}"),
        Side::Trailing => format!("{var_text}{affix_text}"),
    };
    let numeric_affix = affix.is_number() || numeric::admit_number_spelling(&affix_text).is_ok();
    if var.is_number()
        && numeric_affix
        && let Some(number) = numeric::admit_number_spelling(&joined).ok().and_then(numeric::json_number)
    {
        return Ok(Value::Number(number));
    }
    Ok(Value::String(joined))
}

fn ensure_leading(call: &BoundCall<'_>) -> Result<Value, String> {
    ensure_affix(call, Side::Leading)
}

fn ensure_trailing(call: &BoundCall<'_>) -> Result<Value, String> {
    ensure_affix(call, Side::Trailing)
}

/// The three bound texts of a `replace*` call; `None` when any is null.
fn replace_parts<'a>(call: &'a BoundCall<'_>) -> Option<(&'a str, &'a str, &'a str)> {
    let args = call.args();
    Some((args.text(0)?, args.text(1)?, args.text(2)?))
}

/// Shared body for the `replace*` family: literal, case-sensitive matching;
/// an empty `find` is a no-op that returns `x` unchanged, deliberately
/// rejecting `str::replace`'s boundary-insertion behavior; null in any
/// position passes through.
fn replace_with(call: &BoundCall<'_>, apply: fn(&str, &str, &str) -> String) -> Result<Value, String> {
    Ok(match replace_parts(call) {
        Some((x, "", _)) => Value::String(x.to_string()),
        Some((x, find, replacement)) => Value::String(apply(x, find, replacement)),
        None => Value::Null,
    })
}

/// `replace(x, find, replacement)`: every non-overlapping occurrence.
fn replace(call: &BoundCall<'_>) -> Result<Value, String> {
    replace_with(call, |x, find, replacement| x.replace(find, replacement))
}

/// `replace_first(x, find, replacement)`: only the leftmost occurrence.
fn replace_first(call: &BoundCall<'_>) -> Result<Value, String> {
    replace_with(call, |x, find, replacement| x.replacen(find, replacement, 1))
}

/// `replace_last(x, find, replacement)`: only the rightmost occurrence.
/// `str::rfind` returns the match's start on a UTF-8 char boundary, so
/// slicing at it is safe.
fn replace_last(call: &BoundCall<'_>) -> Result<Value, String> {
    replace_with(call, |x, find, replacement| match x.rfind(find) {
        Some(position) => format!("{}{replacement}{}", &x[..position], &x[position + find.len()..]),
        None => x.to_string(),
    })
}
