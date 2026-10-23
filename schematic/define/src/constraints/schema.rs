//! Constraints as JSON Schema keywords, and exact numbers inside schemas.
//!
//! A schema stored as `serde_json::Value` holds numbers as `i64`, `u64`, or
//! `f64`. A numeric keyword whose exact value none of those can carry
//! (`18446744073709551616`, `0.10000000000000000001`) keeps the nearest
//! `f64` as a stand-in and records its exact decimal text in the
//! [`EXACT_EXTENSION`] object beside it:
//!
//! ```json
//! {"type": "integer", "maximum": 1.8446744073709552e19,
//!  "x-schematic-exact": {"maximum": "18446744073709551616"}}
//! ```
//!
//! Readers take the extension's text in preference to the number
//! ([`exact_number`]), and [`crate::openapi::serialize`] writes the exact text
//! back in place of the stand-in, so an exported document never carries the
//! extension.

use serde_json::{Map, Number, Value};

use super::number::Exact;
use super::{ConstraintNumber, Constraints};

/// The schema extension holding exact decimal text for numeric keywords
/// whose JSON number is only a stand-in.
pub const EXACT_EXTENSION: &str = "x-schematic-exact";

/// The numeric keywords whose values [`EXACT_EXTENSION`] may override.
pub const NUMERIC_KEYWORDS: [&str; 5] = ["minimum", "maximum", "exclusiveMinimum", "exclusiveMaximum", "multipleOf"];

/// The JSON number that means exactly `text`, if `serde_json` can hold one:
/// an `i64` or `u64` integer, or an `f64` whose shortest text has the same
/// decimal value (`0.1`, `1e30`, `-9223372036854776000`).
///
/// ## Examples
///
/// ```
/// use schematic_define::constraints::json_number;
///
/// assert_eq!(json_number("9223372036854775808").unwrap().as_u64(), Some(9223372036854775808));
/// assert_eq!(json_number("0.1").unwrap().as_f64(), Some(0.1));
/// assert!(json_number("18446744073709551616").is_none());
/// assert!(json_number("0.10000000000000000001").is_none());
/// ```
pub fn json_number(text: &str) -> Option<Number> {
    let exact = Exact::parse(text).ok()?;
    if let Ok(integer) = text.parse::<i64>() {
        return Some(integer.into());
    }
    if let Ok(integer) = text.parse::<u64>() {
        return Some(integer.into());
    }
    let number = Number::from_f64(text.parse::<f64>().ok()?)?;
    (Exact::parse(&number.to_string()).ok()? == exact).then_some(number)
}

/// The exact value of `keyword` in `schema`: the [`EXACT_EXTENSION`] text
/// when present, else the JSON number's own text. `None` when the keyword
/// is absent or not a number (an OpenAPI 3.0 Boolean `exclusiveMinimum`).
///
/// ## Examples
///
/// ```
/// use schematic_define::constraints::exact_number;
///
/// let schema = serde_json::json!({"maximum": 1.8446744073709552e19, "x-schematic-exact": {"maximum": "18446744073709551616"}});
/// assert_eq!(exact_number(schema.as_object().unwrap(), "maximum").unwrap().as_str(), "18446744073709551616");
/// ```
pub fn exact_number(schema: &Map<String, Value>, keyword: &str) -> Option<ConstraintNumber> {
    if let Some(text) = schema.get(EXACT_EXTENSION).and_then(|exact| exact.get(keyword)).and_then(Value::as_str) {
        return Some(ConstraintNumber::from(text));
    }
    match schema.get(keyword)? {
        Value::Number(number) => Some(ConstraintNumber::from(number_text(number))),
        _ => None,
    }
}

/// A JSON number's text, with a float in plain notation when that stays
/// short (`-9223372036854776000`, not `-9.223372036854776e18`): both are
/// the shortest text that reads back as the same `f64`, so the value is
/// the same either way.
fn number_text(number: &Number) -> String {
    match number.as_f64() {
        Some(float) if !(number.is_i64() || number.is_u64()) => {
            let plain = float.to_string();
            if plain.len() <= 32 { plain } else { number.to_string() }
        }
        _ => number.to_string(),
    }
}

/// Sets `keyword` in `schema` to `number`, as an exact JSON number when one
/// exists and otherwise as the nearest finite `f64` plus an
/// [`EXACT_EXTENSION`] entry.
pub(crate) fn set_number(schema: &mut Map<String, Value>, keyword: &str, number: &ConstraintNumber) {
    let text = number.as_str();
    match json_number(text) {
        Some(exact) => {
            schema.insert(keyword.to_owned(), Value::Number(exact));
            remove_exact_entry(schema, keyword);
        }
        None => {
            schema.insert(keyword.to_owned(), Value::Number(stand_in(text)));
            let entries = schema.entry(EXACT_EXTENSION).or_insert_with(|| Value::Object(Map::new()));
            if let Value::Object(entries) = entries {
                entries.insert(keyword.to_owned(), Value::String(text.to_owned()));
            }
        }
    }
}

