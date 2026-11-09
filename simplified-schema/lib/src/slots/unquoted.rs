//! The "quote the slot" hint: a `{{ … }}` written as an unquoted YAML value.
//!
//! YAML reads an unquoted `status: {{ x }}` as a flow mapping whose key is a
//! mapping, and `status: {{ x }}(required)` does not decode at all. The
//! scanner here reads the authored text, not the decoded value, so it can
//! tell that mistake apart from a genuinely non-string key or an unrelated
//! YAML error. See "Quote a slot that starts a YAML value" in
//! `simplified-schema/docs/topics/language/slots.md`.

use crate::expression::scan::scan_expression_at;
use crate::span::SourceSpan;

/// The span of the first mapping value or block-list item in the YAML `text`
/// that starts with an unquoted `{{ … }}` slot.
///
/// A hit's first non-space bytes after `key:` or `- ` are a slot opener the
/// expression scanner closes (so `{{{` and `{{!data:` are not slots). Text
/// inside quoted scalars (including multi-line ones), comments, and block
/// scalars (`|`, `>` and their indented continuation lines) is never a hit.
///
/// `error_line` is a 1-based line, as YAML decode errors report it
/// (`serde_yaml_ng::Location::line`): with `Some(n)`, only a hit on line `n`
/// or `n - 1` counts; with `None`, the first hit anywhere does.
///
/// ## Returns
///
/// The value's byte range in `text`: from the opener to the end of the
/// value, trimmed and excluding a trailing comment. A slot that spans lines
/// ends where the expression closes.
#[must_use]
pub fn unquoted_slot_hint(text: &str, error_line: Option<usize>) -> Option<SourceSpan> {
    unquoted_slots(text)
        .into_iter()
        .find(|hit| error_line.is_none_or(|line| hit.line == line || hit.line + 1 == line))
        .map(|hit| hit.span)
}

/// The unquoted slot value a YAML decode error on 1-based `error_line` of
/// `text` is blamed on: the [`unquoted_slot_hint`] for that line, kept only
/// when the same text with that value single-quoted decodes, or fails on a
/// later line. The proof keeps an unrelated error on the line after a
/// slot (`a: {{ x }}` then `b: : c`) from being labelled a slot error.
#[must_use]
pub fn decode_error_slot(text: &str, error_line: usize) -> Option<SourceSpan> {
    let slot = unquoted_slot_hint(text, Some(error_line))?;
    let (quoted, _) = quote_slots(text, std::slice::from_ref(&slot));
    match serde_yaml_ng::from_str::<serde_yaml_ng::Value>(&quoted) {
        Ok(_) => Some(slot),
        Err(error) => error.location().is_some_and(|location| location.line() > error_line).then_some(slot),
    }
}

/// The message that replaces a YAML error `slot` (a span
/// [`unquoted_slot_hint`] returned for `text`) explains. It names the key the
/// value belongs to when the line has one, and the list form otherwise.
#[must_use]
pub fn quote_the_slot_message(text: &str, slot: &SourceSpan) -> String {
    let line_start = text[..slot.start].rfind('\n').map_or(0, |newline| newline + 1);
    let lead = text[line_start..slot.start].trim_end();
    match lead.strip_suffix(':') {
        Some(key) => {
            let key = key.trim_start().trim_start_matches(['-', ' ', '\t']);
            format!("quote the slot: write `{key}: \"{{{{ … }}}}\"`")
        }
        None => "quote the slot: write `- \"{{ … }}\"`".to_string(),
    }
}

/// Every [`unquoted_slot_hint`] in `text`, in order.
pub(crate) fn unquoted_slot_spans(text: &str) -> Vec<SourceSpan> {
    unquoted_slots(text).into_iter().map(|hit| hit.span).collect()
}

/// `text` with each of `slots` (ascending, disjoint spans in it) written as
/// a single-quoted YAML scalar, and where each quoted slot starts in it.
pub(crate) fn quote_slots(text: &str, slots: &[SourceSpan]) -> (String, Vec<usize>) {
    let mut quoted = String::with_capacity(text.len() + 2 * slots.len());
    let mut starts = Vec::with_capacity(slots.len());
    let mut copied = 0;
    for slot in slots {
        quoted.push_str(&text[copied..slot.start]);
        starts.push(quoted.len());
        quoted.push('\'');
        quoted.push_str(&text[slot.clone()].replace('\'', "''"));
        quoted.push('\'');
        copied = slot.end;
    }
    quoted.push_str(&text[copied..]);
    (quoted, starts)
}

struct Hit {
    /// 1-based line of the opener.
    line: usize,
    span: SourceSpan,
}

