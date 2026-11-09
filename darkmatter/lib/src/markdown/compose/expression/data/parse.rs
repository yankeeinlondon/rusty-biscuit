//! Decoding and parsing one `load_data` document in its format.
//!
//! The text is the document with one leading byte-order mark removed. Every
//! parser's error position is turned into a byte offset in that text and then
//! into a 1-based line and character column by
//! [`biscuit_file::source_position`], so no parser's own column convention
//! leaks into a [`DataSourceLocation`]. Numbers enter only through
//! `simplified_schema::numeric`; a number the format writes but the numeric
//! model cannot admit makes the document malformed, because admission is
//! part of each format's number rules.
//!
//! Loaded values stay data: nothing here composes, executes, or selects a
//! schema from what the document contains.

use std::path::Path;
use std::sync::Arc;

use biscuit_file::{DataFormat, DelimitedError, Json5, JsonLinesError, Toml, YamlValue, source_position};
use serde_json::Value;
use simplified_schema::SchemaError;
use simplified_schema::coercion::PathSegment;
use simplified_schema::numeric::{self, JsonNumberError};

use super::error::{DataError, DataSourceLocation};
use super::locate::SourceLocator;
use super::runtime::{DataSourceIdentity, SourceContent};
use crate::markdown::schemas::yaml_documents::{first_tag_start, load_yaml_document, pointer_string};

/// A parse failure before it is attributed to a source.
struct Failure {
    message: String,
    location: DataSourceLocation,
}

impl Failure {
    fn new(message: impl Into<String>, location: DataSourceLocation) -> Self {
        Self { message: message.into(), location }
    }
}

/// Decodes a local file's bytes and removes one leading byte-order mark.
///
/// ## Errors
///
/// [`DataError::Malformed`] for bytes that are not UTF-8, located by the
/// zero-based offset of the first invalid byte in `bytes` (a byte-order mark
/// counted), with no line or column.
pub(crate) fn decode_local(
    bytes: Vec<u8>,
    identity: &DataSourceIdentity,
    format: DataFormat,
) -> Result<Arc<str>, DataError> {
    let text = String::from_utf8(bytes).map_err(|error| {
        let offset = error.utf8_error().valid_up_to();
        DataError::Malformed {
            identity: identity.clone(),
            format,
            message: format!("the file is not valid UTF-8 (byte {offset})"),
            location: DataSourceLocation { byte_offset: Some(offset), ..DataSourceLocation::default() },
        }
    })?;
    Ok(Arc::from(strip_bom(&text)))
}

/// `text` without one leading byte-order mark.
pub(crate) fn strip_bom(text: &str) -> &str {
    text.strip_prefix('\u{feff}').unwrap_or(text)
}

/// Parses BOM-stripped `text` as `format`.
///
/// ## Errors
///
/// [`DataError::Malformed`] for text the format's rules reject: invalid
/// syntax, trailing text, an inadmissible number, a YAML stream of several
/// documents, a YAML duplicate or non-string key or tag, a TOML duplicate
/// key, or a CSV/TSV quote violation or ragged row. Empty JSON and JSON5 are
/// malformed; empty YAML is `null`, empty JSON Lines/CSV/TSV have no rows,
/// and empty TOML is an empty table.
pub(crate) fn parse_text(
    text: &str,
    format: DataFormat,
    identity: &DataSourceIdentity,
) -> Result<SourceContent, DataError> {
    let parsed = match format {
        DataFormat::Yaml => parse_yaml(text, identity).map(SourceContent::Tree),
        DataFormat::Json => parse_json(text).map(SourceContent::Tree),
        DataFormat::Jsonl => parse_json_lines(text).map(SourceContent::Tree),
        DataFormat::Toml => parse_toml(text).map(SourceContent::Tree),
        DataFormat::Json5 => parse_json5(text).map(SourceContent::Tree),
        DataFormat::Csv => parse_delimited(text, b',').map(SourceContent::Table),
        DataFormat::Tsv => parse_delimited(text, b'\t').map(SourceContent::Table),
        DataFormat::Markdown | DataFormat::Pdf | DataFormat::Text => {
            Err(Failure::new("not a data format", DataSourceLocation::default()))
        }
    };
    parsed.map_err(|failure| DataError::Malformed {
        identity: identity.clone(),
        format,
        message: failure.message,
        location: failure.location,
    })
}

