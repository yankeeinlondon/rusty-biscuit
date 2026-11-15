//! The one JSON loader every document `ci-push` reads goes through, and the
//! field validators its documents share.
//!
//! `serde_json::Value` keeps the last of two equal keys, so a duplicated field
//! would silently decide a constraint or a cell. [`load`] builds the same value
//! but refuses a duplicate key, trailing content, and an empty document; JSON
//! syntax extensions (comments, trailing commas) are already refused by
//! `serde_json`.

use std::fmt;

use serde::de::{self, Deserialize, Deserializer, MapAccess, SeqAccess, Visitor};
use serde_json::{Map, Value};

/// A problem at one place inside a document.
///
/// `location` is a JSON path such as `cells[2].gate`; it is empty when the
/// problem is the document as a whole.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Located {
    pub location: String,
    pub problem: String,
}

impl Located {
    pub(crate) fn new(location: impl Into<String>, problem: impl Into<String>) -> Self {
        Self {
            location: location.into(),
            problem: problem.into(),
        }
    }
}

/// Parses `bytes` as exactly one JSON value, refusing duplicate object keys.
pub(crate) fn load(bytes: &[u8]) -> Result<Value, Located> {
    let mut deserializer = serde_json::Deserializer::from_slice(bytes);
    let value = Strict::deserialize(&mut deserializer)
        .map_err(|error| Located::new("", format!("is not valid JSON: {error}")))?;
    deserializer
        .end()
        .map_err(|error| Located::new("", format!("has content after its JSON value: {error}")))?;
    Ok(value.0)
}

struct Strict(Value);

impl<'de> Deserialize<'de> for Strict {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_any(StrictVisitor)
    }
}

struct StrictVisitor;

impl<'de> Visitor<'de> for StrictVisitor {
    type Value = Strict;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a JSON value")
    }

    fn visit_bool<E>(self, value: bool) -> Result<Strict, E> {
        Ok(Strict(Value::Bool(value)))
    }

    fn visit_i64<E>(self, value: i64) -> Result<Strict, E> {
        Ok(Strict(Value::from(value)))
    }

    fn visit_u64<E>(self, value: u64) -> Result<Strict, E> {
        Ok(Strict(Value::from(value)))
    }

    fn visit_f64<E: de::Error>(self, value: f64) -> Result<Strict, E> {
        serde_json::Number::from_f64(value)
            .map(|number| Strict(Value::Number(number)))
            .ok_or_else(|| E::custom("a number that is not finite"))
    }

    fn visit_str<E>(self, value: &str) -> Result<Strict, E> {
        Ok(Strict(Value::String(value.to_owned())))
    }

    fn visit_string<E>(self, value: String) -> Result<Strict, E> {
        Ok(Strict(Value::String(value)))
    }

    fn visit_unit<E>(self) -> Result<Strict, E> {
        Ok(Strict(Value::Null))
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut sequence: A) -> Result<Strict, A::Error> {
        let mut values = Vec::new();
        while let Some(Strict(value)) = sequence.next_element()? {
            values.push(value);
        }
        Ok(Strict(Value::Array(values)))
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Strict, A::Error> {
        let mut object = Map::new();
        while let Some(key) = map.next_key::<String>()? {
            if object.contains_key(&key) {
                return Err(de::Error::custom(format!("duplicate key {key:?}")));
            }
            let Strict(value) = map.next_value()?;
            object.insert(key, value);
        }
        Ok(Strict(Value::Object(object)))
    }
}

/// The JSON type of `value`, as an error message names it.
pub(crate) fn kind(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "a boolean",
        Value::Number(_) => "a number",
        Value::String(_) => "a string",
        Value::Array(_) => "an array",
        Value::Object(_) => "an object",
    }
}

pub(crate) fn field_path(at: &str, field: &str) -> String {
    if at.is_empty() {
        field.to_owned()
    } else {
        format!("{at}.{field}")
    }
}

pub(crate) fn index_path(at: &str, index: usize) -> String {
    format!("{at}[{index}]")
}

