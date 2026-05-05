//! Value extraction helpers for parsing harness frontmatter.

use serde_json::Value;

use crate::harness::error::HarnessError;
use crate::harness::model::StructuredShape;

/// Extract a file reference from a value — either a scalar string or an object
/// with a `file` field (or the named field).
pub(super) fn extract_file_ref(value: &Value, field: &str) -> Result<String, HarnessError> {
    if let Some(s) = extract_scalar_string(value) {
        return Ok(s);
    }
    if let Some(s) = extract_string_field(value, field) {
        return Ok(s);
    }
    Err(HarnessError::PathResolutionFailed {
        raw: value.to_string(),
        detail: format!("expected a file path string or an object with a `{field}` field"),
    })
}

/// Extract a string from a scalar JSON value.
pub(super) fn extract_scalar_string(value: &Value) -> Option<String> {
    match value {
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        Value::Bool(b) => Some(b.to_string()),
        _ => None,
    }
}

/// Extract a string field from an object value.
pub(super) fn extract_string_field(value: &Value, field: &str) -> Option<String> {
    value
        .as_object()
        .and_then(|o| o.get(field))
        .and_then(|v| v.as_str())
        .map(String::from)
}

/// Extract a boolean field from an object value.
pub(super) fn extract_bool_field(value: &Value, field: &str) -> Option<bool> {
    value
        .as_object()
        .and_then(|o| o.get(field))
        .and_then(|v| v.as_bool())
}

/// Extract an optional `shape` field and parse it.
pub(super) fn extract_shape(
    value: &Value,
    source_path: &std::path::Path,
) -> Result<Option<StructuredShape>, HarnessError> {
    let Some(shape_str) = extract_string_field(value, "shape") else {
        return Ok(None);
    };
    shape_str
        .parse::<StructuredShape>()
        .map(Some)
        .map_err(|_| HarnessError::InvalidShape {
            source_path: source_path.to_path_buf(),
            raw: shape_str,
        })
}

/// Extract a `usize` from a scalar value.
pub(super) fn extract_usize(value: &Value, name: &str, source_path: &std::path::Path) -> Result<usize, HarnessError> {
    match value {
        Value::Number(n) => {
            n.as_u64()
                .map(|v| v as usize)
                .ok_or_else(|| HarnessError::InvalidFrontmatter {
                    source_path: source_path.to_path_buf(),
                    property: name.to_string(),
                    detail: "requires a positive integer".to_string(),
                })
        }
        Value::Object(obj) => {
            // Expanded form: look for a primary field
            let length_val = obj
                .get("length")
                .or_else(|| obj.get("value"))
                .ok_or_else(|| HarnessError::InvalidFrontmatter {
                    source_path: source_path.to_path_buf(),
                    property: name.to_string(),
                    detail: "requires a number or an object with a `length` field".to_string(),
                })?;
            length_val.as_u64().map(|v| v as usize).ok_or_else(|| {
                HarnessError::InvalidFrontmatter {
                    source_path: source_path.to_path_buf(),
                    property: name.to_string(),
                    detail: "requires a positive integer".to_string(),
                }
            })
        }
        _ => Err(HarnessError::InvalidFrontmatter {
            source_path: source_path.to_path_buf(),
            property: name.to_string(),
            detail: "requires a positive integer".to_string(),
        }),
    }
}
