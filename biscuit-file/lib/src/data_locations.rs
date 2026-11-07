//! Where the value at a JSON Pointer was written, per data format.
//!
//! Each locator takes the source text and an RFC 6901 pointer (`""` is the
//! root; `~1` decodes to `/` and `~0` to `~`) and returns the byte offset in
//! that text at which the addressed value starts. Convert the offset with
//! [`source_position`](crate::source_position::source_position).
//!
//! A locator returns `None`, and the caller reports the pointer alone, when:
//!
//! - the text does not parse in that format;
//! - the pointer is malformed (non-empty without a leading `/`, a `~` not
//!   followed by `0` or `1`, or an array index with a sign or leading zero);
//! - no value exists at the pointer. A caller holding a pointer to a missing
//!   child locates its parent pointer instead.
//!
//! A duplicate object key resolves to its last occurrence in JSON and JSON5,
//! matching the value the parsers keep. CSV and TSV need no locator here:
//! [`DelimitedField::offset`](crate::delimited::DelimitedField) is already a
//! source offset.
//!
//! ## Examples
//!
//! ```rust
//! use biscuit_file::{locate_json, source_position};
//!
//! let text = "{\n  \"servers\": [{\"port\": 1}, {\"port\": \"x\"}]\n}";
//! let offset = locate_json(text, "/servers/1/port").unwrap();
//! assert_eq!(&text[offset..offset + 3], "\"x\"");
//! assert_eq!(source_position(text, offset), Some((2, 37)));
//! assert_eq!(locate_json(text, "/servers/2"), None);
//! ```

use std::fmt;

use serde::Deserialize;
use serde::de::{Deserializer, MapAccess, Visitor};
use serde_json::value::RawValue;

use crate::json_lines::read_json_lines;

/// The byte offset in `text`, a JSON document, of the value at `pointer`.
///
/// The root is the first non-whitespace byte.
#[must_use]
pub fn locate_json(text: &str, pointer: &str) -> Option<usize> {
    let segments = parse_pointer(pointer)?;
    let mut raw = serde_json::from_str::<&RawValue>(text).ok()?;
    for segment in &segments {
        raw = json_child(raw, segment)?;
    }
    subslice_offset(text, raw.get())
}

/// The byte offset in `text`, a JSON Lines document, of the value at
/// `pointer`.
///
/// The pointer's first segment indexes the non-blank records, the same
/// records [`read_json_lines`] returns; the rest is located within that
/// record. The root is the first record's first byte, or `0` when there is
/// none.
#[must_use]
pub fn locate_jsonl(text: &str, pointer: &str) -> Option<usize> {
    let records = read_json_lines(text).ok()?.line_offsets;
    if pointer.is_empty() {
        return Some(records.first().map_or(0, |record| record.text.start));
    }
    let body = pointer.strip_prefix('/')?;
    let head = body.split('/').next().unwrap_or(body);
    let rest = &body[head.len()..];
    let record = records.get(array_index(&decode_segment(head)?)?)?;
    let located = locate_json(&text[record.text.clone()], rest)?;
    Some(record.text.start + located)
}

/// The byte offset in `text`, a TOML document, of the value at `pointer`.
///
/// A table written as a `[header]` starts at its header, an array of tables
/// at its first `[[header]]`. The root starts at byte `0`.
#[cfg(feature = "toml")]
#[must_use]
pub fn locate_toml(text: &str, pointer: &str) -> Option<usize> {
    use toml::de::{DeTable, DeValue};

    let segments = parse_pointer(pointer)?;
    let root = DeTable::parse(text).ok()?;
    let Some((first, rest)) = segments.split_first() else {
        return Some(root.span().start);
    };
    let mut value = toml_table_child(root.get_ref(), first)?;
    for segment in rest {
        value = match value.get_ref() {
            DeValue::Table(table) => toml_table_child(table, segment)?,
            DeValue::Array(array) => array.get(array_index(segment)?)?,
            _ => return None,
        };
    }
    Some(value.span().start)
}

/// The byte offset in `text`, a JSON5 document, of the value at `pointer`.
///
/// A signed number starts at its sign. The root is its first token.
#[cfg(feature = "json5")]
#[must_use]
pub fn locate_json5(text: &str, pointer: &str) -> Option<usize> {
    let segments = parse_pointer(pointer)?;
    let tokens = json_five::tokenize_str(text).ok()?;
    json_five::model_from_tokens(&tokens).ok()?;
    let spans: Vec<(usize, &json_five::tokenize::TokType, usize)> =
        tokens.tok_spans.iter().map(|(start, kind, end)| (*start, kind, *end)).collect();
    let mut at = 0;
    for segment in &segments {
        at = json5_child(text, &spans, at, segment)?;
    }
    spans.get(at).map(|(start, _, _)| *start)
}

