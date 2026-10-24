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
    /// the supported set are not read, and an `items` list (positional
    /// items) or Boolean `items` schema carries no item rules.
    ///
    /// This checks each keyword's JSON type only; [`schema_rule_problems`]
    /// adds the checks between keywords (`minLength` above `maxLength`, a
    /// nonpositive `multipleOf`, an empty `enum`, an invalid `pattern`).
    ///
    /// ## Examples
    ///
    /// ```
    /// use schematic_define::constraints::Constraints;
    ///
    /// let schema = serde_json::json!({"type": "integer", "minimum": 0, "exclusiveMinimum": true, "maximum": 100});
    /// let rules = Constraints::from_schema(schema.as_object().unwrap()).unwrap();
    /// assert_eq!(rules, Constraints::new().exclusive_minimum(0).maximum(100));
    ///
    /// let typo = serde_json::json!({"type": "string", "maxLength": "3"});
    /// let problems = Constraints::from_schema(typo.as_object().unwrap()).unwrap_err();
    /// assert_eq!((problems[0].pointer.as_str(), problems[0].keyword), ("", "maxLength"));
    /// ```
    ///
    /// ## Errors
    ///
    /// Every keyword whose value has the wrong JSON type, including an
    /// explicit `null`: a count that is not a nonnegative integer, a
    /// non-string `pattern` or `format`, a non-numeric bound, a non-Boolean
    /// `uniqueItems`, a non-array `enum`, an `items` that is not a schema,
    /// or an [`EXACT_EXTENSION`] entry that is not text. `const` holds any
    /// JSON value, `null` included.
    pub fn from_schema(schema: &Map<String, Value>) -> Result<Self, Vec<SchemaRuleProblem>> {
        let mut problems = Vec::new();
        let rules = read_rules(schema, "", true, &mut problems);
        if problems.is_empty() { Ok(rules) } else { Err(problems) }
    }
}

/// One rule keyword of a stored schema that is malformed or contradicts
/// another; see [`Constraints::from_schema`] and [`schema_rule_problems`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaRuleProblem {
    /// JSON pointer, inside the schema read, to the schema object holding
    /// the keyword; `""` is that schema's root.
    pub pointer: String,
    /// The JSON Schema keyword at fault (`maxLength`, `enum`, ...).
    pub keyword: &'static str,
    /// Why the schema cannot carry it.
    pub reason: String,
}

/// Every malformed or contradictory rule in the schema `schema` and each
/// of its subschemas, in document order: the keyword type checks of
/// [`Constraints::from_schema`], then the same checks between keywords
/// that endpoint constraints get (a lower bound above its upper bound, an
/// empty interval, a nonpositive `multipleOf`, an empty `enum` or
/// `format`), and a `pattern` that is not a valid regular expression.
///
/// A valid pattern outside the enforced subset is not a problem here; the
/// generator reports it as unenforced. `enum` and `const` members are
/// whole JSON values of any type. Data keywords (`default`, `example`,
/// `enum`, `const`, extensions) are never searched for subschemas.
///
/// ## Examples
///
/// ```
/// use schematic_define::constraints::schema_rule_problems;
///
/// let schema = serde_json::json!({"type": "object", "properties": {
///     "code": {"type": "string", "minLength": 4, "maxLength": 3},
///     "mode": {"enum": ["a", 1, null]},
/// }});
/// let problems = schema_rule_problems(&schema);
/// assert_eq!(problems.len(), 1);
/// assert_eq!((problems[0].pointer.as_str(), problems[0].keyword), ("/properties/code", "minLength"));
/// ```
pub fn schema_rule_problems(schema: &Value) -> Vec<SchemaRuleProblem> {
    let mut problems = Vec::new();
    walk_rules(schema, &mut String::new(), Walk::All, &mut problems);
    problems
}

/// [`schema_rule_problems`] without the declared-type check, for an
/// imported input schema whose rules a later check fits to the input's own
/// type (see `endpoint_problems`).
#[cfg(feature = "openapi")]
pub(crate) fn untyped_schema_rule_problems(schema: &Value) -> Vec<SchemaRuleProblem> {
    let mut problems = Vec::new();
    walk_rules(schema, &mut String::new(), Walk::Untyped, &mut problems);
    problems
}

/// Removes from `schema`, and returns, every rule keyword that applies to
/// none of its schema object's declared `type` names (the problems
/// [`schema_rule_problems`] reports as "does not apply to the declared
/// type"), with its exact-decimal entry.
#[cfg(feature = "openapi")]
pub(crate) fn remove_inapplicable_rules(schema: &mut Value) -> Vec<SchemaRuleProblem> {
    let mut problems = Vec::new();
    walk_rules(schema, &mut String::new(), Walk::Typed, &mut problems);
    for problem in &problems {
        let Some(Value::Object(object)) = schema.pointer_mut(&problem.pointer) else { continue };
        let mut keywords = vec![problem.keyword];
        // An OpenAPI 3.0 exclusive flag's bound is its `minimum`/`maximum`.
        if object.remove(problem.keyword) == Some(Value::Bool(true)) {
            let bound = if problem.keyword == "exclusiveMinimum" { "minimum" } else { "maximum" };
            object.remove(bound);
            keywords.push(bound);
        }
        if let Some(Value::Object(exact)) = object.get_mut(EXACT_EXTENSION) {
            for keyword in keywords { exact.remove(keyword); }
            if exact.is_empty() { object.remove(EXACT_EXTENSION); }
        }
    }
    problems
}

