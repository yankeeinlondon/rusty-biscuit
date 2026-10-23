//! Loading an AsyncAPI document and reading its fields with located
//! diagnostics.
//!
//! Every reader takes the field's value (`None` when the key is absent) and
//! its JSON pointer. A value of the wrong shape, `null` included, is an
//! ERROR at that pointer and reads as `None`; the import then fails before
//! anything is published.

use schematic_define::openapi::import::OpenApiDiagnostic;
use serde_json::{Map, Value, json};

use crate::errors::GeneratorError;
use crate::export::asyncapi::pointer_token;

/// Parses `content` as JSON when it starts with `{`, otherwise as YAML.
/// A duplicate mapping key is an error in both, never last-wins.
pub(super) fn parse_document(content: &str, origin: &str) -> Result<Value, GeneratorError> {
    let parsed: Result<UniqueValue, String> = if content.trim_start().starts_with('{') {
        serde_json::from_str(content).map_err(|e| format!("Failed to parse JSON AsyncAPI '{origin}': {e}"))
    } else {
        biscuit_file::serde_yaml_ng::from_str(content).map_err(|e| format!("Failed to parse YAML AsyncAPI '{origin}': {e}"))
    };
    parsed.map(|value| value.0).map_err(GeneratorError::ParseError)
}

/// A document value whose mappings never repeat a key.
struct UniqueValue(Value);

// The OpenAPI importer rejects duplicate keys the same way
// (`schematic_define::openapi::import`); its visitor is private to that crate.
impl<'de> serde::Deserialize<'de> for UniqueValue {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct Visitor;
        impl<'de> serde::de::Visitor<'de> for Visitor {
            type Value = UniqueValue;
            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("a document without duplicate mapping keys")
            }
            fn visit_bool<E: serde::de::Error>(self, value: bool) -> Result<Self::Value, E> {
                Ok(UniqueValue(json!(value)))
            }
            fn visit_i64<E: serde::de::Error>(self, value: i64) -> Result<Self::Value, E> {
                Ok(UniqueValue(json!(value)))
            }
            fn visit_u64<E: serde::de::Error>(self, value: u64) -> Result<Self::Value, E> {
                Ok(UniqueValue(json!(value)))
            }
            fn visit_f64<E: serde::de::Error>(self, value: f64) -> Result<Self::Value, E> {
                Ok(UniqueValue(json!(value)))
            }
            fn visit_str<E: serde::de::Error>(self, value: &str) -> Result<Self::Value, E> {
                Ok(UniqueValue(json!(value)))
            }
            fn visit_string<E: serde::de::Error>(self, value: String) -> Result<Self::Value, E> {
                Ok(UniqueValue(json!(value)))
            }
            fn visit_unit<E: serde::de::Error>(self) -> Result<Self::Value, E> {
                Ok(UniqueValue(Value::Null))
            }
            fn visit_none<E: serde::de::Error>(self) -> Result<Self::Value, E> {
                self.visit_unit()
            }
            fn visit_seq<A: serde::de::SeqAccess<'de>>(self, mut sequence: A) -> Result<Self::Value, A::Error> {
                let mut values = Vec::new();
                while let Some(value) = sequence.next_element::<UniqueValue>()? {
                    values.push(value.0);
                }
                Ok(UniqueValue(Value::Array(values)))
            }
            fn visit_map<A: serde::de::MapAccess<'de>>(self, mut mapping: A) -> Result<Self::Value, A::Error> {
                let mut values = Map::new();
                while let Some((key, value)) = mapping.next_entry::<String, UniqueValue>()? {
                    if values.insert(key.clone(), value.0).is_some() {
                        return Err(serde::de::Error::custom(format!("Duplicate key {key}")));
                    }
                }
                Ok(UniqueValue(Value::Object(values)))
            }
        }
        deserializer.deserialize_any(Visitor)
    }
}

/// `{at}/{key}` with `key` escaped as a JSON pointer token.
pub(super) fn child(at: &str, key: &str) -> String {
    format!("{at}/{}", pointer_token(key))
}

