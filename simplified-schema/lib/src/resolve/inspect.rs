//! A passive look at the file a `$schema` reference selected.
//!
//! An editor that explains a `$schema` value needs to know whether the
//! selected file is an accepted schema target, its `description`, and the
//! authored definition to preview. [`inspect_schema_target`] answers all three
//! from bytes the caller already read, with the loader's own classifiers, so
//! the explanation cannot disagree with validation about the format.

use std::path::Path;

use serde_json::Value;

use super::{classify_json_schema_text, classify_yaml_schema_text, is_json_schema_path};
use crate::errors::SchemaError;
use crate::simplified::{SchemaSourcePath, SchemaSpanKind, StandaloneSchemaDocument};

/// How a schema target is read, decided by its extension as the loader does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchemaTargetFormat {
    /// A YAML schema document (every extension except `json`).
    Yaml,
    /// A JSON Schema document (a `json` extension, in any case).
    Json,
}

/// What [`inspect_schema_target`] found in a schema target.
#[derive(Debug)]
pub struct SchemaTargetInspection {
    /// How the file is read.
    pub format: SchemaTargetFormat,
    /// `Ok` when the file is an accepted `$schema` target; otherwise the
    /// error the referenced-file loader reports for it. A valid format does
    /// not prove the payload's own references and imports resolve.
    pub classification: Result<(), SchemaError>,
    /// The top-level string `description`, trimmed, when it is not blank and
    /// the file is accepted.
    pub description: Option<String>,
    /// The authored definition, one entry per line, when the file is
    /// accepted: the dedented `$schema` value of a YAML document, or the whole
    /// JSON document. Line endings are normalized and a final newline is not
    /// a line.
    pub lines: Option<Vec<String>>,
}

/// Classifies `bytes`, the content of the schema target at `path`, and
/// extracts its description and authored definition. Performs no I/O.
pub fn inspect_schema_target(path: &Path, bytes: &[u8]) -> SchemaTargetInspection {
    let format = if is_json_schema_path(path) {
        SchemaTargetFormat::Json
    } else {
        SchemaTargetFormat::Yaml
    };
    let rejected = |classification| SchemaTargetInspection {
        format,
        classification: Err(classification),
        description: None,
        lines: None,
    };
    let text = match std::str::from_utf8(bytes) {
        Ok(text) => text,
        Err(error) => {
            return rejected(SchemaError::SchemaDocument {
                path: path.to_path_buf(),
                message: format!("not UTF-8 text: {error}"),
                source: None,
                span: None,
            });
        }
    };
    let (description, lines) = match format {
        SchemaTargetFormat::Json => match classify_json_schema_text(path, text) {
            Ok(value) => (
                value.get("description").and_then(Value::as_str).map(str::to_string),
                logical_lines(text),
            ),
            Err(error) => return rejected(error),
        },
        SchemaTargetFormat::Yaml => match classify_yaml_schema_text(path, text) {
            Ok(document) => (document.description.clone(), yaml_definition_lines(&document, text)),
            Err(error) => return rejected(error),
        },
    };
    SchemaTargetInspection {
        format,
        classification: Ok(()),
        description: description
            .map(|description| description.trim().to_string())
            .filter(|description| !description.is_empty()),
        lines: Some(lines),
    }
}

/// The `$schema` value of an accepted YAML document, with the indentation its
/// lines share removed.
///
/// The value's first line starts at the span, and its indentation is the
/// span's column when only whitespace precedes it (a block mapping or
/// sequence under the key); a value written after the key on the same line
/// (a flow mapping or a scalar) starts flush and its continuation lines keep
/// their indentation, since the key's column says nothing about them.
fn yaml_definition_lines(document: &StandaloneSchemaDocument, text: &str) -> Vec<String> {
    let Some(span) = document
        .source_map
        .spans(&SchemaSourcePath::root(), SchemaSpanKind::Declaration)
        .first()
        .cloned()
    else {
        return Vec::new();
    };
    let line_start = text[..span.start].rfind('\n').map_or(0, |newline| newline + 1);
    let lead = &text[line_start..span.start];
    let first_indent = if lead.chars().all(|c| c == ' ' || c == '\t') { lead.len() } else { 0 };
    let mut lines = logical_lines(&text[span]);
    let common = lines
        .iter()
        .skip(1)
        .filter(|line| !line.trim().is_empty())
        .map(|line| indentation(line))
        .chain(std::iter::once(first_indent))
        .min()
        .unwrap_or(0);
    for (index, line) in lines.iter_mut().enumerate() {
        if index == 0 {
            *line = format!("{}{line}", " ".repeat(first_indent - common));
        } else {
            let strip = indentation(line).min(common);
            line.drain(..strip);
        }
    }
    lines
}

/// The byte length of a line's leading spaces and tabs.
fn indentation(line: &str) -> usize {
    line.len() - line.trim_start_matches([' ', '\t']).len()
}

/// The lines of `text` with `\r\n` normalized; a final newline is not a line.
fn logical_lines(text: &str) -> Vec<String> {
    let text = text.replace("\r\n", "\n");
    let text = text.strip_suffix('\n').unwrap_or(&text);
    if text.is_empty() {
        return Vec::new();
    }
    text.split('\n').map(str::to_string).collect()
}
