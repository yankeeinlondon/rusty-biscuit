//! The one loader of whole-file YAML documents.
//!
//! A `.yaml`/`.yml` document is read as exactly one YAML document through the
//! shared admitted-YAML loader and numeric model
//! ([`crate::numeric::load_admitted_yaml`]). A leading `---` is
//! a document-start marker and a trailing `...` a document-end marker, never
//! Markdown frontmatter fences. Duplicate keys, a second document, an
//! inadmissible number, a non-string mapping key, and malformed YAML are
//! located load errors, whether or not the file has an alias: the parser
//! locates its own errors, and the [marks](super::marks) pass locates the
//! refusals made after parsing (a second document at its `---`, a key at
//! itself). One leading byte-order mark is skipped (the YAML
//! parser reads a mark followed by CRLF lines as a second document); every
//! span this module hands out is a byte range of the caller's original text.

use std::ops::Range;
use std::path::Path;

use serde_yaml_ng::Value as YamlValue;
use serde_json::Value;
use crate::{PositionMap, SchemaError, ValidationProblem};

use super::marks::{self, Offender};

const BYTE_ORDER_MARK: char = '\u{feff}';

/// A loaded YAML document and what locates its problems.
#[derive(Debug, Clone)]
pub struct LoadedYaml {
    /// The document as authored.
    pub value: YamlValue,
    /// The document as the JSON value schemas judge.
    pub json: Value,
    /// Node positions of the text after the byte-order mark, built when a
    /// coordinate is first requested.
    pub positions: PositionMap,
    /// Bytes skipped before the YAML text: the byte-order mark, or `0`.
    pub offset: usize,
}

impl LoadedYaml {
    /// The byte range of `problem`'s token in the original text.
    pub fn byte_range(&self, problem: &ValidationProblem) -> Option<Range<usize>> {
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
pub fn load_yaml_document(text: &str, path: &Path) -> Result<LoadedYaml, SchemaError> {
    let offset = if text.starts_with(BYTE_ORDER_MARK) { BYTE_ORDER_MARK.len_utf8() } else { 0 };
    let body = &text[offset..];
    let value = crate::numeric::load_admitted_yaml(body).map_err(|error| {
        // Only a second document has no parser location; a duplicate key's
        // is its mapping's start, so the repeated key is found instead.
        let duplicate = error.to_string().contains("duplicate entry").then(|| marks::duplicate_key_marks(body)).flatten();
        let start = duplicate
            .map(|(_, repeated)| repeated)
            .or_else(|| error.location().map(|location| location.index()))
            .or_else(|| marks::second_document_start(body));
        let span = start.map(|start| {
            let start = (start + offset).min(text.len());
            start..start
        });
        let mut message = if crate::numeric::is_yaml_admission_error(&error) {
            error.to_string()
        } else {
            format!("not valid YAML: {error}")
        };
        if let Some((first, _)) = duplicate {
            // The parser's own position is the enclosing mapping's start, not
            // either spelling of the key, so it is replaced by the first one's.
            if let Some(at) = message.rfind(" at line ")
                && message[at + " at line ".len()..].split(" column ").all(|part| part.parse::<usize>().is_ok())
            {
                message.truncate(at);
            }
            let (line, column) = crate::line_col_of_offset(text, first + offset);
            message.push_str(&format!("; the key is first written at line {line} column {column}"));
        }
        load_error(path, message, span)
    })?;
    let positions = PositionMap::deferred(std::sync::Arc::from(body));
    if let Some(pointer) = first_non_string_key(&value, &mut Vec::new()) {
        let span = marks::first_offender_start(body, &value, Offender::NonStringKey)
            .map(|start| start + offset..start + offset);
        let at = if pointer.is_empty() { "the document root".to_string() } else { format!("`/{}`", pointer.join("/")) };
        return Err(load_error(path, format!("a mapping key at {at} is not a string; quote it to keep it as text"), span));
    }
    let json = serde_json::to_value(&value)
        .map_err(|error| load_error(path, format!("not representable as a JSON value: {error}"), None))?;
    Ok(LoadedYaml { value, json, positions, offset })
}

/// Where a key of `text` repeats an earlier key of the same mapping, the
/// failure [`load_yaml_document`] reports as a duplicate entry: the first
/// such repeat in document order and the key it repeats.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DuplicateKey {
    /// Byte offset, in the caller's text, of the earlier spelling.
    pub first: usize,
    /// Byte offset, in the caller's text, of the repeated spelling: the
    /// location [`load_yaml_document`]'s error carries.
    pub repeated: usize,
}

/// Both spellings of the first repeated mapping key of `text`, for a
/// diagnostic that shows the earlier key as related information; `None` when
/// no key repeats. Only worth calling after [`load_yaml_document`] refused a
/// duplicate entry: it re-reads the text.
pub fn duplicate_key(text: &str) -> Option<DuplicateKey> {
    let offset = if text.starts_with(BYTE_ORDER_MARK) { BYTE_ORDER_MARK.len_utf8() } else { 0 };
    let (first, repeated) = marks::duplicate_key_marks(&text[offset..])?;
    Some(DuplicateKey { first: first + offset, repeated: repeated + offset })
}

/// The byte offset in `text` of the first tagged value of `loaded`, which
/// was loaded from `text`.
pub fn first_tag_start(text: &str, loaded: &LoadedYaml) -> Option<usize> {
    marks::first_offender_start(&text[loaded.offset..], &loaded.value, Offender::Tag).map(|start| start + loaded.offset)
}

fn load_error(path: &Path, message: String, span: Option<Range<usize>>) -> SchemaError {
    SchemaError::SchemaDocument { path: path.to_path_buf(), message, source: None, span }
}

/// The path of the mapping holding the first key, in document order, that is
/// not a string; the same key the marks pass locates.
fn first_non_string_key(value: &YamlValue, at: &mut Vec<String>) -> Option<Vec<String>> {
    match value {
        YamlValue::Mapping(map) => {
            for (key, child) in map {
                if !key.is_string() {
                    return Some(at.clone());
                }
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
pub fn locating_problem(segments: &[String]) -> ValidationProblem {
    let pointer = crate::JsonPointer::parse(&pointer_string(segments));
    ValidationProblem {
        hint: None,
        path: pointer.as_pointer_string(),
        message: String::new(),
        kind: crate::ValidationProblemKind::Invalid,
        property: None,
        line: None,
        column: None,
        arm_index: None,
        description: None,
        code: crate::ValidationProblemCode::KindCheck,
        instance_path: pointer,
        schema_path: None,
        offending_property: None,
        offending_properties: Vec::new(),
        file_reference: None,
        caller_file: None,
        default: None,
    }
}

/// `segments` as an RFC 6901 pointer string.
pub fn pointer_string(segments: &[String]) -> String {
    segments.iter().map(|segment| format!("/{}", segment.replace('~', "~0").replace('/', "~1"))).collect()
}