/// The nearest finite `f64` to `text`; magnitudes beyond `f64` saturate.
fn stand_in(text: &str) -> Number {
    let value = text.parse::<f64>().unwrap_or(0.0);
    let value = if value.is_finite() { value } else { f64::MAX.copysign(value) };
    Number::from_f64(value).unwrap_or_else(|| Number::from(0))
}

fn remove_exact_entry(schema: &mut Map<String, Value>, keyword: &str) {
    if let Some(Value::Object(entries)) = schema.get_mut(EXACT_EXTENSION) {
        entries.remove(keyword);
        if entries.is_empty() {
            schema.remove(EXACT_EXTENSION);
        }
    }
}

#[cfg(feature = "openapi")]
/// Records an [`EXACT_EXTENSION`] entry for each numeric keyword of `schema`
/// whose JSON number would change value when read as an `f64`, the type
/// `openapiv3` reads most bounds into. Existing entries are kept.
pub(crate) fn mark_inexact_numbers(schema: &mut Map<String, Value>) {
    let inexact: Vec<(&str, String)> = NUMERIC_KEYWORDS
        .iter()
        .filter_map(|keyword| match schema.get(*keyword) {
            Some(Value::Number(number)) if !survives_f64(number) => Some((*keyword, number.to_string())),
            _ => None,
        })
        .filter(|(keyword, _)| schema.get(EXACT_EXTENSION).and_then(|exact| exact.get(*keyword)).is_none())
        .collect();
    if inexact.is_empty() {
        return;
    }
    let entries = schema.entry(EXACT_EXTENSION).or_insert_with(|| Value::Object(Map::new()));
    if let Value::Object(entries) = entries {
        for (keyword, text) in inexact {
            entries.insert(keyword.to_owned(), Value::String(text));
        }
    }
}

#[cfg(feature = "openapi")]
/// Whether `number`'s value is unchanged by a round trip through `f64` and
/// its shortest text.
fn survives_f64(number: &Number) -> bool {
    let Some(float) = number.as_f64().and_then(Number::from_f64) else { return false };
    match (Exact::parse(&number.to_string()), Exact::parse(&float.to_string())) {
        (Ok(original), Ok(read)) => original == read,
        _ => false,
    }
}

#[cfg(feature = "openapi")]
/// Rewrites OpenAPI 3.0 Boolean exclusive bounds throughout `schema` into
/// their numeric (JSON Schema 2020-12) meaning: `minimum: 5` with
/// `exclusiveMinimum: true` becomes `exclusiveMinimum: 5`. A `false` flag,
/// or a `true` flag without its bound, constrains nothing and is removed.
/// Data keywords (`example`, `default`, `enum`, `const`, extensions) are not
/// entered.
pub(crate) fn normalize_exclusive_flags(schema: &mut Value) {
    match schema {
        Value::Object(object) => {
            for (flag, bound) in [("exclusiveMinimum", "minimum"), ("exclusiveMaximum", "maximum")] {
                let Some(Value::Bool(exclusive)) = object.get(flag) else { continue };
                if *exclusive && let Some(limit) = object.remove(bound) {
                    object.insert(flag.to_owned(), limit);
                    if let Some(Value::Object(entries)) = object.get_mut(EXACT_EXTENSION)
                        && let Some(text) = entries.remove(bound)
                    {
                        entries.insert(flag.to_owned(), text);
                    }
                } else {
                    object.remove(flag);
                }
            }
            for (key, child) in object.iter_mut() {
                if !is_data_keyword(key) {
                    normalize_exclusive_flags(child);
                }
            }
        }
        Value::Array(items) => items.iter_mut().for_each(normalize_exclusive_flags),
        _ => {}
    }
}

#[cfg(feature = "openapi")]
/// Keywords whose values are data, never schemas: their contents are not
/// searched for keywords.
pub(crate) fn is_data_keyword(key: &str) -> bool {
    matches!(key, "example" | "examples" | "default" | "enum" | "const") || key.starts_with("x-")
}

impl Constraints {
    /// The rules as JSON Schema keywords, numeric exclusive bounds and
    /// `const` in their JSON Schema 2020-12 form; [`Self::items`] becomes an
    /// `items` object. Numbers no JSON number holds exactly carry an
    /// [`EXACT_EXTENSION`] entry.
    ///
    /// ## Examples
    ///
    /// ```
    /// use schematic_define::constraints::Constraints;
    ///
    /// let schema = Constraints::new().exclusive_minimum(0).max_items(3).to_schema();
    /// assert_eq!(serde_json::Value::Object(schema), serde_json::json!({"exclusiveMinimum": 0, "maxItems": 3}));
    /// ```
    pub fn to_schema(&self) -> Map<String, Value> {
        let mut schema = Map::new();
        let count = |schema: &mut Map<String, Value>, keyword: &str, value: Option<u64>| {
            if let Some(value) = value {
                schema.insert(keyword.to_owned(), Value::from(value));
            }
        };
        count(&mut schema, "minLength", self.min_length);
        count(&mut schema, "maxLength", self.max_length);
        if let Some(pattern) = &self.pattern {
            schema.insert("pattern".into(), Value::String(pattern.clone()));
        }
        if let Some(format) = &self.format {
            schema.insert("format".into(), Value::String(format.clone()));
        }
        for (keyword, number) in [
            ("minimum", &self.minimum),
            ("maximum", &self.maximum),
            ("exclusiveMinimum", &self.exclusive_minimum),
            ("exclusiveMaximum", &self.exclusive_maximum),
            ("multipleOf", &self.multiple_of),
        ] {
            if let Some(number) = number {
                set_number(&mut schema, keyword, number);
            }
        }
        count(&mut schema, "minItems", self.min_items);
        count(&mut schema, "maxItems", self.max_items);
        if self.unique_items {
            schema.insert("uniqueItems".into(), Value::Bool(true));
        }
        if let Some(values) = &self.allowed_values {
            schema.insert("enum".into(), Value::Array(values.clone()));
        }
        if let Some(constant) = &self.constant {
            schema.insert("const".into(), constant.clone());
        }
        if let Some(items) = &self.items {
            schema.insert("items".into(), Value::Object(items.to_schema()));
        }
        schema
    }

