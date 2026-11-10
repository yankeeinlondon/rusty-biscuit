//! JSON Lines (JSONL, NDJSON) reader: one JSON value per physical line.
//!
//! Lines break at LF; a CR before the LF is JSON whitespace and is trimmed
//! with the rest of the record. A line holding only JSON whitespace (space,
//! tab, CR) is skipped, but still counts toward line numbers. Zero-byte text
//! has no records. A leading byte-order mark is not stripped: callers that
//! read files strip it first, and every offset is into the text as given.
//!
//! Each record keeps its physical position, so a caller can locate a value
//! or re-read its original spelling (for example, to admit a number exactly
//! as written) from `text[record.text.clone()]`.
//!
//! ## Examples
//!
//! ```rust
//! use biscuit_file::json_lines::read_json_lines;
//!
//! let rows = read_json_lines("{\"a\":1}\n\n[2]\r\n")?;
//! assert_eq!(rows.values, vec![serde_json::json!({"a": 1}), serde_json::json!([2])]);
//! assert_eq!(rows.line_offsets[1].line, 3);
//! assert_eq!(rows.line_offsets[1].offset, 9);
//! # Ok::<(), biscuit_file::json_lines::JsonLinesError>(())
//! ```

use std::ops::Range;

use serde_json::Value;
use thiserror::Error;

/// The values of a JSON Lines text and where each one was written.
///
/// `values` and `line_offsets` are parallel: `line_offsets[i]` locates
/// `values[i]`.
#[derive(Debug, Clone, PartialEq)]
pub struct JsonLines {
    /// One parsed value per non-blank line, in source order.
    pub values: Vec<Value>,
    /// The source position of each value in `values`.
    pub line_offsets: Vec<JsonLineRecord>,
}

/// Where one JSON Lines record sits in the source text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JsonLineRecord {
    /// 1-based physical line number, counting skipped blank lines.
    pub line: usize,
    /// Byte offset of the start of the record's line.
    pub offset: usize,
    /// Byte range of the record's text with surrounding JSON whitespace
    /// (including a CRLF's CR) trimmed; always within the record's line.
    pub text: Range<usize>,
}

/// A JSON Lines text that could not be read.
#[derive(Debug, Error)]
pub enum JsonLinesError {
    /// A non-blank line is not exactly one valid JSON value.
    #[error("invalid JSON on line {line} (byte {offset}): {source}")]
    InvalidRecord {
        /// 1-based physical line number of the invalid record.
        line: usize,
        /// Byte offset of the parser error within the whole text.
        offset: usize,
        /// The parser's error. Its own line and column are relative to the
        /// record's line; use `line` and `offset` instead.
        #[source]
        source: serde_json::Error,
    },
}

/// Parses JSON Lines text into one value per non-blank line.
///
/// ## Errors
///
/// Returns [`JsonLinesError::InvalidRecord`] for the first line that is not
/// exactly one JSON value, with that line's number and the byte offset of the
/// parser error in `text`.
pub fn read_json_lines(text: &str) -> Result<JsonLines, JsonLinesError> {
    let mut values = Vec::new();
    let mut line_offsets = Vec::new();
    let mut line_start = 0;

    for (index, raw_line) in text.split('\n').enumerate() {
        let line = index + 1;
        let offset = line_start;
        line_start += raw_line.len() + 1;

        let trimmed = raw_line.trim_matches(is_json_whitespace);
        if trimmed.is_empty() {
            continue;
        }

        let value = serde_json::from_str::<Value>(raw_line).map_err(|source| {
            // serde_json reports a 1-based byte column within the parsed line.
            let column = source.column().saturating_sub(1).min(raw_line.len());
            JsonLinesError::InvalidRecord {
                line,
                offset: offset + column,
                source,
            }
        })?;

        let leading = raw_line.len() - raw_line.trim_start_matches(is_json_whitespace).len();
        let start = offset + leading;
        values.push(value);
        line_offsets.push(JsonLineRecord {
            line,
            offset,
            text: start..start + trimmed.len(),
        });
    }

    Ok(JsonLines {
        values,
        line_offsets,
    })
}

/// The whitespace JSON permits around a value (RFC 8259, section 2), less LF,
/// which ends the record.
fn is_json_whitespace(c: char) -> bool {
    matches!(c, ' ' | '\t' | '\r')
}
