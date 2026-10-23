//! Source-text work done before an OpenAPI document is deserialized.
//!
//! `openapiv3` reads numeric schema keywords as `i64` or `f64` and drops
//! `const`, which OpenAPI 3.0 does not define. [`SourceKeywords::capture`]
//! first records each numeric keyword's exact decimal text and each `const`
//! value by JSON pointer, and [`stand_in_bounds`] then makes the text
//! parseable. The importer reads constraint values from the capture, never
//! from the narrowed parse.

use std::collections::BTreeMap;
use std::fmt;

use biscuit_file::serde_yaml_ng;
use serde::de::{DeserializeSeed, Deserializer, Error as _, IgnoredAny, MapAccess, SeqAccess, Visitor};
use serde_json::{Map, Number, Value};

use super::super::error::OpenApiError;
use crate::constraints::{ConstraintNumber, NUMERIC_KEYWORDS, is_data_keyword, set_number};

/// Exact source values of the keywords `openapiv3` narrows or drops, keyed
/// by the RFC 6901 pointer (`#/components/schemas/Pet/properties/age/maximum`)
/// of the keyword itself.
#[derive(Debug, Default)]
pub(crate) struct SourceKeywords {
    numbers: BTreeMap<String, String>,
    constants: BTreeMap<String, Value>,
}

/// The capture of a document that was not read from text.
pub(crate) static NO_SOURCE_KEYWORDS: SourceKeywords = SourceKeywords { numbers: BTreeMap::new(), constants: BTreeMap::new() };

/// Value keywords that `openapiv3` reads as an `Option`, so an explicit
/// `null` would silently read as absent; the capture refuses one instead.
const NULL_CHECKED: &[&str] = &["minLength", "maxLength", "minItems", "maxItems", "uniqueItems", "pattern", "format", "enum"];

/// Object keys whose children are names, not schema keywords: a property
/// called `minimum` is a schema, not a bound.
const NAME_MAPS: &[&str] = &[
    "properties", "patternProperties", "$defs", "definitions", "schemas", "parameters", "responses", "requestBodies",
    "headers", "securitySchemes", "links", "callbacks", "content", "encoding", "mapping", "variables", "paths", "scopes",
];

impl SourceKeywords {
    /// Captures `text`, parsed as JSON when `json` is set and as YAML
    /// otherwise.
    ///
    /// ## Errors
    ///
    /// [`OpenApiError::Validation`] at the keyword's pointer for a numeric
    /// keyword that no exact decimal can represent (YAML's `.inf`/`.nan`, or
    /// an exponent beyond ±10 000), and for an explicit `null` value keyword,
    /// which `openapiv3` would read as absent. A document the capture cannot read at all
    /// is [`OpenApiError::Parse`].
    pub(crate) fn capture(text: &str, json: bool) -> Result<Self, OpenApiError> {
        let mut state = Capture::default();
        let walker = Walker { state: &mut state, pointer: "#".to_owned(), names: false, json };
        let outcome = if json {
            walker.deserialize(&mut serde_json::Deserializer::from_str(text)).map_err(|error| error.to_string())
        } else {
            walker.deserialize(serde_yaml_ng::Deserializer::from_str(text)).map_err(|error| error.to_string())
        };
        if let Some((path, message)) = state.unrepresentable {
            return Err(OpenApiError::Validation { path, message });
        }
        outcome.map_err(|message| OpenApiError::Parse { message, location: None })?;
        Ok(state.keywords)
    }

    /// The `openapiv3` schema value at `pointer`, serialized, with the
    /// captured exact numbers and `const` values restored and OpenAPI 3.0
    /// Boolean exclusive bounds rewritten to their numeric meaning.
    pub(crate) fn source_schema(&self, schema: &impl serde::Serialize, pointer: &str) -> Value {
        // An `openapiv3` schema always serializes; `{}` keeps the result a schema.
        let mut value = serde_json::to_value(schema).unwrap_or_else(|_| Value::Object(Map::new()));
        self.restore(&mut value, pointer);
        crate::constraints::normalize_exclusive_flags(&mut value);
        value
    }

