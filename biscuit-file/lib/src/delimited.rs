//! Strict delimited-text reader for CSV and TSV (feature `delimited`).
//!
//! Both formats use CSV-style quoting: a field that starts with `"` is
//! quoted, may contain the delimiter and line breaks, and escapes a quote by
//! doubling it. The first record is the header. The reader returns cell text
//! exactly as decoded, never trimmed, and keeps each record's start line and
//! byte offset and each field's raw start in the source. It does not map rows
//! to objects or lists, or check header names: those rules depend on how the
//! caller uses the rows.
//!
//! The rules, beyond what the `csv` crate enforces:
//!
//! - a quoted field must be closed, and only the delimiter or a record
//!   terminator may follow its closing quote;
//! - an unquoted field must not contain a quote;
//! - every data row has exactly as many fields as the header.
//!
//! LF, CRLF, and a lone CR each end a record (the `csv` crate's terminator).
//! Line numbers count LF only, so a record after a lone CR keeps the line
//! number of the one before it. An empty physical line outside a quoted field
//! is skipped; a line holding only delimiters is a record of empty cells.
//!
//! ## Examples
//!
//! ```rust
//! use biscuit_file::delimited::read_delimited;
//!
//! let data = read_delimited("name,note\nada,\"says \"\"hi\"\", twice\"\n", b',')?;
//! let headers = data.headers.expect("header row");
//! assert_eq!(headers.fields[1].text, "note");
//! assert_eq!(data.rows[0].line, 2);
//! assert_eq!(data.rows[0].fields[1].text, "says \"hi\", twice");
//! assert_eq!(data.rows[0].fields[1].offset, 14);
//! # Ok::<(), biscuit_file::delimited::DelimitedError>(())
//! ```

use csv_core::{ReadFieldResult, Reader as CoreReader, ReaderBuilder as CoreReaderBuilder};
use thiserror::Error;

/// The header and data rows of a delimited text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DelimitedData {
    /// The first record, or `None` when the text has no records (it is empty
    /// or holds only blank lines).
    pub headers: Option<DelimitedRecord>,
    /// Every record after the header, in source order. Each has as many
    /// fields as `headers`.
    pub rows: Vec<DelimitedRecord>,
}

/// One record (a header or a data row) and where it starts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DelimitedRecord {
    /// 1-based line on which the record starts.
    pub line: usize,
    /// Byte offset of the record's first field, equal to `fields[0].offset`.
    pub offset: usize,
    /// The record's fields, in source order; never empty.
    pub fields: Vec<DelimitedField>,
}

/// One decoded field and its raw position.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DelimitedField {
    /// The cell text with quoting removed and doubled quotes collapsed.
    pub text: String,
    /// Byte offset of the field's first source byte, which is the opening
    /// quote of a quoted field.
    pub offset: usize,
}

/// A delimited text that could not be read.
///
/// Every positional variant carries a byte offset into the source text and
/// the 1-based line containing it.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum DelimitedError {
    /// The delimiter is a quote, a line break, or not an ASCII byte.
    #[error("invalid delimiter byte 0x{0:02x}")]
    InvalidDelimiter(u8),
    /// A quoted field has no closing quote; `offset` is its opening quote.
    #[error("unterminated quoted field starting on line {line} (byte {offset})")]
    UnterminatedQuote {
        /// Line of the opening quote.
        line: usize,
        /// Byte offset of the opening quote.
        offset: usize,
    },
    /// An unquoted field contains a quote; `offset` is that quote.
    #[error("quote inside an unquoted field on line {line} (byte {offset})")]
    QuoteInUnquotedField {
        /// Line of the quote.
        line: usize,
        /// Byte offset of the quote.
        offset: usize,
    },
    /// Text follows a closing quote before the next delimiter or record end;
    /// `offset` is the first such byte.
    #[error("text after a closing quote on line {line} (byte {offset})")]
    TextAfterClosingQuote {
        /// Line of the first byte after the closing quote.
        line: usize,
        /// Byte offset of the first byte after the closing quote.
        offset: usize,
    },
    /// A data row's field count differs from the header's; `line` and
    /// `offset` are the row's start.
    #[error("row on line {line} (byte {offset}) has {found} fields; the header has {expected}")]
    RaggedRow {
        /// Line on which the row starts.
        line: usize,
        /// Byte offset of the row's first field.
        offset: usize,
        /// The header's field count.
        expected: usize,
        /// The row's field count.
        found: usize,
    },
    /// The decoding and offset passes disagreed about a record. This is a
    /// defect in this reader, never a property of the input.
    #[error("delimited reader passes disagree at line {line} (byte {offset})")]
    Desynchronized {
        /// Line of the record where the passes diverged.
        line: usize,
        /// Byte offset of that record.
        offset: usize,
    },
}

