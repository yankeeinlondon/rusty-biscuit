//! The one loader of whole-file YAML documents.
//!
//! A `.yaml`/`.yml` document is read as exactly one YAML document through the
//! shared admitted-YAML loader and numeric model
//! ([`simplified_schema::numeric::load_admitted_yaml`]). A leading `---` is
//! a document-start marker and a trailing `...` a document-end marker, never
//! Markdown frontmatter fences. Duplicate keys, a second document, an
//! inadmissible number, a non-string mapping key, and malformed YAML are
//! located load errors. One leading byte-order mark is skipped (the YAML
//! parser reads a mark followed by CRLF lines as a second document); every
//! span this module hands out is a byte range of the caller's original text.

use std::ops::Range;
use std::path::Path;

use serde_yaml_ng::Value as YamlValue;
use serde_json::Value;
use simplified_schema::{PositionMap, SchemaError, ValidationProblem};

const BYTE_ORDER_MARK: char = '\u{feff}';

/// A loaded YAML document and what locates its problems.
#[derive(Debug, Clone)]
pub(crate) struct LoadedYaml {
    /// The document as authored.
    pub(crate) value: YamlValue,
    /// The document as the JSON value schemas judge.
    pub(crate) json: Value,
    /// Node positions of the text after the byte-order mark.
    pub(crate) positions: PositionMap,
    /// Bytes skipped before the YAML text: the byte-order mark, or `0`.
    pub(crate) offset: usize,
}

impl LoadedYaml {
    /// The byte range of `problem`'s token in the original text.
    pub(crate) fn byte_range(&self, problem: &ValidationProblem) -> Option<Range<usize>> {
        self.positions.byte_range(problem).map(|range| range.start + self.offset..range.end + self.offset)
    }
}

/// Loads `text`, the content of the YAML document at `path`.
///
/// ## Errors
///
/// [`SchemaError::SchemaDocument`] naming `path`, with the byte span of the
/// failure in `text` when the parser reports one, for every load failure
/// listed in the [module docs](self).
pub(crate) fn load_yaml_document(text: &str, path: &Path) -> Result<LoadedYaml, SchemaError> {
    let offset = if text.starts_with(BYTE_ORDER_MARK) { BYTE_ORDER_MARK.len_utf8() } else { 0 };
    let body = &text[offset..];
    let value = simplified_schema::numeric::load_admitted_yaml(body).map_err(|error| {
        let span = error.location().map(|location| {
            let start = (location.index() + offset).min(text.len());
            start..start
        });
        let message = if simplified_schema::numeric::is_yaml_admission_error(&error) {
            error.to_string()
        } else {
            format!("not valid YAML: {error}")
        };
        load_error(path, message, span)
    })?;
    let positions = simplified_schema::positions::build_position_map(body);
    if let Some(pointer) = first_non_string_key(&value, &mut Vec::new()) {
        let located = locating_problem(&pointer);
        let span = positions.byte_range(&located).map(|range| range.start + offset..range.start + offset);
        let at = if pointer.is_empty() { "the document root".to_string() } else { format!("`/{}`", pointer.join("/")) };
        return Err(load_error(path, format!("a mapping key at {at} is not a string; quote it to keep it as text"), span));
    }
    let json = serde_json::to_value(&value)
        .map_err(|error| load_error(path, format!("not representable as a JSON value: {error}"), None))?;
    Ok(LoadedYaml { value, json, positions, offset })
}

fn load_error(path: &Path, message: String, span: Option<Range<usize>>) -> SchemaError {
    SchemaError::SchemaDocument { path: path.to_path_buf(), message, source: None, span }
}

/// The path of the first mapping holding a key that is not a string.
fn first_non_string_key(value: &YamlValue, at: &mut Vec<String>) -> Option<Vec<String>> {
    match value {
        YamlValue::Mapping(map) => {
            if map.keys().any(|key| !key.is_string()) {
                return Some(at.clone());
            }
            for (key, child) in map {
                at.push(key.as_str().unwrap_or_default().to_string());
                let found = first_non_string_key(child, at);
                at.pop();
                if found.is_some() {
                    return found;
                }
            }
            None
        }
        YamlValue::Sequence(items) => items.iter().enumerate().find_map(|(index, item)| {
            at.push(index.to_string());
            let found = first_non_string_key(item, at);
            at.pop();
            found
        }),
        YamlValue::Tagged(tagged) => first_non_string_key(&tagged.value, at),
        _ => None,
    }
}

/// A problem whose only purpose is to be located: the value at `segments`.
pub(crate) fn locating_problem(segments: &[String]) -> ValidationProblem {
    let pointer = simplified_schema::JsonPointer::parse(&pointer_string(segments));
    ValidationProblem {
        path: pointer.as_pointer_string(),
        message: String::new(),
        kind: simplified_schema::ValidationProblemKind::Invalid,
        property: None,
        line: None,
        column: None,
        arm_index: None,
        description: None,
        code: simplified_schema::ValidationProblemCode::KindCheck,
        instance_path: pointer,
        schema_path: None,
        offending_property: None,
        offending_properties: Vec::new(),
        file_reference: None,
        caller_file: None,
    }
}

/// `segments` as an RFC 6901 pointer string.
pub(crate) fn pointer_string(segments: &[String]) -> String {
    segments.iter().map(|segment| format!("/{}", segment.replace('~', "~0").replace('/', "~1"))).collect()
}
