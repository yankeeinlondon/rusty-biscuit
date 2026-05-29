//! Parsing and serialization of the on-disk `hash` frontmatter property.
//!
//! A stored hash lives in frontmatter in one of two shapes:
//!
//! - **Shorthand** — a bare string, which is always a `simple`
//!   `{fm}-{body}` value with no extra ignored properties.
//! - **Longhand** — an object `{ kind, value, ignored? }`, used for every
//!   non-`simple` kind and for `simple` whenever extra properties are ignored.
//!   For `detailed`, `value` is the nested [`DetailedValue`] object.
//!
//! The promotion invariant is symmetric: [`StoredHash::to_frontmatter_value`]
//! emits a bare string **iff** the kind is `simple` and there are zero extra
//! ignored properties; otherwise it emits an object. The `ignored` list is
//! sorted and omitted entirely when empty, and records only extra ignored
//! properties — never the active hash property or `last_updated`.

use serde_json::Value;

use super::compute::{ComputedHash, DetailedValue};
use super::kind::MdHashKind;
use crate::markdown::{MarkdownError, MarkdownResult};

/// The value half of a stored hash: a flat string for `fm`, `body`, `simple`,
/// and `structured`, or the nested object for `detailed`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StoredHashValue {
    /// `fm` / `body` / `simple` / `structured` — a single string.
    Flat(String),
    /// `detailed` — the nested object shape.
    Detailed(DetailedValue),
}

/// A parsed view of a document's stored `hash` property.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredHash {
    /// The kind the stored value was recorded under.
    pub kind: MdHashKind,
    /// The stored value, flat or detailed.
    pub value: StoredHashValue,
    /// Extra ignored properties, sorted. Empty means the property had no
    /// `ignored` list. Never contains the active hash property or
    /// `last_updated`.
    pub ignored: Vec<String>,
}

impl StoredHash {
    /// Parses a stored `hash` property value.
    ///
    /// A JSON string is the shorthand `simple` form. A JSON object is the
    /// longhand form carrying `kind`, `value`, and an optional `ignored` list;
    /// for `detailed`, `value` is parsed as the nested object.
    ///
    /// ## Errors
    ///
    /// Returns [`MarkdownError::MalformedStoredHash`] when the value is neither
    /// a string nor an object, when a required field is missing or has the
    /// wrong type, when `kind` is not a recognized token, or when a `detailed`
    /// value does not match the expected nested shape. The `property` argument
    /// is echoed back in the error so CLI users know which key to fix.
    pub fn parse(value: &Value, property: &str) -> MarkdownResult<Self> {
        match value {
            Value::String(flat) => {
                validate_flat(flat, MdHashKind::Simple, property)?;
                Ok(Self {
                    kind: MdHashKind::Simple,
                    value: StoredHashValue::Flat(flat.clone()),
                    ignored: Vec::new(),
                })
            }
            Value::Object(map) => Self::parse_object(map, property),
            other => Err(malformed(
                property,
                format!(
                    "expected a string or an object, found {}",
                    json_type_name(other)
                ),
            )),
        }
    }

    fn parse_object(map: &serde_json::Map<String, Value>, property: &str) -> MarkdownResult<Self> {
        let kind = match map.get("kind") {
            Some(Value::String(token)) => token
                .parse::<MdHashKind>()
                .map_err(|err| malformed(property, err.to_string()))?,
            Some(other) => {
                return Err(malformed(
                    property,
                    format!("'kind' must be a string, found {}", json_type_name(other)),
                ));
            }
            None => return Err(malformed(property, "missing required field 'kind'")),
        };

        let raw_value = map
            .get("value")
            .ok_or_else(|| malformed(property, "missing required field 'value'"))?;

        let value = match kind {
            MdHashKind::Detailed => {
                let detailed: DetailedValue =
                    serde_json::from_value(raw_value.clone()).map_err(|err| {
                        malformed(property, format!("invalid 'detailed' value: {err}"))
                    })?;
                validate_detailed(&detailed, property)?;
                StoredHashValue::Detailed(detailed)
            }
            _ => match raw_value {
                Value::String(flat) => {
                    validate_flat(flat, kind, property)?;
                    StoredHashValue::Flat(flat.clone())
                }
                other => {
                    return Err(malformed(
                        property,
                        format!(
                            "'{kind}' value must be a string, found {}",
                            json_type_name(other)
                        ),
                    ));
                }
            },
        };

        let ignored = match map.get("ignored") {
            None | Some(Value::Null) => Vec::new(),
            Some(Value::Array(items)) => {
                let mut names = Vec::with_capacity(items.len());
                for item in items {
                    match item {
                        Value::String(name) => names.push(name.clone()),
                        other => {
                            return Err(malformed(
                                property,
                                format!(
                                    "'ignored' entries must be strings, found {}",
                                    json_type_name(other)
                                ),
                            ));
                        }
                    }
                }
                names.sort();
                names
            }
            Some(other) => {
                return Err(malformed(
                    property,
                    format!(
                        "'ignored' must be a list of strings, found {}",
                        json_type_name(other)
                    ),
                ));
            }
        };

        Ok(Self {
            kind,
            value,
            ignored,
        })
    }