pub(crate) fn object<'a>(value: &'a Value, at: &str) -> Result<&'a Map<String, Value>, Located> {
    value
        .as_object()
        .ok_or_else(|| Located::new(at, format!("must be an object, found {}", kind(value))))
}

/// A field that must be present as a nonempty string.
pub(crate) fn required_text<'a>(
    object: &'a Map<String, Value>,
    at: &str,
    field: &str,
) -> Result<&'a str, Located> {
    let location = field_path(at, field);
    match object.get(field) {
        None => Err(Located::new(location, "is required")),
        Some(Value::String(text)) if text.is_empty() => {
            Err(Located::new(location, "must be a nonempty string, found \"\""))
        }
        Some(Value::String(text)) => Ok(text),
        Some(other) => Err(Located::new(
            location,
            format!("must be a nonempty string, found {}", kind(other)),
        )),
    }
}

/// A field whose absence and `""` both mean "every value" (a wildcard).
///
/// Explicit `null` is not absence: it is refused like any other non-string.
pub(crate) fn optional_wildcard_text<'a>(
    object: &'a Map<String, Value>,
    at: &str,
    field: &str,
) -> Result<&'a str, Located> {
    match object.get(field) {
        None => Ok(""),
        Some(Value::String(text)) => Ok(text),
        Some(other) => Err(Located::new(
            field_path(at, field),
            format!("must be a string when present, found {}", kind(other)),
        )),
    }
}

/// A field that must be present as an array; `[]` is a valid, empty answer.
pub(crate) fn required_array<'a>(
    object: &'a Map<String, Value>,
    at: &str,
    field: &str,
) -> Result<&'a [Value], Located> {
    let location = field_path(at, field);
    match object.get(field) {
        None => Err(Located::new(location, "is required")),
        Some(Value::Array(values)) => Ok(values),
        Some(other) => Err(Located::new(
            location,
            format!("must be an array, found {}", kind(other)),
        )),
    }
}

/// A field that must be a JSON integer greater than zero (not a boolean,
/// float, or numeric string).
pub(crate) fn positive_integer(
    object: &Map<String, Value>,
    at: &str,
    field: &str,
) -> Result<u64, Located> {
    let location = field_path(at, field);
    match object.get(field) {
        None => Err(Located::new(location, "is required")),
        Some(Value::Number(number)) => match number.as_u64() {
            Some(value) if value > 0 => Ok(value),
            _ => Err(Located::new(
                location,
                format!("must be a positive integer, found {number}"),
            )),
        },
        Some(other) => Err(Located::new(
            location,
            format!("must be a positive integer, found {}", kind(other)),
        )),
    }
}

/// A required nonempty string spelled exactly `YYYY-MM-DD` that names a real
/// calendar day.
///
/// Python's `date.fromisoformat` (3.11+) also reads the basic `20261105` and
/// week-date `2026-W45-1` spellings; both are refused here, so neither reader
/// depends on the interpreter's version.
pub(crate) fn calendar_date(
    object: &Map<String, Value>,
    at: &str,
    field: &str,
) -> Result<chrono::NaiveDate, Located> {
    let text = required_text(object, at, field)?;
    parse_calendar_date(text)
        .ok_or_else(|| Located::new(field_path(at, field), format!("must be a YYYY-MM-DD calendar date, found {text:?}")))
}

pub(crate) fn parse_calendar_date(text: &str) -> Option<chrono::NaiveDate> {
    let bytes = text.as_bytes();
    let shaped = bytes.len() == 10
        && bytes[4] == b'-'
        && bytes[7] == b'-'
        && bytes
            .iter()
            .enumerate()
            .all(|(index, byte)| index == 4 || index == 7 || byte.is_ascii_digit());
    if !shaped {
        return None;
    }
    let year = text[0..4].parse().ok()?;
    // chrono's proleptic year 0 has no Python `date`; refusing it keeps the
    // two constraint readers in agreement.
    if year == 0 {
        return None;
    }
    let month = text[5..7].parse().ok()?;
    let day = text[8..10].parse().ok()?;
    chrono::NaiveDate::from_ymd_opt(year, month, day)
}