/// Which problems [`walk_rules`] reports.
#[derive(Clone, Copy, PartialEq, Eq)]
#[cfg_attr(not(feature = "openapi"), allow(dead_code, reason = "only the OpenAPI importer walks a subset"))]
enum Walk {
    /// Every malformed, contradictory, or type-inapplicable rule.
    All,
    /// All but the type-inapplicable rules.
    Untyped,
    /// Only the type-inapplicable rules.
    Typed,
}

/// Keywords whose value is one subschema.
const SCHEMA_KEYWORDS: [&str; 10] = [
    "additionalProperties", "not", "contains", "propertyNames", "if", "then", "else", "unevaluatedItems",
    "unevaluatedProperties", "additionalItems",
];

/// Keywords whose value is one subschema or a list of them.
const SCHEMA_LIST_KEYWORDS: [&str; 5] = ["items", "prefixItems", "allOf", "anyOf", "oneOf"];

/// Keywords whose value maps names to subschemas. A `dependencies` entry
/// may instead list property names, which holds no schema.
const SCHEMA_MAP_KEYWORDS: [&str; 6] = ["properties", "patternProperties", "$defs", "definitions", "dependentSchemas", "dependencies"];

fn walk_rules(value: &Value, pointer: &mut String, walk: Walk, problems: &mut Vec<SchemaRuleProblem>) {
    let Value::Object(object) = value else { return };
    let before = problems.len();
    let mut malformed = Vec::new();
    let rules = read_rules(object, pointer, false, &mut malformed);
    if walk != Walk::Typed {
        problems.extend(malformed);
        // A keyword of the wrong type is absent from `rules`, so the checks
        // between keywords see only the well-formed rest. Marking the pattern
        // unenforced leaves only its validity to check: whether it lies in the
        // enforced subset is the generator's warning, not a definition error.
        let mut checked = rules.clone();
        checked.pattern_unenforced = checked.pattern.is_some();
        for (_, keyword, reason) in super::check::check(&checked, &super::check::Target::of(super::check::Kind::Any), "") {
            problems.push(SchemaRuleProblem { pointer: pointer.clone(), keyword, reason });
        }
    }
    // The declared `type`, as authored endpoint metadata is checked against
    // its input's type; no `type` declares nothing to check.
    let types: Vec<&str> = match object.get("type") {
        Some(Value::String(name)) => vec![name.as_str()],
        Some(Value::Array(names)) => names.iter().filter_map(Value::as_str).collect(),
        _ => Vec::new(),
    };
    if walk != Walk::Untyped && !types.is_empty() {
        // A keyword already malformed or contradictory is reported once.
        for (keyword, reason) in super::check::inapplicable_to_types(&rules, &types) {
            if !problems[before..].iter().any(|problem| problem.keyword == keyword) {
                problems.push(SchemaRuleProblem { pointer: pointer.clone(), keyword, reason });
            }
        }
    }
    problems[before..].sort_by_key(|problem| keyword_order(problem.keyword));

    for (key, child) in object {
        let key = key.as_str();
        let length = pointer.len();
        push_token(pointer, key);
        if SCHEMA_KEYWORDS.contains(&key) {
            walk_rules(child, pointer, walk, problems);
        } else if SCHEMA_LIST_KEYWORDS.contains(&key) {
            match child {
                Value::Array(members) => {
                    for (index, member) in members.iter().enumerate() {
                        let length = pointer.len();
                        push_token(pointer, &index.to_string());
                        walk_rules(member, pointer, walk, problems);
                        pointer.truncate(length);
                    }
                }
                child => walk_rules(child, pointer, walk, problems),
            }
        } else if SCHEMA_MAP_KEYWORDS.contains(&key)
            && let Value::Object(members) = child
        {
            for (name, member) in members {
                let length = pointer.len();
                push_token(pointer, name);
                walk_rules(member, pointer, walk, problems);
                pointer.truncate(length);
            }
        }
        pointer.truncate(length);
    }
}

/// The position of `keyword` among [`read_rules`]' keywords, so one
/// object's problems are reported in a fixed order.
fn keyword_order(keyword: &str) -> usize {
    RULE_ORDER.iter().position(|known| *known == keyword).unwrap_or(RULE_ORDER.len())
}

const RULE_ORDER: [&str; 16] = [
    "minLength", "maxLength", "pattern", "format", "minimum", "maximum", "exclusiveMinimum", "exclusiveMaximum",
    "multipleOf", EXACT_EXTENSION, "minItems", "maxItems", "uniqueItems", "enum", "const", "items",
];