/// A local data document read by [`parse_local_document`]: its value and
/// the text its positions are read from.
#[derive(Debug, Clone)]
pub struct LocalDataDocument {
    format: DataFormat,
    text: Arc<str>,
    value: Value,
}

impl LocalDataDocument {
    pub fn format(&self) -> DataFormat {
        self.format
    }

    pub fn value(&self) -> &Value {
        &self.value
    }

    pub fn into_value(self) -> Value {
        self.value
    }

    /// The BOM-stripped text the value was parsed from.
    pub fn text(&self) -> &str {
        &self.text
    }

    /// The 1-based line and character column at which the value at each of
    /// `paths` starts, in order; `None` where the format's locator cannot
    /// prove a position (a YAML layout the position map cannot match to the
    /// parsed value is never positioned), never a guessed one.
    pub fn positions(&self, paths: &[Vec<PathSegment>]) -> Vec<Option<(usize, usize)>> {
        let mut locator = SourceLocator::new(self.format, &self.text, None);
        paths.iter().map(|path| locator.place(path, false).position).collect()
    }
}

/// Reads a local data file's `bytes` as `format` with exactly the rules
/// `load_data` applies: one leading byte-order mark removed, the whole text
/// consumed, JSON and JSON5 keeping the last of duplicate keys, YAML and
/// TOML rejecting them, a second YAML document or a tag rejected, and every
/// number admitted through `simplified_schema::numeric`.
///
/// Only the tree formats (JSON, JSON5, JSON Lines, YAML, TOML) are read; a
/// CSV or TSV table has no value without a row mode.
///
/// ## Errors
///
/// [`DataError::UnsupportedFormat`] for a tabular or non-data `format`, and
/// [`DataError::Malformed`] for bytes that are not UTF-8 or text the format
/// rejects, located as [`decode_local`] and [`parse_text`] locate it.
pub fn parse_local_document(
    bytes: Vec<u8>,
    format: DataFormat,
    identity: &DataSourceIdentity,
) -> Result<LocalDataDocument, DataError> {
    let tree = matches!(
        format,
        DataFormat::Json | DataFormat::Json5 | DataFormat::Jsonl | DataFormat::Yaml | DataFormat::Toml
    );
    if !tree {
        return Err(DataError::UnsupportedFormat { identity: identity.clone(), extension: None });
    }
    let text = decode_local(bytes, identity, format)?;
    match parse_text(&text, format, identity)? {
        SourceContent::Tree(value) => Ok(LocalDataDocument { format, text, value }),
        SourceContent::Table(_) => Err(DataError::UnsupportedFormat { identity: identity.clone(), extension: None }),
    }
}

/// The line and character column of `offset` in `text`. An offset inside a
/// character is moved back to that character's start; one past the end is
/// clamped to the end.
pub(crate) fn located_at(text: &str, offset: usize) -> DataSourceLocation {
    let mut offset = offset.min(text.len());
    while !text.is_char_boundary(offset) {
        offset -= 1;
    }
    let (line, column) = source_position(text, offset).unzip();
    DataSourceLocation { line, column, ..DataSourceLocation::default() }
}

/// A number admission failure, which already carries a character position.
fn number_failure(error: JsonNumberError) -> Failure {
    let location = DataSourceLocation {
        pointer: Some(error.pointer.clone()),
        line: Some(error.line),
        column: Some(error.column),
        byte_offset: None,
    };
    Failure::new(error.to_string(), location)
}