/// The byte offset in `text` at which reading it as a JSON5 value fails, or
/// `None` when it reads.
///
/// [`Json5`](crate::Json5) reports a failure as text only; this recovers its
/// position. A syntax error is located where the parser stops. A grammatical
/// text can still fail to become a value: a number too large for its type or
/// a string or key with an invalid escape is located at the start of that
/// token (a signed number at its sign).
///
/// ## Examples
///
/// ```rust
/// use biscuit_file::json5_parse_error_offset;
///
/// assert_eq!(json5_parse_error_offset("{a: 1, b: ]}"), Some(10));
/// assert_eq!(json5_parse_error_offset("{a: 1}"), None);
/// ```
#[cfg(feature = "json5")]
#[must_use]
pub fn json5_parse_error_offset(text: &str) -> Option<usize> {
    // `ParsingError::index` is a byte offset (`char_index` is the char count).
    if let Err(error) = json_five::parser::from_str(text) {
        return Some(error.index.min(text.len()));
    }
    json5_value_error_offset(text)
}

/// The start of the first token of grammatical `text` that cannot become a
/// value. Each token is read on its own, as the value reader would read it.
#[cfg(feature = "json5")]
fn json5_value_error_offset(text: &str) -> Option<usize> {
    use json_five::tokenize::TokType;

    let tokens = json_five::tokenize_str(text).ok()?;
    let mut previous: Option<(usize, &TokType)> = None;
    for (start, kind, end) in &tokens.tok_spans {
        let lexeme = &text[*start..*end];
        let reads = match kind {
            TokType::SingleQuotedString | TokType::DoubleQuotedString => {
                json_five::from_str::<serde_json::Value>(lexeme).is_ok()
            }
            // An unquoted key may hold `\u` escapes; it reads only as a key.
            TokType::Name => json_five::from_str::<serde_json::Value>(&format!("{{{lexeme}: 0}}")).is_ok(),
            TokType::Integer | TokType::Float | TokType::Exponent | TokType::Hexadecimal => {
                json_five::from_str::<serde_json::Value>(lexeme).is_ok()
            }
            _ => true,
        };
        if !reads {
            return Some(match previous {
                Some((sign, TokType::Plus | TokType::Minus)) => sign,
                _ => *start,
            });
        }
        previous = Some((*start, kind));
    }
    None
}

/// The decoded segments of `pointer`; `None` when it is malformed.
fn parse_pointer(pointer: &str) -> Option<Vec<String>> {
    if pointer.is_empty() {
        return Some(Vec::new());
    }
    pointer.strip_prefix('/')?.split('/').map(decode_segment).collect()
}

fn decode_segment(raw: &str) -> Option<String> {
    let mut decoded = String::with_capacity(raw.len());
    let mut chars = raw.chars();
    while let Some(c) = chars.next() {
        if c == '~' {
            match chars.next()? {
                '0' => decoded.push('~'),
                '1' => decoded.push('/'),
                _ => return None,
            }
        } else {
            decoded.push(c);
        }
    }
    Some(decoded)
}

/// An RFC 6901 array index: `0`, or digits without a leading zero.
fn array_index(segment: &str) -> Option<usize> {
    let well_formed = !segment.is_empty()
        && segment.bytes().all(|byte| byte.is_ascii_digit())
        && (segment == "0" || !segment.starts_with('0'));
    if well_formed { segment.parse().ok() } else { None }
}

fn subslice_offset(text: &str, slice: &str) -> Option<usize> {
    let offset = (slice.as_ptr() as usize).checked_sub(text.as_ptr() as usize)?;
    (offset + slice.len() <= text.len()).then_some(offset)
}

fn json_child<'a>(raw: &'a RawValue, segment: &str) -> Option<&'a RawValue> {
    let text = raw.get();
    match text.as_bytes().first()? {
        b'{' => {
            let JsonEntries(entries) = serde_json::from_str::<JsonEntries<'a>>(text).ok()?;
            entries.into_iter().rev().find(|(key, _)| key == segment).map(|(_, value)| value)
        }
        b'[' => {
            let items = serde_json::from_str::<Vec<&'a RawValue>>(text).ok()?;
            items.get(array_index(segment)?).copied()
        }
        _ => None,
    }
}

