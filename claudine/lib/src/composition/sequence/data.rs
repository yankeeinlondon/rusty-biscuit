//! Data-file loading, offset traversal, and operator application.
//!
//! Every supported sequence source (Markdown frontmatter, YAML, JSON, JSON5,
//! JSONL, NDJSON) is
//! loaded into one common [`serde_json::Value`] model so the offset and
//! operator layers above never branch on file format. The line-delimited
//! formats are the exception the model has to carry: their root is always the
//! list, so an offset against one is a typed error rather than a lookup miss.

use std::path::Path;

use darkmatter::markdown::MappingOrders;
use serde_json::{Map, Value};

use super::super::error::{CompositionError, SequenceLoadCause};
use super::super::json_util::json_type_name;
use super::grammar::SourceOperator;

/// Computes a `template(expr)` name for one item.
///
/// Injected so this module stays free of the expression engine and its
/// resolution context: the operator layer only needs "expression plus item
/// fields yields a value", not the lookup machinery behind it.
pub type TemplateEvaluator<'a> =
    &'a dyn Fn(&str, &Map<String, Value>) -> Result<Value, CompositionError>;

/// The document formats a sequence source may be read from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceFormat {
    /// YAML (`.yaml` / `.yml`).
    Yaml,
    /// JSON (`.json`).
    Json,
    /// JSON5 (`.json5`).
    Json5,
    /// Line-delimited JSON (`.jsonl` / `.ndjson`).
    LineDelimited,
}

impl SourceFormat {
    /// Classify by file extension. Unknown extensions fall back to YAML, which
    /// is a superset of JSON and matches the pre-existing behavior of treating
    /// an extensionless sequence reference as YAML.
    pub fn for_path(path: &Path) -> Self {
        match path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_ascii_lowercase()
            .as_str()
        {
            "json" => Self::Json,
            "json5" => Self::Json5,
            "jsonl" | "ndjson" => Self::LineDelimited,
            _ => Self::Yaml,
        }
    }

    /// `true` when the format's root is always the list, making `->` invalid.
    pub fn is_line_delimited(self) -> bool {
        matches!(self, Self::LineDelimited)
    }
}

/// A loaded data document plus the authored key order of its mappings.
///
/// [`Self::orders`] is populated for YAML, JSON, and JSON5 — every format a
/// `kind: task` / `kind: group` / `kind: group-catalog` document may be
/// authored in — so a lifecycle `set:` keeps its source order whichever one
/// the author chose. A line-delimited file records no order: its root is the
/// list and it cannot author a task or group document, so its index is empty
/// and consumers keep canonical `serde_json::Map` order.
#[derive(Debug, Clone, Default)]
pub struct LoadedDocument {
    /// The document in the common value model.
    pub value: Value,
    /// Authored key order for every mapping in the document.
    pub orders: MappingOrders,
}

/// Load a data file into the common value model.
///
/// A Markdown file is parsed by Darkmatter's own read of it; every other
/// format reads the file once and parses that text through
/// [`load_document_from_text`].
///
/// ## Errors
///
/// Returns [`CompositionError::SequenceExternalLoad`] carrying the typed
/// lower-layer parse cause, including [`SequenceLoadCause::DuplicateKey`] for
/// a JSON, JSON5, JSONL, or NDJSON object that repeats a key (YAML and
/// frontmatter reject one in their own parser).
pub fn load_document(path: &Path) -> Result<LoadedDocument, CompositionError> {
    if is_markdown(path) {
        return load_markdown_document(path).map(|(document, _)| document);
    }
    let context = || biscuit_file::to_portable_string(path);
    let text = std::fs::read_to_string(path).map_err(|error| CompositionError::SequenceExternalLoad {
        context: context(),
        source: match SourceFormat::for_path(path) {
            SourceFormat::Yaml => SequenceLoadCause::Yaml(error.into()),
            SourceFormat::Json | SourceFormat::Json5 => SequenceLoadCause::Json5(error.into()),
            SourceFormat::LineDelimited => SequenceLoadCause::Read(error),
        },
    })?;
    load_document_from_text(path, &text)
}