fn parse_yaml(text: &str, identity: &DataSourceIdentity) -> Result<Value, Failure> {
    let name = identity.read_set_entry();
    let loaded = load_yaml_document(text, Path::new(&name)).map_err(|error| match error {
        SchemaError::SchemaDocument { message, span, .. } => {
            let location = span.map(|span| located_at(text, span.start)).unwrap_or_default();
            Failure::new(message, location)
        }
        other => Failure::new(other.to_string(), DataSourceLocation::default()),
    })?;
    // The shared loader keeps tags for its other callers; a data document
    // carries plain values only, so a tag is refused before its JSON form
    // (a one-key mapping named after the tag) can be used.
    if let Some((segments, tag)) = first_tag(&loaded.value, &mut Vec::new()) {
        let start = first_tag_start(text, &loaded);
        let location = DataSourceLocation {
            pointer: Some(pointer_string(&segments)),
            ..start.map(|start| located_at(text, start)).unwrap_or_default()
        };
        return Err(Failure::new(format!("the tag `{tag}` is not supported in a data file"), location));
    }
    Ok(loaded.json)
}

/// The path and tag of the first tagged node, in document order.
fn first_tag(value: &YamlValue, at: &mut Vec<String>) -> Option<(Vec<String>, String)> {
    match value {
        YamlValue::Tagged(tagged) => Some((at.clone(), tagged.tag.to_string())),
        YamlValue::Mapping(map) => map.iter().find_map(|(key, child)| {
            at.push(key.as_str().unwrap_or_default().to_string());
            let found = first_tag(child, at);
            at.pop();
            found
        }),
        YamlValue::Sequence(items) => items.iter().enumerate().find_map(|(index, item)| {
            at.push(index.to_string());
            let found = first_tag(item, at);
            at.pop();
            found
        }),
        _ => None,
    }
}

/// `serde_json`'s message without its own `at line L column C` suffix, whose
/// column counts bytes.
fn serde_json_message(error: &serde_json::Error) -> String {
    let message = error.to_string();
    let suffix = format!(" at line {} column {}", error.line(), error.column());
    message.strip_suffix(&suffix).map(str::to_string).unwrap_or(message)
}

/// The byte offset `serde_json` means by its 1-based line and byte column:
/// the line's start plus `column - 1` bytes.
fn serde_json_offset(text: &str, line: usize, column: usize) -> usize {
    let line_start = if line <= 1 {
        0
    } else {
        text.match_indices('\n').nth(line - 2).map_or(text.len(), |(index, _)| index + 1)
    };
    line_start + column.saturating_sub(1)
}

fn parse_json(text: &str) -> Result<Value, Failure> {
    let value: Value = serde_json::from_str(text).map_err(|error| {
        let offset = serde_json_offset(text, error.line(), error.column());
        Failure::new(serde_json_message(&error), located_at(text, offset))
    })?;
    numeric::admit_json_text_numbers(text).map_err(number_failure)?;
    Ok(value)
}

fn parse_json_lines(text: &str) -> Result<Value, Failure> {
    let lines = biscuit_file::read_json_lines(text).map_err(|error| match error {
        JsonLinesError::InvalidRecord { line, offset, source } => {
            Failure::new(format!("line {line}: {}", serde_json_message(&source)), located_at(text, offset))
        }
    })?;
    for (index, record) in lines.line_offsets.iter().enumerate() {
        numeric::admit_json_text_numbers(&text[record.text.clone()]).map_err(|error| {
            // The record's own pointer and column are relative to its text.
            let start = located_at(text, record.text.start);
            let location = DataSourceLocation {
                pointer: Some(format!("/{index}{}", error.pointer)),
                line: start.line,
                column: start.column.map(|column| column + error.column - 1),
                byte_offset: None,
            };
            Failure::new(format!("line {}: {error}", record.line), location)
        })?;
    }
    Ok(Value::Array(lines.values))
}

fn parse_toml(text: &str) -> Result<Value, Failure> {
    let document = Toml::from_str(text).map_err(|error| match error {
        biscuit_file::TomlError::Parse(parse) => {
            let location = parse.span().map(|span| located_at(text, span.start)).unwrap_or_default();
            Failure::new(parse.message().trim_end().to_string(), location)
        }
        other => Failure::new(other.to_string(), DataSourceLocation::default()),
    })?;
    toml_value(document.value(), &mut String::new()).map_err(|(pointer, message)| {
        let start = biscuit_file::locate_toml(text, &pointer);
        let location = DataSourceLocation {
            pointer: Some(pointer),
            ..start.map(|start| located_at(text, start)).unwrap_or_default()
        };
        Failure::new(message, location)
    })
}