    /// Restores captured values throughout the subtree `value` found at
    /// `pointer`. A number replaces only a keyword the parse kept; a `const`
    /// is added back.
    fn restore(&self, value: &mut Value, pointer: &str) {
        match value {
            Value::Object(object) => {
                for keyword in NUMERIC_KEYWORDS {
                    if object.get(keyword).is_some_and(Value::is_number)
                        && let Some(text) = self.numbers.get(&format!("{pointer}/{keyword}"))
                    {
                        set_number(object, keyword, &ConstraintNumber::from(text.as_str()));
                    }
                }
                if let Some(constant) = self.constants.get(&format!("{pointer}/const")) {
                    object.insert("const".to_owned(), constant.clone());
                }
                for (key, child) in object.iter_mut() {
                    if !is_data_keyword(key) {
                        self.restore(child, &format!("{pointer}/{}", escape(key)));
                    }
                }
            }
            Value::Array(items) => {
                for (index, item) in items.iter_mut().enumerate() {
                    self.restore(item, &format!("{pointer}/{index}"));
                }
            }
            _ => {}
        }
    }
}

/// Escapes one RFC 6901 reference token.
fn escape(token: &str) -> String {
    token.replace('~', "~0").replace('/', "~1")
}

#[derive(Default)]
struct Capture {
    keywords: SourceKeywords,
    unrepresentable: Option<(String, String)>,
}

/// Walks one value at `pointer`; `names` marks an object whose keys are
/// names rather than keywords.
struct Walker<'a> {
    state: &'a mut Capture,
    pointer: String,
    names: bool,
    json: bool,
}

impl<'de> DeserializeSeed<'de> for Walker<'_> {
    type Value = ();

    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<(), D::Error> {
        deserializer.deserialize_any(self)
    }
}

impl<'de> Visitor<'de> for Walker<'_> {
    type Value = ();

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("an OpenAPI document")
    }

    fn visit_map<A: MapAccess<'de>>(mut self, mut map: A) -> Result<(), A::Error> {
        while let Some(key) = map.next_key::<String>()? {
            let pointer = format!("{}/{}", self.pointer, escape(&key));
            if !self.names && NUMERIC_KEYWORDS.contains(&key.as_str()) {
                let text = if self.json {
                    map.next_value::<Box<serde_json::value::RawValue>>()?.get().trim().to_owned()
                } else {
                    map.next_value_seed(ScalarText)?
                };
                self.record_number(pointer, &text).map_err(A::Error::custom)?;
            } else if !self.names && NULL_CHECKED.contains(&key.as_str()) {
                if map.next_value_seed(ValueSeed)?.is_null() {
                    let message = format!("`{key}` is null; a keyword is either absent or holds a value");
                    self.state.unrepresentable = Some((pointer, message.clone()));
                    return Err(A::Error::custom(message));
                }
            } else if !self.names && key == "const" {
                let constant = map.next_value_seed(ValueSeed)?;
                self.state.keywords.constants.insert(pointer, constant);
            } else if !self.names && is_data_keyword(&key) {
                map.next_value::<IgnoredAny>()?;
            } else {
                let names = !self.names && NAME_MAPS.contains(&key.as_str());
                map.next_value_seed(Walker { state: &mut *self.state, pointer, names, json: self.json })?;
            }
        }
        Ok(())
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<(), A::Error> {
        let mut index = 0;
        while seq
            .next_element_seed(Walker { state: &mut *self.state, pointer: format!("{}/{index}", self.pointer), names: false, json: self.json })?
            .is_some()
        {
            index += 1;
        }
        Ok(())
    }

    fn visit_bool<E>(self, _: bool) -> Result<(), E> { Ok(()) }
    fn visit_i64<E>(self, _: i64) -> Result<(), E> { Ok(()) }
    fn visit_u64<E>(self, _: u64) -> Result<(), E> { Ok(()) }
    fn visit_i128<E>(self, _: i128) -> Result<(), E> { Ok(()) }
    fn visit_u128<E>(self, _: u128) -> Result<(), E> { Ok(()) }
    fn visit_f64<E>(self, _: f64) -> Result<(), E> { Ok(()) }
    fn visit_str<E>(self, _: &str) -> Result<(), E> { Ok(()) }
    fn visit_unit<E>(self) -> Result<(), E> { Ok(()) }
    fn visit_none<E>(self) -> Result<(), E> { Ok(()) }
    fn visit_some<D: Deserializer<'de>>(self, deserializer: D) -> Result<(), D::Error> {
        deserializer.deserialize_any(self)
    }
}