    /// Serializes back to a frontmatter value, upholding the promotion
    /// invariant: a bare string iff `kind == Simple && ignored.is_empty()`,
    /// otherwise an object with sorted `ignored` (omitted when empty).
    pub fn to_frontmatter_value(&self) -> Value {
        if self.kind == MdHashKind::Simple
            && self.ignored.is_empty()
            && let StoredHashValue::Flat(flat) = &self.value
        {
            return Value::String(flat.clone());
        }

        let mut object = serde_json::Map::new();
        object.insert("kind".to_string(), Value::String(self.kind.to_string()));
        object.insert(
            "value".to_string(),
            match &self.value {
                StoredHashValue::Flat(flat) => Value::String(flat.clone()),
                StoredHashValue::Detailed(detailed) => serde_json::to_value(detailed)
                    .expect("DetailedValue always serializes to a JSON object"),
            },
        );
        if !self.ignored.is_empty() {
            let mut sorted = self.ignored.clone();
            sorted.sort();
            object.insert(
                "ignored".to_string(),
                Value::Array(sorted.into_iter().map(Value::String).collect()),
            );
        }
        Value::Object(object)
    }
}

/// Stored-hash representation of a freshly computed hash.
impl ComputedHash {
    /// Renders this computed hash into a [`StoredHashValue`] — the flat string
    /// form for non-`detailed` kinds, or the nested object for `detailed`.
    pub fn to_stored_value(&self) -> StoredHashValue {
        match self.flat_string() {
            Some(flat) => StoredHashValue::Flat(flat),
            None => match self {
                ComputedHash::Detailed(detailed) => StoredHashValue::Detailed(detailed.clone()),
                // `flat_string` returns `None` only for `Detailed`.
                _ => unreachable!("only detailed hashes lack a flat string form"),
            },
        }
    }
}