/// Reads delimited text (`b','` for CSV, `b'\t'` for TSV) into a header and
/// data rows.
///
/// ## Errors
///
/// Returns the first violation in source order: a quote rule
/// ([`DelimitedError::UnterminatedQuote`],
/// [`DelimitedError::QuoteInUnquotedField`],
/// [`DelimitedError::TextAfterClosingQuote`]) or a row whose width differs
/// from the header's ([`DelimitedError::RaggedRow`]). An unusable `delimiter`
/// is [`DelimitedError::InvalidDelimiter`].
pub fn read_delimited(text: &str, delimiter: u8) -> Result<DelimitedData, DelimitedError> {
    if !delimiter.is_ascii() || matches!(delimiter, b'"' | b'\r' | b'\n') {
        return Err(DelimitedError::InvalidDelimiter(delimiter));
    }

    let positions = scan_positions(text, delimiter)?;

    // The `csv` crate decodes the same records; the scan above has already
    // proven the text strict, so the two agree record for record.
    let mut reader = csv::ReaderBuilder::new()
        .delimiter(delimiter)
        .has_headers(false)
        .flexible(true)
        .from_reader(text.as_bytes());
    let mut records = Vec::with_capacity(positions.len());
    let mut decoded = reader.records();
    for position in positions {
        let desynchronized = DelimitedError::Desynchronized {
            line: position.line,
            offset: position.field_offsets[0],
        };
        let record = match decoded.next() {
            Some(Ok(record)) if record.len() == position.field_offsets.len() => record,
            _ => return Err(desynchronized),
        };
        records.push(DelimitedRecord {
            line: position.line,
            offset: position.field_offsets[0],
            fields: record
                .iter()
                .zip(position.field_offsets)
                .map(|(text, offset)| DelimitedField {
                    text: text.to_owned(),
                    offset,
                })
                .collect(),
        });
    }
    if decoded.next().is_some() {
        let (line, offset) = records
            .last()
            .map_or((1, 0), |record| (record.line, record.offset));
        return Err(DelimitedError::Desynchronized { line, offset });
    }

    let mut records = records.into_iter();
    Ok(DelimitedData {
        headers: records.next(),
        rows: records.collect(),
    })
}

/// A record's start line and the raw start of each of its fields.
struct RecordPosition {
    line: usize,
    field_offsets: Vec<usize>,
}

/// Walks `text` with `csv-core`, recording raw field starts and enforcing the
/// quote and width rules.
///
/// `csv-core` reports how many input bytes each `read_field` call consumed.
/// A call consumes the field's trailing delimiter or its record's first
/// terminator byte, and the next call first discards any further terminator
/// bytes (a CRLF's LF, blank lines), so a record's first field starts after
/// those discarded bytes.
fn scan_positions(text: &str, delimiter: u8) -> Result<Vec<RecordPosition>, DelimitedError> {
    let bytes = text.as_bytes();
    let mut core: CoreReader = CoreReaderBuilder::new().delimiter(delimiter).build();
    // A field decodes to at most its source length, so this never fills.
    let mut output = vec![0; bytes.len().max(1)];
    let mut lines = LineCounter::default();
    let mut records: Vec<RecordPosition> = Vec::new();
    let mut fields: Vec<usize> = Vec::new();
    // `csv-core` skips a leading byte-order mark and counts it as consumed.
    let mut cursor = if text.starts_with('\u{feff}') { 3 } else { 0 };
    let mut call_start = cursor;

    loop {
        let (result, consumed, _) = core.read_field(&bytes[cursor..], &mut output);
        cursor += consumed;
        let record_end = match result {
            ReadFieldResult::InputEmpty | ReadFieldResult::OutputFull => continue,
            ReadFieldResult::End => break,
            ReadFieldResult::Field { record_end } => record_end,
        };

        let mut field_start = call_start;
        if fields.is_empty() {
            while matches!(bytes.get(field_start), Some(b'\r' | b'\n')) {
                field_start += 1;
            }
        }
        // `consumed` is zero only for the final call at end of input, which
        // consumes no delimiter or terminator.
        let field_end = if consumed > 0 { cursor - 1 } else { cursor };
        validate_quoting(bytes, field_start, field_end.max(field_start), &mut lines)?;
        fields.push(field_start);
        call_start = cursor;

        if record_end {
            let field_offsets = std::mem::take(&mut fields);
            let line = lines.line_at(bytes, field_offsets[0]);
            if let Some(header) = records.first()
                && header.field_offsets.len() != field_offsets.len()
            {
                return Err(DelimitedError::RaggedRow {
                    line,
                    offset: field_offsets[0],
                    expected: header.field_offsets.len(),
                    found: field_offsets.len(),
                });
            }
            records.push(RecordPosition {
                line,
                field_offsets,
            });
        }
    }
    Ok(records)
}

/// Checks one raw field, `bytes[start..end]`, against the strict quote rules.
fn validate_quoting(
    bytes: &[u8],
    start: usize,
    end: usize,
    lines: &mut LineCounter,
) -> Result<(), DelimitedError> {
    let field = &bytes[start..end];
    if field.first() != Some(&b'"') {
        return match field.iter().position(|&byte| byte == b'"') {
            Some(index) => {
                let offset = start + index;
                Err(DelimitedError::QuoteInUnquotedField {
                    line: lines.line_at(bytes, offset),
                    offset,
                })
            }
            None => Ok(()),
        };
    }

    let mut index = 1;
    loop {
        match field[index..].iter().position(|&byte| byte == b'"') {
            None => {
                return Err(DelimitedError::UnterminatedQuote {
                    line: lines.line_at(bytes, start),
                    offset: start,
                });
            }
            Some(found) => {
                let quote = index + found;
                if field.get(quote + 1) == Some(&b'"') {
                    index = quote + 2;
                } else if quote + 1 < field.len() {
                    let offset = start + quote + 1;
                    return Err(DelimitedError::TextAfterClosingQuote {
                        line: lines.line_at(bytes, offset),
                        offset,
                    });
                } else {
                    return Ok(());
                }
            }
        }
    }
}

/// Converts monotonically increasing byte offsets to 1-based LF line numbers
/// without rescanning the text from the start.
#[derive(Default)]
struct LineCounter {
    offset: usize,
    newlines: usize,
}

impl LineCounter {
    fn line_at(&mut self, bytes: &[u8], offset: usize) -> usize {
        if offset < self.offset {
            // Callers ask in source order; recount rather than miscount.
            return 1 + bytes[..offset].iter().filter(|&&byte| byte == b'\n').count();
        }
        self.newlines += bytes[self.offset..offset]
            .iter()
            .filter(|&&byte| byte == b'\n')
            .count();
        self.offset = offset;
        1 + self.newlines
    }
}
