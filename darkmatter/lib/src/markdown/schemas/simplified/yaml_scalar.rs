//! Decoded-YAML-scalar-to-authored-byte projection.
//!
//! A single YAML scalar can be authored plain, single-quoted, or double-quoted
//! (with escapes). Any consumer that parses the *decoded* value (a schema
//! suggestion span, or a language server parsing an Expression-typed value) and
//! then needs to point back at the *authored* source must project decoded byte
//! offsets through the quoting and escaping. [`DecodedScalar`] carries that
//! byte-level map so the projection is exact rather than approximate.
//!
//! This module owns only the decoding + mapping; the schema-source scanner
//! ([`super::source`]) and DMLS both build on it.

use std::ops::Range;

/// A YAML scalar decoded from its authored source text, retaining a byte-level
/// map from each decoded byte boundary back to the authored (raw) source.
///
/// The map has one entry per decoded byte boundary (`decoded.len() + 1`
/// entries), so both a decoded byte offset and the position just past the last
/// decoded byte project. Raw offsets carry whatever `base` the scalar was
/// decoded with, so callers that decode a standalone value pass `base = 0` and
/// then add the value's document offset themselves.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodedScalar {
    decoded: String,
    /// One raw byte offset per decoded byte boundary (length `decoded.len()+1`).
    decoded_to_raw: Vec<usize>,
}

impl DecodedScalar {
    /// The decoded scalar text (quotes removed, escapes resolved).
    pub fn decoded(&self) -> &str {
        &self.decoded
    }

    /// The authored source offset of decoded byte boundary `decoded_offset`, or
    /// `None` when it is out of range.
    pub fn raw_offset(&self, decoded_offset: usize) -> Option<usize> {
        self.decoded_to_raw.get(decoded_offset).copied()
    }

    /// Projects a decoded byte range into an authored source range. `None` when
    /// either endpoint is out of range.
    pub fn project(&self, range: Range<usize>) -> Option<Range<usize>> {
        Some(self.raw_offset(range.start)?..self.raw_offset(range.end)?)
    }

    /// The decoded byte offset for an authored (raw) offset — the inverse of
    /// [`raw_offset`](Self::raw_offset), clamped into the decoded range.
    ///
    /// Used to convert a document cursor sitting inside the authored value into
    /// an offset the decoded-value parser understands. When several decoded
    /// bytes share one raw position (an escape), the earliest matching decoded
    /// boundary is returned.
    pub fn decoded_offset(&self, raw_offset: usize) -> usize {
        match self.decoded_to_raw.iter().rposition(|&raw| raw <= raw_offset) {
            Some(index) => index.min(self.decoded.len()),
            None => 0,
        }
    }
}

/// Decodes the whole authored text of one scalar value into a [`DecodedScalar`],
/// with raw offsets relative to the start of `raw`.
///
/// ## Returns
///
/// `None` when `raw` does not begin with a recognizable plain, single-quoted, or
/// double-quoted scalar (e.g. an empty string or a flow collection). Callers use
/// that as the signal to fall back to a whole-node range.
pub fn decode_scalar(raw: &str) -> Option<DecodedScalar> {
    decode_scalar_at(raw, 0).map(|(scalar, _)| scalar)
}

/// Decodes the leading scalar of `raw`, adding `base` to every raw coordinate,
/// and reports how many raw bytes it consumed (so a caller scanning a flow
/// sequence can advance). `None` when `raw` does not begin with a scalar.
pub fn decode_scalar_at(raw: &str, base: usize) -> Option<(DecodedScalar, usize)> {
    match raw.as_bytes().first().copied()? {
        b'\'' => decode_single_quoted(raw, base, false),
        b'"' => decode_double_quoted(raw, base, false),
        _ => {
            let content = raw.trim_end();
            if content.is_empty() {
                return None;
            }
            Some((plain_scalar(content, base), content.len()))
        }
    }
}

/// Decodes the leading scalar of `raw` the way [`decode_scalar_at`] does, but
/// accepts text that is still being authored.
///
/// A quoted scalar whose closing quote has not been typed yet decodes to
/// everything authored so far instead of failing, an empty input decodes to an
/// empty scalar, and a plain scalar keeps its trailing whitespace so a caller
/// can tell `min` from `min ` at the cursor. The decoded-to-authored map is
/// exact in every case, so a caller may still project spans through quoting,
/// escapes, and multibyte content.
///
/// ## Notes
///
/// This is the tolerant half of the projection seam: interactive callers ask
/// what a value means while it is mid-edit, when [`decode_scalar_at`] can only
/// answer `None`.
pub fn decode_partial_scalar_at(raw: &str, base: usize) -> (DecodedScalar, usize) {
    match raw.as_bytes().first().copied() {
        Some(b'\'') => decode_single_quoted(raw, base, true),
        Some(b'"') => decode_double_quoted(raw, base, true),
        _ => Some((plain_scalar(raw, base), raw.len())),
    }
    .unwrap_or_else(|| (plain_scalar("", base), 0))
}

fn plain_scalar(content: &str, base: usize) -> DecodedScalar {
    let mut map = Vec::with_capacity(content.len() + 1);
    map.extend(base..=base + content.len());
    DecodedScalar {
        decoded: content.to_string(),
        decoded_to_raw: map,
    }
}