fn push_token(pointer: &mut String, token: &str) {
    pointer.push('/');
    pointer.push_str(&token.replace('~', "~0").replace('/', "~1"));
}

/// The rules of `schema`, found at `pointer`, recording each keyword of
/// the wrong JSON type in `problems` and leaving it out. With `nested`, an
/// `items` object's rules are read too.
fn read_rules(schema: &Map<String, Value>, pointer: &str, nested: bool, problems: &mut Vec<SchemaRuleProblem>) -> Constraints {
    let mut fail = |keyword: &'static str, expected: &str, value: &Value| {
        problems.push(SchemaRuleProblem { pointer: pointer.to_owned(), keyword, reason: format!("must be {expected}, not {}", json_kind(value)) });
    };
    let mut count = |keyword: &'static str| match schema.get(keyword) {
        Some(value) => value.as_u64().or_else(|| { fail(keyword, "a nonnegative integer", value); None }),
        None => None,
    };
    let min_length = count("minLength");
    let max_length = count("maxLength");
    let min_items = count("minItems");
    let max_items = count("maxItems");
    let mut text = |keyword: &'static str| match schema.get(keyword) {
        Some(Value::String(text)) => Some(text.clone()),
        Some(value) => { fail(keyword, "a string", value); None }
        None => None,
    };
    let pattern = text("pattern");
    let format = text("format");
    let unique_items = match schema.get("uniqueItems") {
        Some(Value::Bool(flag)) => *flag,
        Some(value) => { fail("uniqueItems", "a Boolean", value); false }
        None => false,
    };
    let allowed_values = match schema.get("enum") {
        Some(Value::Array(values)) => Some(values.clone()),
        Some(value) => { fail("enum", "an array of values", value); None }
        None => None,
    };
    let exact = match schema.get(EXACT_EXTENSION) {
        Some(Value::Object(entries)) => {
            for (keyword, text) in entries {
                if let Some(keyword) = NUMERIC_KEYWORDS.iter().find(|known| *known == keyword)
                    && !text.is_string() { fail(keyword, "exact decimal text", text) }
            }
            true
        }
        Some(value) => { fail(EXACT_EXTENSION, "an object of exact decimal text", value); false }
        None => false,
    };
    let mut number = |keyword: &'static str, flag: bool| {
        if exact && let Some(Value::String(text)) = schema.get(EXACT_EXTENSION).and_then(|entries| entries.get(keyword)) {
            return Some(ConstraintNumber::from(text.as_str()));
        }
        match schema.get(keyword)? {
            Value::Number(number) => Some(ConstraintNumber::from(number_text(number))),
            // An OpenAPI 3.0 exclusive-bound flag, read with its bound below.
            Value::Bool(_) if flag => None,
            value => { fail(keyword, if flag { "a number or an OpenAPI 3.0 Boolean flag" } else { "a number" }, value); None }
        }
    };
    let minimum = number("minimum", false);
    let maximum = number("maximum", false);
    let mut rules = Constraints {
        exclusive_minimum: number("exclusiveMinimum", true),
        exclusive_maximum: number("exclusiveMaximum", true),
        multiple_of: number("multipleOf", false),
        ..Constraints::default()
    };
    let flagged = |flag: &str| schema.get(flag) == Some(&Value::Bool(true));
    if flagged("exclusiveMinimum") { rules.exclusive_minimum = minimum } else { rules.minimum = minimum }
    if flagged("exclusiveMaximum") { rules.exclusive_maximum = maximum } else { rules.maximum = maximum }
    rules.min_length = min_length;
    rules.max_length = max_length;
    rules.pattern = pattern;
    rules.format = format;
    rules.min_items = min_items;
    rules.max_items = max_items;
    rules.unique_items = unique_items;
    rules.allowed_values = allowed_values;
    rules.constant = schema.get("const").cloned();
    match schema.get("items") {
        Some(Value::Object(items)) if nested => {
            let items = read_rules(items, &format!("{pointer}/items"), true, problems);
            if !items.is_empty() { rules.items = Some(Box::new(items)); }
        }
        Some(Value::Object(_) | Value::Array(_) | Value::Bool(_)) | None => {}
        Some(value) => problems.push(SchemaRuleProblem {
            pointer: pointer.to_owned(), keyword: "items", reason: format!("must be a schema, not {}", json_kind(value)),
        }),
    }
    rules
}

/// The JSON kind of `value`, for a problem's reason.
fn json_kind(value: &Value) -> String {
    match value {
        Value::Null => "null".to_owned(),
        Value::Bool(flag) => format!("the Boolean {flag}"),
        Value::Number(number) => format!("the number {number}"),
        Value::String(_) => "a string".to_owned(),
        Value::Array(_) => "an array".to_owned(),
        Value::Object(_) => "an object".to_owned(),
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
