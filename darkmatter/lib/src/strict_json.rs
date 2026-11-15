//! The one strict JSON reader: a single JSON value, with a duplicate object
//! key at any depth (which `serde_json` would silently resolve last-wins) and
//! any content after the value both rejected.
//!
//! Shared by the host function facts artifact, the lifecycle host profile,
//! and configuration recovery receipts, so none of them grows its own parser.

use std::fmt;

use serde::de::{self, Deserialize, Deserializer, MapAccess, SeqAccess, Visitor};
use serde_json::{Map, Value};

/// Why [`parse`] refused its text.
#[derive(Debug)]
pub(crate) enum StrictJsonError {
    /// The text does not begin with one valid JSON value, or that value
    /// repeats an object key.
    Invalid(serde_json::Error),
    /// A valid JSON value is followed by more than whitespace.
    Trailing(serde_json::Error),
}

impl fmt::Display for StrictJsonError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Invalid(error) => write!(f, "is not valid JSON: {error}"),
            Self::Trailing(error) => write!(f, "has content after its JSON value: {error}"),
        }
    }
}

/// Parses `text` as exactly one JSON value.
pub(crate) fn parse(text: &str) -> Result<Value, StrictJsonError> {
    let mut deserializer = serde_json::Deserializer::from_str(text);
    let StrictValue(value) = StrictValue::deserialize(&mut deserializer).map_err(StrictJsonError::Invalid)?;
    deserializer.end().map_err(StrictJsonError::Trailing)?;
    Ok(value)
}

struct StrictValue(Value);

impl<'de> Deserialize<'de> for StrictValue {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_any(StrictVisitor).map(StrictValue)
    }
}

struct StrictVisitor;

impl<'de> Visitor<'de> for StrictVisitor {
    type Value = Value;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a JSON value")
    }

    fn visit_bool<E>(self, value: bool) -> Result<Value, E> {
        Ok(Value::Bool(value))
    }

    fn visit_i64<E>(self, value: i64) -> Result<Value, E> {
        Ok(Value::Number(value.into()))
    }

    fn visit_u64<E>(self, value: u64) -> Result<Value, E> {
        Ok(Value::Number(value.into()))
    }

    fn visit_f64<E: de::Error>(self, value: f64) -> Result<Value, E> {
        serde_json::Number::from_f64(value)
            .map(Value::Number)
            .ok_or_else(|| E::custom("a number is not finite"))
    }

    fn visit_str<E>(self, value: &str) -> Result<Value, E> {
        Ok(Value::String(value.to_string()))
    }

    fn visit_string<E>(self, value: String) -> Result<Value, E> {
        Ok(Value::String(value))
    }

    fn visit_unit<E>(self) -> Result<Value, E> {
        Ok(Value::Null)
    }

    fn visit_none<E>(self) -> Result<Value, E> {
        Ok(Value::Null)
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Value, A::Error> {
        let mut items = Vec::new();
        while let Some(StrictValue(item)) = seq.next_element()? {
            items.push(item);
        }
        Ok(Value::Array(items))
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Value, A::Error> {
        let mut object = Map::new();
        while let Some(key) = map.next_key::<String>()? {
            if object.contains_key(&key) {
                return Err(de::Error::custom(format!("duplicate key `{key}`")));
            }
            let StrictValue(value) = map.next_value()?;
            object.insert(key, value);
        }
        Ok(Value::Object(object))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn duplicate_keys_at_any_depth_and_trailing_content_are_refused() {
        assert_eq!(parse(r#"{"a": {"b": 1}}"#).unwrap(), json!({"a": {"b": 1}}));
        for text in [r#"{"a": 1, "a": 2}"#, r#"{"a": {"b": 1, "b": 1}}"#, r#"[{"x": 1, "x": 1}]"#] {
            let error = parse(text).unwrap_err();
            assert!(matches!(error, StrictJsonError::Invalid(_)), "{text}: {error}");
            assert!(error.to_string().contains("duplicate key"), "{text}: {error}");
        }
        assert!(matches!(parse("{} {}"), Err(StrictJsonError::Trailing(_))));
        assert!(matches!(parse("{"), Err(StrictJsonError::Invalid(_))));
        assert!(matches!(parse(""), Err(StrictJsonError::Invalid(_))));
        assert_eq!(parse(" {}\n\n").unwrap(), json!({}));
        assert_eq!(parse("null").unwrap(), Value::Null);
    }
}