/// A TOML value as JSON, every number admitted. Unlike biscuit-file's
/// `Toml::as_json_value`, which turns `nan` and `inf` into `null` and passes
/// every integer through, this rejects what the numeric model cannot hold.
/// A date or time becomes its TOML text, as that conversion does.
///
/// ## Errors
///
/// The pointer of the first inadmissible number and why.
fn toml_value(value: &biscuit_file::toml_crate::Value, pointer: &mut String) -> Result<Value, (String, String)> {
    use biscuit_file::toml_crate::Value as TomlValue;
    let rejected = |pointer: &str, written: String, reason: numeric::NumericAdmissionError| {
        let at = if pointer.is_empty() { "/" } else { pointer };
        (pointer.to_string(), format!("{at}: number {written} {reason}"))
    };
    Ok(match value {
        TomlValue::String(text) => Value::String(text.clone()),
        TomlValue::Boolean(flag) => Value::Bool(*flag),
        TomlValue::Datetime(datetime) => Value::String(datetime.to_string()),
        TomlValue::Integer(integer) => {
            numeric::admit_integer(i128::from(*integer))
                .map_err(|reason| rejected(pointer, integer.to_string(), reason))?;
            Value::Number((*integer).into())
        }
        TomlValue::Float(float) => {
            let admitted = numeric::admit_f64(*float).map_err(|reason| rejected(pointer, float.to_string(), reason))?;
            // Admission rejects every non-finite float, the only input
            // `from_f64` refuses; a `null` here would silently replace data.
            Value::Number(serde_json::Number::from_f64(admitted).expect("an admitted float is finite"))
        }
        TomlValue::Array(items) => {
            let mut converted = Vec::with_capacity(items.len());
            for (index, item) in items.iter().enumerate() {
                let length = pointer.len();
                pointer.push_str(&format!("/{index}"));
                converted.push(toml_value(item, pointer)?);
                pointer.truncate(length);
            }
            Value::Array(converted)
        }
        TomlValue::Table(table) => {
            let mut converted = serde_json::Map::with_capacity(table.len());
            for (key, item) in table {
                let length = pointer.len();
                pointer.push_str(&pointer_string(std::slice::from_ref(key)));
                converted.insert(key.clone(), toml_value(item, pointer)?);
                pointer.truncate(length);
            }
            Value::Object(converted)
        }
    })
}

fn parse_json5(text: &str) -> Result<Value, Failure> {
    let document = Json5::from_str(text).map_err(|error| {
        let location = biscuit_file::json5_parse_error_offset(text)
            .map(|offset| located_at(text, offset))
            .unwrap_or_default();
        Failure::new(error.to_string(), location)
    })?;
    // Admission reads each number's spelling, so an integer the parser has
    // already rounded to a nearby `f64` is still rejected.
    numeric::admit_json5_text_numbers(text).map_err(number_failure)?;
    numeric::reject_non_finite_json5(&document)
        .map_err(|reason| Failure::new(format!("a number {reason}"), DataSourceLocation::default()))?;
    Ok(document.value().clone())
}