impl Walker<'_> {
    /// Records a numeric keyword's text. Text outside the JSON number
    /// grammar that still names a number (`0x1F`, `+5`) is left to the
    /// parse, which reads it exactly; text naming no finite decimal fails.
    fn record_number(&mut self, pointer: String, text: &str) -> Result<(), String> {
        if matches!(text, "null" | "Null" | "NULL" | "~" | "") {
            self.state.unrepresentable = Some((pointer, "the numeric bound is null; a keyword is either absent or holds a value".to_owned()));
            return Err("null numeric bound".to_owned());
        }
        match ConstraintNumber::from(text).exact() {
            Ok(_) => {
                self.state.keywords.numbers.insert(pointer, text.to_owned());
            }
            Err(reason) if reason.contains("exponent") => {
                self.state.unrepresentable = Some((pointer, format!("the numeric bound {text} {reason}, so no exact decimal can represent it")));
                return Err(format!("unrepresentable numeric bound {text}"));
            }
            Err(_) if is_non_finite(text) => {
                self.state.unrepresentable = Some((pointer, format!("the numeric bound {text} is not a finite number")));
                return Err(format!("unrepresentable numeric bound {text}"));
            }
            Err(_) => {}
        }
        Ok(())
    }
}

/// YAML's spellings of infinity and not-a-number.
fn is_non_finite(text: &str) -> bool {
    let unsigned = text.strip_prefix(['-', '+']).unwrap_or(text);
    matches!(unsigned.to_ascii_lowercase().as_str(), ".inf" | ".nan")
}

/// A YAML scalar's text as written, before YAML resolves it to a number.
struct ScalarText;

impl<'de> DeserializeSeed<'de> for ScalarText {
    type Value = String;

    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<String, D::Error> {
        struct Text;
        impl Visitor<'_> for Text {
            type Value = String;
            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a scalar")
            }
            fn visit_str<E>(self, value: &str) -> Result<String, E> { Ok(value.to_owned()) }
        }
        // `serde_yaml_ng` hands a plain scalar's source text to `visit_str`.
        deserializer.deserialize_str(Text)
    }
}

/// A `const` value as JSON; an integer beyond 64 bits becomes the nearest
/// `f64`, as `serde_json` itself reads one.
struct ValueSeed;

impl<'de> DeserializeSeed<'de> for ValueSeed {
    type Value = Value;

    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<Value, D::Error> {
        deserializer.deserialize_any(self)
    }
}

impl<'de> Visitor<'de> for ValueSeed {
    type Value = Value;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a JSON value")
    }

    fn visit_bool<E>(self, value: bool) -> Result<Value, E> { Ok(Value::Bool(value)) }
    fn visit_i64<E>(self, value: i64) -> Result<Value, E> { Ok(Value::from(value)) }
    fn visit_u64<E>(self, value: u64) -> Result<Value, E> { Ok(Value::from(value)) }
    fn visit_i128<E>(self, value: i128) -> Result<Value, E> { Ok(float(value as f64)) }
    fn visit_u128<E>(self, value: u128) -> Result<Value, E> { Ok(float(value as f64)) }
    fn visit_f64<E>(self, value: f64) -> Result<Value, E> { Ok(float(value)) }
    fn visit_str<E>(self, value: &str) -> Result<Value, E> { Ok(Value::String(value.to_owned())) }
    fn visit_unit<E>(self) -> Result<Value, E> { Ok(Value::Null) }
    fn visit_none<E>(self) -> Result<Value, E> { Ok(Value::Null) }
    fn visit_some<D: Deserializer<'de>>(self, deserializer: D) -> Result<Value, D::Error> {
        deserializer.deserialize_any(self)
    }
    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Value, A::Error> {
        let mut items = Vec::new();
        while let Some(item) = seq.next_element_seed(ValueSeed)? {
            items.push(item);
        }
        Ok(Value::Array(items))
    }
    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Value, A::Error> {
        let mut object = Map::new();
        while let Some(key) = map.next_key::<String>()? {
            object.insert(key, map.next_value_seed(ValueSeed)?);
        }
        Ok(Value::Object(object))
    }
}