/// Collects diagnostics while fields are read.
#[derive(Debug, Default)]
pub(super) struct Reader {
    pub(super) diagnostics: Vec<OpenApiDiagnostic>,
}

impl Reader {
    pub(super) fn error(&mut self, at: &str, message: impl Into<String>) {
        self.diagnostics.push(OpenApiDiagnostic::error(at.to_string(), message.into()));
    }

    pub(super) fn warn(&mut self, at: &str, message: impl Into<String>) {
        self.diagnostics.push(OpenApiDiagnostic::warn(at.to_string(), message.into()));
    }

    pub(super) fn info(&mut self, at: &str, message: impl Into<String>) {
        self.diagnostics.push(OpenApiDiagnostic::info(at.to_string(), message.into()));
    }

    /// An optional object.
    pub(super) fn object<'a>(&mut self, value: Option<&'a Value>, at: &str) -> Option<&'a Map<String, Value>> {
        match value {
            None => None,
            Some(Value::Object(map)) => Some(map),
            Some(other) => {
                self.error(at, format!("Expected an object, found {}", kind(other)));
                None
            }
        }
    }

    /// A required object.
    pub(super) fn required_object<'a>(&mut self, value: Option<&'a Value>, at: &str) -> Option<&'a Map<String, Value>> {
        if value.is_none() {
            self.error(at, "Missing required object");
        }
        self.object(value, at)
    }

    /// An optional array.
    pub(super) fn array<'a>(&mut self, value: Option<&'a Value>, at: &str) -> Option<&'a Vec<Value>> {
        match value {
            None => None,
            Some(Value::Array(items)) => Some(items),
            Some(other) => {
                self.error(at, format!("Expected an array, found {}", kind(other)));
                None
            }
        }
    }

    /// An optional string; empty is allowed.
    pub(super) fn string<'a>(&mut self, value: Option<&'a Value>, at: &str) -> Option<&'a str> {
        match value {
            None => None,
            Some(Value::String(text)) => Some(text),
            Some(other) => {
                self.error(at, format!("Expected a string, found {}", kind(other)));
                None
            }
        }
    }

    /// A required, nonempty string.
    pub(super) fn required_string<'a>(&mut self, value: Option<&'a Value>, at: &str) -> Option<&'a str> {
        if value.is_none() {
            self.error(at, "Missing required string");
            return None;
        }
        self.nonempty_string(value, at)
    }

    /// An optional string that must not be empty when present.
    pub(super) fn nonempty_string<'a>(&mut self, value: Option<&'a Value>, at: &str) -> Option<&'a str> {
        let text = self.string(value, at)?;
        if text.is_empty() {
            self.error(at, "Expected a nonempty string");
            return None;
        }
        Some(text)
    }

    /// The local target of a `{"$ref": "#/<prefix>/<name>"}` object, by name.
    /// Anything else (a remote reference, another prefix, a missing target)
    /// is an ERROR: imports never fetch, and every reference resolves locally.
    pub(super) fn local_ref<'a>(
        &mut self,
        reference: &Value,
        prefix: &str,
        targets: Option<&'a Map<String, Value>>,
        at: &str,
    ) -> Option<(String, &'a Value)> {
        let at = child(at, "$ref");
        let text = self.required_string(reference.get("$ref"), &at)?;
        let Some(name) = text.strip_prefix(prefix).map(unescape_token) else {
            self.error(&at, format!("Reference `{text}` must point into `{prefix}`"));
            return None;
        };
        match targets.and_then(|map| map.get(&name)) {
            Some(target) => Some((name, target)),
            None => {
                self.error(&at, format!("Reference `{text}` does not resolve in this document"));
                None
            }
        }
    }
}

/// The JSON type name of `value`, for diagnostics.
pub(super) fn kind(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "a boolean",
        Value::Number(_) => "a number",
        Value::String(_) => "a string",
        Value::Array(_) => "an array",
        Value::Object(_) => "an object",
    }
}

/// Decodes one JSON pointer token.
pub(super) fn unescape_token(token: &str) -> String {
    token.replace("~1", "/").replace("~0", "~")
}