fn parse_delimited(text: &str, delimiter: u8) -> Result<Arc<biscuit_file::DelimitedData>, Failure> {
    biscuit_file::read_delimited(text, delimiter).map(Arc::new).map_err(|error| {
        let offset = match &error {
            DelimitedError::UnterminatedQuote { offset, .. }
            | DelimitedError::QuoteInUnquotedField { offset, .. }
            | DelimitedError::TextAfterClosingQuote { offset, .. }
            | DelimitedError::RaggedRow { offset, .. }
            | DelimitedError::Desynchronized { offset, .. } => Some(*offset),
            DelimitedError::InvalidDelimiter(_) => None,
        };
        Failure::new(error.to_string(), offset.map(|offset| located_at(text, offset)).unwrap_or_default())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn identity() -> DataSourceIdentity {
        DataSourceIdentity::Local(std::path::PathBuf::from("/data/x"))
    }

    fn malformed(text: &str, format: DataFormat) -> (String, DataSourceLocation) {
        match parse_text(text, format, &identity()) {
            Err(DataError::Malformed { message, location, .. }) => (message, location),
            other => panic!("{text:?} as {format:?}: expected malformed, got {other:?}"),
        }
    }

    fn tree(text: &str, format: DataFormat) -> Value {
        match parse_text(text, format, &identity()) {
            Ok(SourceContent::Tree(value)) => value,
            other => panic!("{text:?} as {format:?}: {other:?}"),
        }
    }

    fn key(name: &str) -> PathSegment {
        PathSegment::Key(name.to_string())
    }

    #[test]
    fn a_local_document_keeps_the_reader_rules_and_positions_each_value() {
        let json = parse_local_document(b"{\"a\": 1, \"a\": 2,\n \"b\": {\"c\": 3}}".to_vec(), DataFormat::Json, &identity())
            .expect("JSON keeps the last duplicate");
        assert_eq!(json.value(), &serde_json::json!({"a": 2, "b": {"c": 3}}));
        assert_eq!(json.positions(&[vec![key("b"), key("c")], vec![key("z")]]), [Some((2, 13)), None]);

        let yaml = parse_local_document(b"\xEF\xBB\xBFpage:\n  color: red\n".to_vec(), DataFormat::Yaml, &identity())
            .expect("YAML reads");
        assert_eq!(yaml.positions(&[vec![key("page"), key("color")]]), [Some((2, 10))]);
        assert!(matches!(
            parse_local_document(b"a: 1\na: 2\n".to_vec(), DataFormat::Yaml, &identity()),
            Err(DataError::Malformed { .. })
        ));
    }

    #[test]
    fn a_local_document_is_a_tree_format_only() {
        let refused = parse_local_document(b"a,b\n1,2\n".to_vec(), DataFormat::Csv, &identity());
        assert!(matches!(refused, Err(DataError::UnsupportedFormat { .. })), "{refused:?}");
    }

    #[test]
    fn invalid_utf8_is_located_in_the_original_bytes() {
        let bytes = b"\xEF\xBB\xBFab\xFF".to_vec();
        match decode_local(bytes, &identity(), DataFormat::Json) {
            Err(DataError::Malformed { location, .. }) => {
                assert_eq!(location, DataSourceLocation { byte_offset: Some(5), ..DataSourceLocation::default() });
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn a_serde_json_byte_column_becomes_a_character_column() {
        let (message, location) = malformed("{\"a\": \"é\", x}", DataFormat::Json);
        assert_eq!((location.line, location.column), (Some(1), Some(12)), "{message}");
        assert!(!message.contains("column"), "the byte column is not repeated: {message}");
    }

    #[test]
    fn a_json_lines_number_failure_names_the_record_and_its_column() {
        let (_, location) = malformed("[1]\n\n  {\"n\": 9007199254740993}\n", DataFormat::Jsonl);
        assert_eq!(location.pointer.as_deref(), Some("/1/n"));
        assert_eq!((location.line, location.column), (Some(3), Some(9)));
    }

    #[test]
    fn toml_nan_is_rejected_at_its_pointer() {
        let (_, location) = malformed("[a]\nb = nan\n", DataFormat::Toml);
        assert_eq!(location.pointer.as_deref(), Some("/a/b"));
        assert_eq!(location.line, Some(2));
    }

    #[test]
    fn a_toml_datetime_is_text() {
        assert_eq!(tree("at = 1979-05-27\n", DataFormat::Toml), serde_json::json!({"at": "1979-05-27"}));
    }

    #[test]
    fn a_nested_yaml_tag_is_rejected_at_its_pointer() {
        let (message, location) = malformed("a:\n  - !shout loud\n", DataFormat::Yaml);
        assert!(message.contains("!shout"), "{message}");
        assert_eq!(location.pointer.as_deref(), Some("/a/0"));
        assert_eq!((location.line, location.column), (Some(2), Some(5)));
    }
}