fn float(value: f64) -> Value {
    Number::from_f64(value).map_or(Value::Null, Value::Number)
}

/// Numeric schema keywords whose source integers can overflow `i64`.
const STAND_IN_KEYS: [&str; 5] = NUMERIC_KEYWORDS;

/// Replaces each out-of-`i64`-range integer bound in raw specification text
/// with the nearest `i64`, so the document parses and an integer schema
/// keeps its `integer` type.
///
/// The replacement is a parse stand-in, not the bound: the exact value was
/// captured beforehand by [`SourceKeywords::capture`] and replaces the stand-in
/// wherever the importer reads it. Only `key: value` lines (YAML, or JSON
/// with one member per line) are rewritten; indentation, trailing commas,
/// and comments are kept.
///
/// ## Notes
///
/// OpenAI's published specification writes its `seed` bounds as
/// `-9223372036854776000` / `9223372036854776000`. `serde_yaml_ng` routes
/// integers outside `i64` to `Visitor::visit_i128`, which `openapiv3` does not
/// accept, so the document would not parse at all without a stand-in.
pub(crate) fn stand_in_bounds(text: &str) -> String {
    // Specs are large enough (OpenAI's is 2.8 MB) that the line scan is worth
    // skipping when no candidate keyword appears.
    if !STAND_IN_KEYS.iter().any(|key| text.contains(key)) {
        return text.to_string();
    }

    let mut out = String::with_capacity(text.len());
    for (index, line) in text.lines().enumerate() {
        if index > 0 {
            out.push('\n');
        }
        match stand_in_line(line) {
            Some(rewritten) => out.push_str(&rewritten),
            None => out.push_str(line),
        }
    }
    if text.ends_with('\n') {
        out.push('\n');
    }
    out
}

/// Rewrites a single line whose bound overflows `i64`.
fn stand_in_line(line: &str) -> Option<String> {
    let indent_len = line.len() - line.trim_start().len();
    let (indent, rest) = line.split_at(indent_len);

    let after_key = match_key(rest)?;
    let (value, suffix) = split_value(after_key);

    let literal = value.trim();
    if !is_integer_literal(literal) || literal.parse::<i64>().is_ok() {
        return None;
    }
    let stand_in = if literal.starts_with('-') { i64::MIN } else { i64::MAX };

    // Rebuilt piecewise rather than by substitution so the line round-trips
    // byte-for-byte apart from the digits themselves.
    let key_part = &rest[..rest.len() - after_key.len()];
    let leading = &value[..value.len() - value.trim_start().len()];
    let trailing = &value[value.len() - (value.trim_start().len() - literal.len())..];
    Some(format!("{indent}{key_part}{leading}{stand_in}{trailing}{suffix}"))
}

/// Matches a leading `key:` or `"key":`, returning the text after the colon.
fn match_key(rest: &str) -> Option<&str> {
    for key in STAND_IN_KEYS {
        for prefix in [key.to_string(), format!("\"{key}\"")] {
            if let Some(tail) = rest.strip_prefix(&prefix)
                && let Some(after) = tail.strip_prefix(':')
            {
                return Some(after);
            }
        }
    }
    None
}

/// Splits the text after a colon into its scalar value and trailing suffix
/// (a JSON trailing comma or a YAML `#` comment).
fn split_value(after_key: &str) -> (&str, &str) {
    let end = after_key
        .find('#')
        .into_iter()
        .chain(after_key.rfind(',').filter(|index| after_key[index + 1..].trim().is_empty()))
        .min()
        .unwrap_or(after_key.len());
    after_key.split_at(end)
}