/// [`load_document`] over `text`, the content of the YAML, JSON, JSON5,
/// JSONL, or NDJSON file at `path`, which is not read: a caller that already
/// holds the file's bytes parses exactly those. `path` selects the format and
/// names the file in errors.
///
/// A Markdown path is not a data format here: its frontmatter parser takes
/// its own read ([`load_markdown_document`]).
///
/// ## Errors
///
/// As [`load_document`], with the same text for the same content.
pub fn load_document_from_text(path: &Path, text: &str) -> Result<LoadedDocument, CompositionError> {
    let context = || biscuit_file::to_portable_string(path);
    match SourceFormat::for_path(path) {
        SourceFormat::Yaml => {
            let yaml = biscuit_file::Yaml::from_str(text).map_err(|e| {
                CompositionError::SequenceExternalLoad {
                    context: context(),
                    source: SequenceLoadCause::Yaml(e),
                }
            })?;
            let orders = MappingOrders::collect(yaml.value());
            let value = yaml
                .as_json()
                .map_err(|e| CompositionError::SequenceExternalLoad {
                    context: context(),
                    source: SequenceLoadCause::Yaml(e),
                })?;
            Ok(LoadedDocument { value, orders })
        }
        SourceFormat::Json | SourceFormat::Json5 => {
            let json5_failure = |e| CompositionError::SequenceExternalLoad {
                context: context(),
                source: SequenceLoadCause::Json5(e),
            };
            let json5 = biscuit_file::Json5::from_str(text).map_err(json5_failure)?;
            // `serde_json::Value` keeps the last of two equal keys, so the
            // raw text is checked before that conversion is trusted.
            if let DuplicateScan(Some(duplicate)) = json5.deserialize_raw::<DuplicateScan>().map_err(json5_failure)? {
                return Err(duplicate.into_error(context(), None));
            }
            let orders = json5.deserialize_raw::<MappingOrders>().map_err(json5_failure)?;
            Ok(LoadedDocument {
                value: json5.as_json_value().clone(),
                orders,
            })
        }
        SourceFormat::LineDelimited => Ok(LoadedDocument {
            value: parse_line_delimited(path, text)?,
            orders: MappingOrders::default(),
        }),
    }
}

/// `true` for a `.md`/`.markdown` path, whose data is its frontmatter.
pub(crate) fn is_markdown(path: &Path) -> bool {
    path.extension().and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("md") || extension.eq_ignore_ascii_case("markdown"))
}

/// A Markdown file's frontmatter in the common value model, with the text
/// Darkmatter's read parsed (`None` only if it retained none).
pub(crate) fn load_markdown_document(path: &Path) -> Result<(LoadedDocument, Option<String>), CompositionError> {
    let markdown = darkmatter::markdown::Markdown::try_from(path).map_err(|source| CompositionError::MarkdownLoad {
        path: path.to_path_buf(),
        source: super::super::error::MarkdownLoadCause::Parse(Box::new(source)),
    })?;
    let document = LoadedDocument {
        value: super::super::prepare::frontmatter_to_value(markdown.frontmatter()),
        orders: markdown.frontmatter().mapping_orders().clone(),
    };
    Ok((document, markdown.loaded_text().map(str::to_owned)))
}

/// Parse JSONL/NDJSON `text` into an array, one entry per non-blank line.
fn parse_line_delimited(path: &Path, text: &str) -> Result<Value, CompositionError> {
    let mut items = Vec::new();
    for (offset, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let line_failure = |e| CompositionError::SequenceExternalLoad {
            context: biscuit_file::to_portable_string(path),
            source: SequenceLoadCause::JsonLine {
                line: offset + 1,
                source: e,
            },
        };
        if let DuplicateScan(Some(duplicate)) = serde_json::from_str::<DuplicateScan>(line).map_err(line_failure)? {
            return Err(duplicate.into_error(biscuit_file::to_portable_string(path), Some(offset + 1)));
        }
        let value: Value = serde_json::from_str(line).map_err(line_failure)?;
        items.push(value);
    }
    Ok(Value::Array(items))
}

/// The first key a JSON-family document repeats within one object, found by
/// deserializing the raw text, since the `serde_json::Value` model has
/// already kept only the last value.
struct DuplicateScan(Option<DuplicateKey>);

/// A repeated key and the path of the object that holds it.
struct DuplicateKey {
    path: String,
    key: String,
}

impl DuplicateKey {
    fn into_error(self, context: String, line: Option<usize>) -> CompositionError {
        CompositionError::SequenceExternalLoad {
            context,
            source: SequenceLoadCause::DuplicateKey {
                key: self.key,
                path: if self.path.is_empty() { "<root>".to_string() } else { self.path },
                line,
            },
        }
    }
}

impl<'de> serde::Deserialize<'de> for DuplicateScan {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        serde::de::DeserializeSeed::deserialize(ScanAt(String::new()), deserializer).map(DuplicateScan)
    }
}

/// Scans the value at one path; the path names nested objects in errors.
struct ScanAt(String);

impl ScanAt {
    fn child(&self, segment: &str) -> Self {
        if self.0.is_empty() { Self(segment.to_string()) } else { Self(format!("{}.{segment}", self.0)) }
    }
}

impl<'de> serde::de::DeserializeSeed<'de> for ScanAt {
    type Value = Option<DuplicateKey>;

    fn deserialize<D: serde::Deserializer<'de>>(self, deserializer: D) -> Result<Self::Value, D::Error> {
        deserializer.deserialize_any(self)
    }
}