/// What the previous lines leave open.
#[derive(Clone, Copy)]
enum Carry {
    Nothing,
    /// A block scalar whose content lines are indented past `parent`.
    Block { parent: usize },
    /// A quoted scalar still waiting for its closing `quote`.
    Quoted { quote: u8 },
}

fn unquoted_slots(text: &str) -> Vec<Hit> {
    let mut hits = Vec::new();
    let mut carry = Carry::Nothing;
    let mut line_start = 0;
    for (index, raw) in text.split('\n').enumerate() {
        let start = line_start;
        line_start += raw.len() + 1;
        let line = raw.strip_suffix('\r').unwrap_or(raw);
        let indent = line.len() - line.trim_start_matches(' ').len();
        match carry {
            Carry::Block { parent } if line.trim().is_empty() || indent > parent => continue,
            Carry::Quoted { quote } => {
                if closing_quote(line, 0, quote).is_some() {
                    carry = Carry::Nothing;
                }
                continue;
            }
            _ => carry = Carry::Nothing,
        }

        let mut pos = indent;
        let mut item_column = None;
        while let Some(after) = list_item_content(line, pos) {
            item_column = Some(pos);
            pos = after;
        }
        let (value_pos, parent) = match mapping_value(line, pos) {
            Some(value_pos) => (value_pos, pos),
            None => match item_column {
                Some(column) => (pos, column),
                None => continue,
            },
        };
        let value = &line[value_pos..];
        match value.as_bytes().first() {
            Some(b'|' | b'>') => carry = Carry::Block { parent },
            Some(&quote @ (b'"' | b'\'')) => {
                if closing_quote(value, 1, quote).is_none() {
                    carry = Carry::Quoted { quote };
                }
            }
            Some(b'{') => {
                let opener = start + value_pos;
                if let Some(found) = scan_expression_at(text, opener) {
                    let line_end = start + line.len();
                    let end = if found.end >= line_end {
                        found.end
                    } else {
                        let tail = &text[found.end..line_end];
                        found.end + tail[..comment_start(tail)].trim_end().len()
                    };
                    hits.push(Hit { line: index + 1, span: opener..end });
                }
            }
            _ => {}
        }
    }
    hits
}

/// The position after `- ` (and its trailing spaces) when a block-list item
/// starts at `pos`.
fn list_item_content(line: &str, pos: usize) -> Option<usize> {
    let rest = line.get(pos..)?.strip_prefix('-')?;
    if rest.is_empty() {
        return Some(line.len());
    }
    if !rest.starts_with([' ', '\t']) {
        return None;
    }
    Some(line.len() - rest.trim_start_matches([' ', '\t']).len())
}

/// The position of the value of a `key: value` entry starting at `pos`.
fn mapping_value(line: &str, pos: usize) -> Option<usize> {
    let rest = &line[pos..];
    let bytes = rest.as_bytes();
    let key_end = match bytes.first()? {
        &quote @ (b'"' | b'\'') => {
            let close = closing_quote(rest, 1, quote)?;
            let after = &rest[close + 1..];
            let colon = close + 1 + (after.len() - after.trim_start_matches([' ', '\t']).len());
            (bytes.get(colon) == Some(&b':')).then_some(colon)?
        }
        b'#' | b'{' | b'[' | b'?' | b'&' | b'*' | b'!' | b'|' | b'>' => return None,
        _ => (0..bytes.len()).find_map(|at| match bytes[at] {
            b'#' if at > 0 && matches!(bytes[at - 1], b' ' | b'\t') => Some(None),
            b':' if matches!(bytes.get(at + 1), None | Some(b' ' | b'\t')) => Some(Some(at)),
            _ => None,
        })??,
    };
    if !matches!(bytes.get(key_end + 1), None | Some(b' ' | b'\t')) {
        return None;
    }
    let after = &rest[key_end + 1..];
    Some(pos + key_end + 1 + (after.len() - after.trim_start_matches([' ', '\t']).len()))
}

/// The index in `text` of the quote closing a scalar opened by `quote`,
/// searching from `from`.
fn closing_quote(text: &str, from: usize, quote: u8) -> Option<usize> {
    let bytes = text.as_bytes();
    let mut at = from;
    while at < bytes.len() {
        match bytes[at] {
            b'\\' if quote == b'"' => at += 2,
            b'\'' if quote == b'\'' && bytes.get(at + 1) == Some(&b'\'') => at += 2,
            byte if byte == quote => return Some(at),
            _ => at += 1,
        }
    }
    None
}

/// Where a ` #` comment starts in `tail`, or its length.
fn comment_start(tail: &str) -> usize {
    let bytes = tail.as_bytes();
    (0..bytes.len())
        .find(|&at| bytes[at] == b'#' && at > 0 && matches!(bytes[at - 1], b' ' | b'\t'))
        .unwrap_or(bytes.len())
}