/// Whether a scalar is a bare decimal integer.
fn is_integer_literal(value: &str) -> bool {
    let digits = value.strip_prefix(['-', '+']).unwrap_or(value);
    !digits.is_empty() && digits.bytes().all(|byte| byte.is_ascii_digit())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stand_ins_replace_only_out_of_range_integer_bounds() {
        let text = "  minimum: -9223372036854776000 # seed\n    \"maximum\": 9223372036854776000,\n  maximum: 100\n  default: 99999999999999999999\n  minimumLength: 99999999999999999999";
        assert_eq!(
            stand_in_bounds(text),
            "  minimum: -9223372036854775808 # seed\n    \"maximum\": 9223372036854775807,\n  maximum: 100\n  default: 99999999999999999999\n  minimumLength: 99999999999999999999"
        );
    }

    #[test]
    fn yaml_capture_records_exact_text_by_pointer_and_skips_names_and_data() {
        let text = "components:\n  schemas:\n    minimum:\n      type: object\n      properties:\n        maximum: {type: integer, maximum: 99999999999999999999, multipleOf: 0.10000000000000000001}\n      example: {minimum: 5}\n    K:\n      const: [1, two]\n";
        let captured = SourceKeywords::capture(text, false).unwrap();
        assert_eq!(captured.numbers, BTreeMap::from([
            ("#/components/schemas/minimum/properties/maximum/maximum".to_owned(), "99999999999999999999".to_owned()),
            ("#/components/schemas/minimum/properties/maximum/multipleOf".to_owned(), "0.10000000000000000001".to_owned()),
        ]));
        assert_eq!(captured.constants["#/components/schemas/K/const"], serde_json::json!([1, "two"]));
    }

    #[test]
    fn json_capture_reads_raw_number_text() {
        let text = r#"{"components":{"schemas":{"A":{"type":"number","minimum":1e400,"maximum":0.10000000000000000001}}}}"#;
        let captured = SourceKeywords::capture(text, true).unwrap();
        assert_eq!(captured.numbers["#/components/schemas/A/minimum"], "1e400");
        assert_eq!(captured.numbers["#/components/schemas/A/maximum"], "0.10000000000000000001");
    }

    #[test]
    fn unrepresentable_bounds_fail_with_their_pointer() {
        for text in ["a:\n  maximum: .inf\n", "a:\n  maximum: 1e10001\n"] {
            match SourceKeywords::capture(text, false) {
                Err(OpenApiError::Validation { path, .. }) => assert_eq!(path, "#/a/maximum"),
                other => panic!("{text}: {other:?}"),
            }
        }
    }

    #[test]
    fn null_value_keywords_fail_instead_of_reading_as_absent() {
        for (text, json) in [("a:\n  maxLength: null\n", false), ("a:\n  minimum: ~\n", false), (r#"{"a":{"enum":null}}"#, true), (r#"{"a":{"maximum":null}}"#, true)] {
            match SourceKeywords::capture(text, json) {
                Err(OpenApiError::Validation { path, message }) => {
                    assert!(path.starts_with("#/a/"), "{text}: {path}");
                    assert!(message.contains("null"), "{text}: {message}");
                }
                other => panic!("{text}: {other:?}"),
            }
        }
        assert!(SourceKeywords::capture("properties:\n  maxLength: {type: string}\n", false).is_ok(), "a property named maxLength is a name");
    }

    #[test]
    fn restored_schemas_carry_exact_numbers_and_constants() {
        let text = "s:\n  type: integer\n  minimum: 0\n  exclusiveMinimum: true\n  maximum: 99999999999999999999\n  const: 7\n";
        let captured = SourceKeywords::capture(text, false).unwrap();
        let parsed = serde_json::json!({"type": "integer", "minimum": 0, "exclusiveMinimum": true, "maximum": 9223372036854775807_i64});
        let restored = captured.source_schema(&parsed, "#/s");
        assert_eq!(restored, serde_json::json!({"type": "integer", "exclusiveMinimum": 0, "maximum": 1e20,
            "x-schematic-exact": {"maximum": "99999999999999999999"}, "const": 7}));
    }
}