    /// The supported rules of a JSON Schema or OpenAPI 3.0 schema object.
    ///
    /// Reads both exclusive-bound spellings (a 2020-12 number, or a 3.0
    /// Boolean flag with its bound), [`EXACT_EXTENSION`] text, and an
    /// `items` object's rules. A pattern is recorded as enforced; callers
    /// that import it decide with [`super::admit_pattern`]. Keywords outside
    /// the supported set, and values of the wrong JSON type, are not read.
    ///
    /// ## Examples
    ///
    /// ```
    /// use schematic_define::constraints::Constraints;
    ///
    /// let schema = serde_json::json!({"type": "integer", "minimum": 0, "exclusiveMinimum": true, "maximum": 100});
    /// let rules = Constraints::from_schema(schema.as_object().unwrap());
    /// assert_eq!(rules, Constraints::new().exclusive_minimum(0).maximum(100));
    /// ```
    pub fn from_schema(schema: &Map<String, Value>) -> Self {
        let count = |keyword: &str| schema.get(keyword).and_then(Value::as_u64);
        let text = |keyword: &str| schema.get(keyword).and_then(Value::as_str).map(str::to_owned);
        let flagged = |flag: &str| schema.get(flag) == Some(&Value::Bool(true));
        let mut rules = Self {
            min_length: count("minLength"),
            max_length: count("maxLength"),
            pattern: text("pattern"),
            format: text("format"),
            min_items: count("minItems"),
            max_items: count("maxItems"),
            unique_items: schema.get("uniqueItems") == Some(&Value::Bool(true)),
            allowed_values: schema.get("enum").and_then(Value::as_array).cloned(),
            constant: schema.get("const").cloned(),
            multiple_of: exact_number(schema, "multipleOf"),
            exclusive_minimum: exact_number(schema, "exclusiveMinimum"),
            exclusive_maximum: exact_number(schema, "exclusiveMaximum"),
            ..Self::default()
        };
        let minimum = exact_number(schema, "minimum");
        let maximum = exact_number(schema, "maximum");
        if flagged("exclusiveMinimum") { rules.exclusive_minimum = minimum } else { rules.minimum = minimum }
        if flagged("exclusiveMaximum") { rules.exclusive_maximum = maximum } else { rules.maximum = maximum }
        if let Some(Value::Object(items)) = schema.get("items") {
            let items = Self::from_schema(items);
            if !items.is_empty() {
                rules.items = Some(Box::new(items));
            }
        }
        rules
    }
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "openapi")]
    use serde_json::json;

    use super::*;

    #[test]
    fn exact_numbers_round_trip_through_a_schema() {
        for text in ["100", "-9223372036854775809", "18446744073709551616", "0.1", "0.10000000000000000001", "1e400"] {
            let mut schema = Map::new();
            set_number(&mut schema, "maximum", &ConstraintNumber::from(text));
            assert_eq!(exact_number(&schema, "maximum").unwrap().as_str(), text);
            assert!(schema["maximum"].is_number(), "{text}: a stand-in number is always present");
        }
    }

    #[cfg(feature = "openapi")]
    #[test]
    fn only_inexact_numbers_are_marked() {
        let mut schema = json!({"minimum": 1, "maximum": 9223372036854775807_i64, "multipleOf": 0.1}).as_object().unwrap().clone();
        mark_inexact_numbers(&mut schema);
        assert_eq!(schema[EXACT_EXTENSION], json!({"maximum": "9223372036854775807"}));
    }

    #[cfg(feature = "openapi")]
    #[test]
    fn boolean_exclusive_flags_become_numeric_bounds() {
        let mut schema = json!({"minimum": 0, "exclusiveMinimum": true, "maximum": 5, "exclusiveMaximum": false,
            "properties": {"a": {"maximum": 1, "exclusiveMaximum": true}}, "example": {"minimum": 1, "exclusiveMinimum": true}});
        normalize_exclusive_flags(&mut schema);
        assert_eq!(schema, json!({"exclusiveMinimum": 0, "maximum": 5,
            "properties": {"a": {"exclusiveMaximum": 1}}, "example": {"minimum": 1, "exclusiveMinimum": true}}));
    }
}