fn decode_single_quoted(
    raw: &str,
    base: usize,
    tolerant: bool,
) -> Option<(DecodedScalar, usize)> {
    let mut decoded = String::new();
    let mut map = vec![base + 1];
    let mut cursor = 1;
    while cursor < raw.len() {
        if raw[cursor..].starts_with("''") {
            push_mapped(&mut decoded, &mut map, "'", base + cursor + 2);
            cursor += 2;
        } else if raw.as_bytes()[cursor] == b'\'' {
            return Some((DecodedScalar { decoded, decoded_to_raw: map }, cursor + 1));
        } else {
            let ch = raw[cursor..].chars().next()?;
            cursor += ch.len_utf8();
            push_mapped(&mut decoded, &mut map, &ch.to_string(), base + cursor);
        }
    }
    tolerant.then_some((DecodedScalar { decoded, decoded_to_raw: map }, cursor))
}

fn decode_double_quoted(
    raw: &str,
    base: usize,
    tolerant: bool,
) -> Option<(DecodedScalar, usize)> {
    let mut decoded = String::new();
    let mut map = vec![base + 1];
    let mut cursor = 1;
    while cursor < raw.len() {
        match raw.as_bytes()[cursor] {
            b'"' => {
                return Some((DecodedScalar { decoded, decoded_to_raw: map }, cursor + 1));
            }
            b'\\' => {
                // A half-typed escape (`"\` or `"\u00`) has no value yet; the
                // tolerant caller keeps what decoded cleanly before it.
                let Some((value, consumed)) = decode_yaml_escape(&raw[cursor..]) else {
                    return tolerant
                        .then_some((DecodedScalar { decoded, decoded_to_raw: map }, cursor));
                };
                cursor += consumed;
                push_mapped(&mut decoded, &mut map, &value, base + cursor);
            }
            _ => {
                let ch = raw[cursor..].chars().next()?;
                cursor += ch.len_utf8();
                push_mapped(&mut decoded, &mut map, &ch.to_string(), base + cursor);
            }
        }
    }
    tolerant.then_some((DecodedScalar { decoded, decoded_to_raw: map }, cursor))
}

fn decode_yaml_escape(raw: &str) -> Option<(String, usize)> {
    let escape = *raw.as_bytes().get(1)?;
    let simple = match escape {
        b'0' => Some('\0'),
        b'a' => Some('\u{7}'),
        b'b' => Some('\u{8}'),
        b't' | b'\t' => Some('\t'),
        b'n' => Some('\n'),
        b'v' => Some('\u{b}'),
        b'f' => Some('\u{c}'),
        b'r' => Some('\r'),
        b'e' => Some('\u{1b}'),
        b' ' => Some(' '),
        b'"' => Some('"'),
        b'/' => Some('/'),
        b'\\' => Some('\\'),
        b'N' => Some('\u{85}'),
        b'_' => Some('\u{a0}'),
        b'L' => Some('\u{2028}'),
        b'P' => Some('\u{2029}'),
        _ => None,
    };
    if let Some(ch) = simple {
        return Some((ch.to_string(), 2));
    }
    let digits = match escape {
        b'x' => 2,
        b'u' => 4,
        b'U' => 8,
        _ => return None,
    };
    let end = 2 + digits;
    let value = u32::from_str_radix(raw.get(2..end)?, 16).ok()?;
    Some((char::from_u32(value)?.to_string(), end))
}

fn push_mapped(decoded: &mut String, map: &mut Vec<usize>, value: &str, raw_end: usize) {
    decoded.push_str(value);
    while map.len() <= decoded.len() {
        map.push(raw_end);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_scalar_maps_one_to_one() {
        let scalar = decode_scalar("ctx.today").expect("plain scalar decodes");
        assert_eq!(scalar.decoded(), "ctx.today");
        assert_eq!(scalar.raw_offset(0), Some(0));
        assert_eq!(scalar.raw_offset(4), Some(4));
        assert_eq!(scalar.project(0..3), Some(0..3));
        // A plain scalar's cursor round-trips through the inverse map.
        assert_eq!(scalar.decoded_offset(4), 4);
    }

    #[test]
    fn single_quoted_projection_excludes_quotes() {
        // `'ctx.today'` — the decoded value starts one byte past the opening
        // quote, so a decoded offset of 0 projects to raw offset 1.
        let scalar = decode_scalar("'ctx.today'").expect("single-quoted decodes");
        assert_eq!(scalar.decoded(), "ctx.today");
        assert_eq!(scalar.raw_offset(0), Some(1));
        assert_eq!(scalar.project(0..3), Some(1..4));
    }

    #[test]
    fn single_quoted_escaped_quote_collapses() {
        // `''` decodes to one `'`; the map still projects each decoded byte.
        let scalar = decode_scalar("'a''b'").expect("decodes");
        assert_eq!(scalar.decoded(), "a'b");
        assert_eq!(scalar.raw_offset(0), Some(1));
        // The `b` after the escaped quote sits at raw offset 4 (`'a''b'`).
        assert_eq!(scalar.raw_offset(2), Some(4));
    }

    #[test]
    fn double_quoted_escape_projection() {
        // `"café"` decodes to `café`; the projection stays exact across the
        // multibyte escaped char.
        let scalar = decode_scalar("\"caf\\u00e9\"").expect("decodes");
        assert_eq!(scalar.decoded(), "café");
        assert_eq!(scalar.raw_offset(0), Some(1));
    }

    #[test]
    fn multibyte_plain_scalar_projects_by_byte() {
        let scalar = decode_scalar("café").expect("decodes");
        assert_eq!(scalar.decoded(), "café");
        // `é` is two bytes; the boundary past it is byte 5.
        assert_eq!(scalar.raw_offset(5), Some(5));
    }

    #[test]
    fn empty_and_missing_scalars_yield_none() {
        assert!(decode_scalar("").is_none());
        assert!(decode_scalar("   ").is_none());
    }
}
