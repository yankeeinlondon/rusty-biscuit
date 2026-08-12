//! Data-file loading, offset traversal, and operator application.
//!
//! Every supported sequence data format (YAML, JSON, JSON5, JSONL, NDJSON) is
//! loaded into one common [`serde_json::Value`] model so the offset and
//! operator layers above never branch on file format. The line-delimited
//! formats are the exception the model has to carry: their root is always the
//! list, so an offset against one is a typed error rather than a lookup miss.

use std::path::Path;

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

/// Load a data file into the common value model.
///
/// ## Errors
///
/// Returns [`CompositionError::SequenceExternalLoad`] carrying the typed
/// lower-layer parse cause.
pub fn load_document(path: &Path) -> Result<Value, CompositionError> {
    let format = SourceFormat::for_path(path);
    let context = || biscuit_file::to_portable_string(path);

    match format {
        SourceFormat::Yaml => {
            let yaml = biscuit_file::Yaml::new(path).map_err(|e| {
                CompositionError::SequenceExternalLoad {
                    context: context(),
                    source: SequenceLoadCause::Yaml(e),
                }
            })?;
            yaml.as_json()
                .map_err(|e| CompositionError::SequenceExternalLoad {
                    context: context(),
                    source: SequenceLoadCause::Yaml(e),
                })
        }
        SourceFormat::Json | SourceFormat::Json5 => {
            let json5 = biscuit_file::Json5::new(path).map_err(|e| {
                CompositionError::SequenceExternalLoad {
                    context: context(),
                    source: SequenceLoadCause::Json5(e),
                }
            })?;
            Ok(json5.as_json_value().clone())
        }
        SourceFormat::LineDelimited => load_line_delimited(path),
    }
}

/// Parse a JSONL/NDJSON file into an array, one entry per non-blank line.
fn load_line_delimited(path: &Path) -> Result<Value, CompositionError> {
    let text =
        std::fs::read_to_string(path).map_err(|e| CompositionError::SequenceExternalLoad {
            context: biscuit_file::to_portable_string(path),
            source: SequenceLoadCause::Read(e),
        })?;

    let mut items = Vec::new();
    for (offset, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let value: Value = serde_json::from_str(line).map_err(|e| {
            CompositionError::SequenceExternalLoad {
                context: biscuit_file::to_portable_string(path),
                source: SequenceLoadCause::JsonLine {
                    line: offset + 1,
                    source: e,
                },
            }
        })?;
        items.push(value);
    }
    Ok(Value::Array(items))
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