impl<'de> serde::de::Visitor<'de> for ScanAt {
    type Value = Option<DuplicateKey>;

    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("a JSON value")
    }

    fn visit_bool<E>(self, _: bool) -> Result<Self::Value, E> { Ok(None) }
    fn visit_i64<E>(self, _: i64) -> Result<Self::Value, E> { Ok(None) }
    fn visit_u64<E>(self, _: u64) -> Result<Self::Value, E> { Ok(None) }
    fn visit_i128<E>(self, _: i128) -> Result<Self::Value, E> { Ok(None) }
    fn visit_u128<E>(self, _: u128) -> Result<Self::Value, E> { Ok(None) }
    fn visit_f64<E>(self, _: f64) -> Result<Self::Value, E> { Ok(None) }
    fn visit_str<E>(self, _: &str) -> Result<Self::Value, E> { Ok(None) }
    fn visit_unit<E>(self) -> Result<Self::Value, E> { Ok(None) }
    fn visit_none<E>(self) -> Result<Self::Value, E> { Ok(None) }

    fn visit_some<D: serde::Deserializer<'de>>(self, deserializer: D) -> Result<Self::Value, D::Error> {
        deserializer.deserialize_any(self)
    }

    fn visit_seq<A: serde::de::SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
        let mut first = None;
        let mut index = 0usize;
        while let Some(found) = seq.next_element_seed(Self(format!("{}[{index}]", self.0)))? {
            first = first.or(found);
            index += 1;
        }
        Ok(first)
    }

    fn visit_map<A: serde::de::MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
        let mut seen = std::collections::HashSet::new();
        let mut first = None;
        while let Some(key) = map.next_key::<String>()? {
            let child = self.child(&key);
            if !seen.insert(key.clone()) && first.is_none() {
                first = Some(DuplicateKey { path: self.0.clone(), key });
            }
            first = first.or(map.next_value_seed(child)?);
        }
        Ok(first)
    }
}

/// Walk a dot-notation offset path into a loaded document.
///
/// ## Errors
///
/// Returns [`CompositionError::SequenceOffsetMissing`] naming the full path
/// attempted and the segment that failed, or
/// [`CompositionError::SequenceOffsetNotAList`] with the type actually found.
pub fn apply_offset<'a>(root: &'a Value, path: &str) -> Result<&'a Value, CompositionError> {
    let mut current = root;
    let mut walked = String::new();

    for segment in path.split('.') {
        if !walked.is_empty() {
            walked.push('.');
        }
        walked.push_str(segment);

        current = current.get(segment).ok_or_else(|| {
            CompositionError::SequenceOffsetMissing {
                path: path.to_string(),
                failed_at: walked.clone(),
                found: json_type_name(current).to_string(),
            }
        })?;
    }
    Ok(current)
}

/// Coerce a resolved document node into the list of items to normalize.
///
/// ## Errors
///
/// Returns [`CompositionError::SequenceOffsetNotAList`] when the node is not an
/// array. `path` is the offset that reached it, or `None` for the document root.
pub fn expect_list(value: &Value, path: Option<&str>) -> Result<Vec<Value>, CompositionError> {
    value
        .as_array()
        .cloned()
        .ok_or_else(|| CompositionError::SequenceOffsetNotAList {
            path: path.unwrap_or("<root>").to_string(),
            found: json_type_name(value).to_string(),
        })
}

/// Apply the reference's operator to every item.
///
/// ## Errors
///
/// Every failure names the zero-based item index that caused it.
pub fn apply_operator(
    items: Vec<Value>,
    operator: &SourceOperator,
    evaluate_template: TemplateEvaluator<'_>,
) -> Result<Vec<Value>, CompositionError> {
    items
        .into_iter()
        .enumerate()
        .map(|(index, item)| apply_operator_to_item(index, item, operator, evaluate_template))
        .collect()
}

fn apply_operator_to_item(
    index: usize,
    item: Value,
    operator: &SourceOperator,
    evaluate_template: TemplateEvaluator<'_>,
) -> Result<Value, CompositionError> {
    let Value::Object(mut map) = item else {
        return Err(CompositionError::SequenceOperatorItemNotObject {
            operator: operator.verb().to_string(),
            index,
            found: json_type_name(&item).to_string(),
        });
    };

    match operator {
        SourceOperator::Map { from, to } => {
            let value = take_field(&map, from, operator, index)?;
            map.remove(from);
            map.insert(to.clone(), value);
        }
        SourceOperator::Name { from } => {
            let value = take_field(&map, from, operator, index)?;
            map.insert("name".to_string(), value);
        }
        SourceOperator::Template { expr } => {
            let rendered = evaluate_template(expr, &map)?;
            let name = match &rendered {
                Value::String(s) => s.clone(),
                Value::Null => String::new(),
                other => other.to_string(),
            };
            if name.trim().is_empty() {
                return Err(CompositionError::SequenceOperatorEmptyName {
                    index,
                    expression: expr.clone(),
                });
            }
            map.insert("name".to_string(), Value::String(name));
        }
    }

    Ok(Value::Object(map))
}

/// Read a required source field, reporting the item index when it is absent.
fn take_field(
    map: &Map<String, Value>,
    field: &str,
    operator: &SourceOperator,
    index: usize,
) -> Result<Value, CompositionError> {
    map.get(field)
        .cloned()
        .ok_or_else(|| CompositionError::SequenceOperatorMissingField {
            operator: operator.verb().to_string(),
            field: field.to_string(),
            index,
        })
}