/// Whether `s` is a canonical hash component: exactly 16 lowercase hex digits.
fn is_hash_component(s: &str) -> bool {
    s.len() == 16
        && s.bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

/// The number of `-`-joined components a flat (non-`detailed`) kind carries.
fn flat_component_count(kind: MdHashKind) -> usize {
    match kind {
        MdHashKind::Fm | MdHashKind::Body => 1,
        MdHashKind::Simple => 2,
        MdHashKind::Structured => 4,
        // `detailed` never stores a flat value; it is validated separately.
        MdHashKind::Detailed => unreachable!("detailed has no flat component count"),
    }
}

/// Validates a flat stored value: the right number of `-`-joined components for
/// `kind`, each a canonical 16-digit lowercase hex hash.
fn validate_flat(flat: &str, kind: MdHashKind, property: &str) -> MarkdownResult<()> {
    let expected = flat_component_count(kind);
    let parts: Vec<&str> = flat.split('-').collect();
    if parts.len() != expected {
        return Err(malformed(
            property,
            format!(
                "expected {expected} '-'-joined hash components for kind '{kind}', found {}",
                parts.len()
            ),
        ));
    }
    for part in parts {
        if !is_hash_component(part) {
            return Err(malformed(
                property,
                format!("'{part}' is not a 16-digit lowercase hex hash"),
            ));
        }
    }
    Ok(())
}

/// Validates every hash field of a `detailed` stored value.
fn validate_detailed(detailed: &DetailedValue, property: &str) -> MarkdownResult<()> {
    let check = |label: &str, value: &str| -> MarkdownResult<()> {
        if is_hash_component(value) {
            Ok(())
        } else {
            Err(malformed(
                property,
                format!("{label} '{value}' is not a 16-digit lowercase hex hash"),
            ))
        }
    };

    check("frontmatter.fm", &detailed.frontmatter.fm)?;
    check("frontmatter.keys", &detailed.frontmatter.keys)?;
    if let Some(preamble) = &detailed.preamble {
        check("preamble", preamble)?;
    }
    for (index, section) in detailed.sections.iter().enumerate() {
        check(&format!("sections[{index}].content_hash"), &section.content_hash)?;
    }
    Ok(())
}

/// Constructs a [`MarkdownError::MalformedStoredHash`] for `property`.
pub(super) fn malformed(property: &str, reason: impl Into<String>) -> MarkdownError {
    MarkdownError::MalformedStoredHash {
        property: property.to_string(),
        reason: reason.into(),
    }
}

/// Human-readable JSON type name for error messages.
fn json_type_name(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "a boolean",
        Value::Number(_) => "a number",
        Value::String(_) => "a string",
        Value::Array(_) => "a list",
        Value::Object(_) => "an object",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markdown::hash::{FmHashPair, SectionTuple};

    fn detailed_fixture() -> DetailedValue {
        DetailedValue {
            frontmatter: FmHashPair {
                fm: "0000000000000001".to_string(),
                keys: "0000000000000002".to_string(),
            },
            preamble: Some("0000000000000003".to_string()),
            sections: vec![
                SectionTuple {
                    level: 1,
                    heading: "Intro".to_string(),
                    content_hash: "0000000000000004".to_string(),
                },
                SectionTuple {
                    level: 2,
                    heading: "Setup".to_string(),
                    content_hash: "0000000000000005".to_string(),
                },
            ],
        }
    }

    /// `parse → to_frontmatter_value → parse` must reproduce the value.
    fn assert_round_trip(stored: StoredHash) {
        let serialized = stored.to_frontmatter_value();
        let reparsed = StoredHash::parse(&serialized, "hash").unwrap();
        assert_eq!(reparsed, stored);
    }

    #[test]
    fn shorthand_simple_parses_as_simple_with_no_ignored() {
        let value = Value::String("aaaa000000000000-bbbb000000000000".to_string());
        let stored = StoredHash::parse(&value, "hash").unwrap();
        assert_eq!(stored.kind, MdHashKind::Simple);
        assert_eq!(
            stored.value,
            StoredHashValue::Flat("aaaa000000000000-bbbb000000000000".to_string())
        );
        assert!(stored.ignored.is_empty());
    }

    #[test]
    fn shorthand_simple_round_trips_to_bare_string() {
        let stored = StoredHash {
            kind: MdHashKind::Simple,
            value: StoredHashValue::Flat("aaaa000000000000-bbbb000000000000".to_string()),
            ignored: Vec::new(),
        };
        // The promotion invariant: a string, never an object.
        let serialized = stored.to_frontmatter_value();
        assert!(serialized.is_string(), "expected bare string, got {serialized}");
        assert_round_trip(stored);
    }

    #[test]
    fn longhand_simple_with_ignored_stays_an_object() {
        let stored = StoredHash {
            kind: MdHashKind::Simple,
            value: StoredHashValue::Flat("aaaa000000000000-bbbb000000000000".to_string()),
            ignored: vec!["draft".to_string(), "reviewed".to_string()],
        };
        let serialized = stored.to_frontmatter_value();
        assert!(serialized.is_object(), "expected object, got {serialized}");
        assert_eq!(serialized["kind"], Value::String("simple".to_string()));
        assert_eq!(
            serialized["ignored"],
            serde_json::json!(["draft", "reviewed"])
        );
        assert_round_trip(stored);
    }

    #[test]
    fn ignored_is_sorted_on_serialization() {
        let stored = StoredHash {
            kind: MdHashKind::Fm,
            value: StoredHashValue::Flat("aaaa000000000000".to_string()),
            ignored: vec!["reviewed".to_string(), "draft".to_string(), "author".to_string()],
        };
        let serialized = stored.to_frontmatter_value();
        assert_eq!(
            serialized["ignored"],
            serde_json::json!(["author", "draft", "reviewed"])
        );
    }

    #[test]
    fn empty_ignored_is_omitted_in_longhand() {
        let stored = StoredHash {
            kind: MdHashKind::Structured,
            value: StoredHashValue::Flat(
                "a000000000000000-b000000000000000-c000000000000000-d000000000000000".to_string(),
            ),
            ignored: Vec::new(),
        };
        let serialized = stored.to_frontmatter_value();
        assert!(
            serialized.get("ignored").is_none(),
            "ignored should be omitted when empty: {serialized}"
        );
    }

    #[test]
    fn fm_and_body_round_trip() {
        assert_round_trip(StoredHash {
            kind: MdHashKind::Fm,
            value: StoredHashValue::Flat("aaaa000000000000".to_string()),
            ignored: Vec::new(),
        });
        assert_round_trip(StoredHash {
            kind: MdHashKind::Body,
            value: StoredHashValue::Flat("bbbb000000000000".to_string()),
            ignored: Vec::new(),
        });
    }

    #[test]
    fn structured_round_trips() {
        assert_round_trip(StoredHash {
            kind: MdHashKind::Structured,
            value: StoredHashValue::Flat(
                "a000000000000000-b000000000000000-c000000000000000-d000000000000000".to_string(),
            ),
            ignored: vec!["draft".to_string()],
        });
    }

    #[test]
    fn detailed_round_trips_as_nested_object() {
        let stored = StoredHash {
            kind: MdHashKind::Detailed,
            value: StoredHashValue::Detailed(detailed_fixture()),
            ignored: Vec::new(),
        };
        let serialized = stored.to_frontmatter_value();
        assert!(serialized.is_object());
        assert_eq!(serialized["kind"], Value::String("detailed".to_string()));
        assert!(serialized["value"]["frontmatter"].is_object());
        assert_eq!(
            serialized["value"]["sections"][0],
            serde_json::json!([1, "Intro", "0000000000000004"])
        );
        assert_round_trip(stored);
    }

    #[test]
    fn detailed_preamble_null_round_trips() {
        let mut detailed = detailed_fixture();
        detailed.preamble = None;
        assert_round_trip(StoredHash {
            kind: MdHashKind::Detailed,
            value: StoredHashValue::Detailed(detailed),
            ignored: vec!["draft".to_string()],
        });
    }

    #[test]
    fn computed_hash_to_stored_value_flat_and_detailed() {
        let simple = ComputedHash::Simple {
            fm: "aaaa000000000000".to_string(),
            body: "bbbb000000000000".to_string(),
        };
        assert_eq!(
            simple.to_stored_value(),
            StoredHashValue::Flat("aaaa000000000000-bbbb000000000000".to_string())
        );

        let detailed = ComputedHash::Detailed(detailed_fixture());
        assert_eq!(
            detailed.to_stored_value(),
            StoredHashValue::Detailed(detailed_fixture())
        );
    }

    #[test]
    fn rejects_non_string_non_object() {
        let err = StoredHash::parse(&serde_json::json!(42), "hash").unwrap_err();
        assert!(matches!(err, MarkdownError::MalformedStoredHash { .. }));
        assert!(err.to_string().contains("hash"));
        assert!(err.to_string().contains("number"));
    }

    #[test]
    fn rejects_missing_kind() {
        let value = serde_json::json!({ "value": "aaaa000000000000" });
        let err = StoredHash::parse(&value, "fingerprint").unwrap_err();
        assert!(err.to_string().contains("missing required field 'kind'"));
        assert!(err.to_string().contains("fingerprint"));
    }

    #[test]
    fn rejects_unknown_kind() {
        let value = serde_json::json!({ "kind": "bogus", "value": "x" });
        let err = StoredHash::parse(&value, "hash").unwrap_err();
        assert!(err.to_string().contains("unknown hash kind 'bogus'"));
    }

    #[test]
    fn rejects_missing_value() {
        let value = serde_json::json!({ "kind": "structured" });
        let err = StoredHash::parse(&value, "hash").unwrap_err();
        assert!(err.to_string().contains("missing required field 'value'"));
    }

    #[test]
    fn rejects_flat_kind_with_object_value() {
        let value = serde_json::json!({ "kind": "simple", "value": { "nested": true } });
        let err = StoredHash::parse(&value, "hash").unwrap_err();
        assert!(err.to_string().contains("must be a string"));
    }

    #[test]
    fn rejects_detailed_with_string_value() {
        let value = serde_json::json!({ "kind": "detailed", "value": "not-an-object" });
        let err = StoredHash::parse(&value, "hash").unwrap_err();
        assert!(err.to_string().contains("invalid 'detailed' value"));
    }

    #[test]
    fn rejects_non_string_ignored_entry() {
        let value = serde_json::json!({
            "kind": "fm",
            "value": "aaaa000000000000",
            "ignored": ["draft", 7],
        });
        let err = StoredHash::parse(&value, "hash").unwrap_err();
        assert!(err.to_string().contains("'ignored' entries must be strings"));
    }

    #[test]
    fn ignored_is_sorted_on_parse() {
        let value = serde_json::json!({
            "kind": "fm",
            "value": "aaaa000000000000",
            "ignored": ["reviewed", "draft"],
        });
        let stored = StoredHash::parse(&value, "hash").unwrap();
        assert_eq!(stored.ignored, vec!["draft".to_string(), "reviewed".to_string()]);
    }

    #[test]
    fn rejects_shorthand_with_non_hex_components() {
        // Two 16-char components, but outside the lowercase hex alphabet.
        let value = Value::String("zzzzzzzzzzzzzzzz-yyyyyyyyyyyyyyyy".to_string());
        let err = StoredHash::parse(&value, "hash").unwrap_err();
        assert!(matches!(err, MarkdownError::MalformedStoredHash { .. }));
        assert!(err.to_string().contains("not a 16-digit lowercase hex hash"));
    }

    #[test]
    fn rejects_shorthand_with_extra_components() {
        // The review's literal `not-a-real-hash-but-two-parts` splits into many
        // components, so the component-count guard fires before the hex check.
        let value = Value::String("not-a-real-hash-but-two-parts".to_string());
        let err = StoredHash::parse(&value, "hash").unwrap_err();
        assert!(err.to_string().contains("expected 2"));
    }

    #[test]
    fn rejects_shorthand_with_uppercase_hex() {
        let value = Value::String("AAAA000000000000-bbbb000000000000".to_string());
        let err = StoredHash::parse(&value, "hash").unwrap_err();
        assert!(err.to_string().contains("not a 16-digit lowercase hex hash"));
    }

    #[test]
    fn rejects_shorthand_with_wrong_component_length() {
        // Correct hex alphabet, but each component is the wrong length.
        let value = Value::String("aaaa-bbbb".to_string());
        let err = StoredHash::parse(&value, "hash").unwrap_err();
        assert!(err.to_string().contains("not a 16-digit lowercase hex hash"));
    }

    #[test]
    fn rejects_structured_with_wrong_component_count() {
        let value = serde_json::json!({
            "kind": "structured",
            "value": "aaaa000000000000-bbbb000000000000",
        });
        let err = StoredHash::parse(&value, "hash").unwrap_err();
        assert!(err.to_string().contains("expected 4"));
    }

    #[test]
    fn rejects_structured_with_non_hex_component() {
        let value = serde_json::json!({
            "kind": "structured",
            "value": "aaaa000000000000-bbbb000000000000-cccc000000000000-zzzzzzzzzzzzzzzz",
        });
        let err = StoredHash::parse(&value, "hash").unwrap_err();
        assert!(err.to_string().contains("not a 16-digit lowercase hex hash"));
    }

    #[test]
    fn rejects_detailed_with_non_hex_section_hash() {
        let mut detailed = detailed_fixture();
        detailed.sections[1].content_hash = "zzzz000000000000".to_string();
        let value = StoredHash {
            kind: MdHashKind::Detailed,
            value: StoredHashValue::Detailed(detailed),
            ignored: Vec::new(),
        }
        .to_frontmatter_value();
        let err = StoredHash::parse(&value, "hash").unwrap_err();
        assert!(err.to_string().contains("sections[1].content_hash"));
    }

    #[test]
    fn null_ignored_parses_as_empty() {
        let value = serde_json::json!({
            "kind": "fm",
            "value": "aaaa000000000000",
            "ignored": null,
        });
        let stored = StoredHash::parse(&value, "hash").unwrap();
        assert!(stored.ignored.is_empty());
    }
}