/// An object's members in written order, duplicates kept, each value still
/// borrowed as its raw text.
struct JsonEntries<'a>(Vec<(String, &'a RawValue)>);

impl<'de: 'a, 'a> Deserialize<'de> for JsonEntries<'a> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct EntriesVisitor;

        impl<'de> Visitor<'de> for EntriesVisitor {
            type Value = JsonEntries<'de>;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a JSON object")
            }

            fn visit_map<A: MapAccess<'de>>(self, mut access: A) -> Result<Self::Value, A::Error> {
                let mut entries = Vec::new();
                while let Some(entry) = access.next_entry::<String, &'de RawValue>()? {
                    entries.push(entry);
                }
                Ok(JsonEntries(entries))
            }
        }

        deserializer.deserialize_map(EntriesVisitor)
    }
}

#[cfg(feature = "toml")]
fn toml_table_child<'a, 'i>(
    table: &'a toml::de::DeTable<'i>,
    key: &str,
) -> Option<&'a toml::Spanned<toml::de::DeValue<'i>>> {
    table.iter().find(|(name, _)| name.get_ref().as_ref() == key).map(|(_, value)| value)
}

#[cfg(feature = "json5")]
type Json5Span<'t> = (usize, &'t json_five::tokenize::TokType, usize);

/// The token index of the child `segment` of the container starting at token
/// `at`. Assumes grammatical tokens; `None` for a scalar or a missing child.
#[cfg(feature = "json5")]
fn json5_child(text: &str, spans: &[Json5Span<'_>], at: usize, segment: &str) -> Option<usize> {
    use json_five::tokenize::TokType;

    match spans.get(at)?.1 {
        TokType::LeftBrace => {
            let mut index = at + 1;
            let mut found = None;
            loop {
                let (start, kind, end) = *spans.get(index)?;
                if *kind == TokType::RightBrace {
                    return found;
                }
                // key, `:`, value
                let value = index + 2;
                if json5_key(&text[start..end], kind)? == segment {
                    found = Some(value);
                }
                index = json5_value_end(spans, value)?;
                if *spans.get(index)?.1 == TokType::Comma {
                    index += 1;
                }
            }
        }
        TokType::LeftBracket => {
            let target = array_index(segment)?;
            let mut index = at + 1;
            for _ in 0..target {
                if *spans.get(index)?.1 == TokType::RightBracket {
                    return None;
                }
                index = json5_value_end(spans, index)?;
                if *spans.get(index)?.1 == TokType::Comma {
                    index += 1;
                }
            }
            (*spans.get(index)?.1 != TokType::RightBracket).then_some(index)
        }
        _ => None,
    }
}

/// The token index just past the value starting at token `at`.
#[cfg(feature = "json5")]
fn json5_value_end(spans: &[Json5Span<'_>], at: usize) -> Option<usize> {
    use json_five::tokenize::TokType;

    match spans.get(at)?.1 {
        TokType::Plus | TokType::Minus => json5_value_end(spans, at + 1),
        TokType::LeftBrace | TokType::LeftBracket => {
            let mut depth = 0_usize;
            for (index, (_, kind, _)) in spans.iter().enumerate().skip(at) {
                match kind {
                    TokType::LeftBrace | TokType::LeftBracket => depth += 1,
                    TokType::RightBrace | TokType::RightBracket => {
                        depth -= 1;
                        if depth == 0 {
                            return Some(index + 1);
                        }
                    }
                    _ => {}
                }
            }
            None
        }
        TokType::EOF => None,
        _ => Some(at + 1),
    }
}

/// A member name decoded the way json-five decodes it into a value tree.
#[cfg(feature = "json5")]
fn json5_key(lexeme: &str, kind: &json_five::tokenize::TokType) -> Option<String> {
    use json_five::tokenize::TokType;

    let quoted = matches!(kind, TokType::SingleQuotedString | TokType::DoubleQuotedString);
    if !lexeme.contains('\\') {
        let unquoted = if quoted { lexeme.get(1..lexeme.len() - 1)? } else { lexeme };
        return Some(unquoted.to_string());
    }
    if quoted {
        return json_five::from_str::<String>(lexeme).ok();
    }
    // An identifier escape (`\u0061`) decodes only in member-name position.
    json_five::from_str::<serde_json::Map<String, serde_json::Value>>(&format!("{{{lexeme}:0}}"))
        .ok()?
        .into_iter()
        .next()
        .map(|(key, _)| key)
}
