//! Decoded-YAML-scalar-to-authored-byte projection.
//!
//! A single YAML scalar can be authored plain, single-quoted, or double-quoted
//! (with escapes), and as a literal (`|`) or folded (`>`) block scalar. Any
//! consumer that parses the *decoded* value (a schema suggestion span, a
//! composition error anchor, or a language server parsing an Expression-typed
//! value) and then needs to point back at the *authored* source must project
//! decoded byte offsets through the quoting, escaping, folding, and
//! indentation. [`DecodedScalar`] carries that byte-level map so the projection
//! is exact rather than approximate.
//!
//! This module owns only the decoding + mapping; the schema-source scanner
//! ([`super::source`]), frontmatter composition, and DMLS all build on it.
//! [`decode_scalar_node`] is the multi-line decoder both composition and DMLS
//! use for frontmatter values; its folding, chomping, and indentation rules
//! follow the libyaml scanner behind `serde_yaml_ng`, the frontmatter parser.

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
    /// One raw byte offset per decoded byte boundary (length `decoded.len()+1`):
    /// entry 0 is where the first decoded byte is authored, and entry `i` is
    /// where the source of decoded byte `i - 1` ends.
    decoded_to_raw: Vec<usize>,
    /// Where the source of decoded byte `i` starts (length `decoded.len()`).
    /// It differs from `decoded_to_raw[i]` only where the scalar skips authored
    /// bytes between two decoded bytes (indentation, trimmed whitespace), so a
    /// projected range starts at its first authored byte rather than in the
    /// skipped run.
    decoded_starts: Vec<usize>,
}

impl DecodedScalar {
    /// A scalar whose decoded bytes come from one uninterrupted authored run,
    /// so each decoded byte starts where the previous one's source ends.
    fn contiguous(decoded: String, decoded_to_raw: Vec<usize>) -> Self {
        let decoded_starts = decoded_to_raw[..decoded.len()].to_vec();
        Self {
            decoded,
            decoded_to_raw,
            decoded_starts,
        }
    }

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
    ///
    /// A non-empty range starts at the authored start of its first decoded byte
    /// and ends where its last decoded byte's source ends, so a range inside a
    /// multi-line scalar excludes the indentation before it. An empty range
    /// projects to an empty range at its authored position.
    pub fn project(&self, range: Range<usize>) -> Option<Range<usize>> {
        let end = self.raw_offset(range.end)?;
        if range.is_empty() {
            let start = self.raw_start(range.start)?;
            return Some(start..start);
        }
        Some(self.raw_start(range.start)?..end)
    }

    /// The authored offset where decoded byte `decoded_offset` starts; the
    /// scalar's authored end for the boundary past the last byte.
    fn raw_start(&self, decoded_offset: usize) -> Option<usize> {
        match self.decoded_starts.get(decoded_offset) {
            Some(start) => Some(*start),
            None => self.raw_offset(decoded_offset),
        }
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

/// Decodes the scalar node whose authored text starts at byte `start` of
/// `source`, following it across lines, and reports where its authored text
/// ends.
///
/// Covers plain, single-quoted, and double-quoted scalars on one line or
/// several (line folding, escaped line breaks), and literal (`|`) and folded
/// (`>`) block scalars with chomping (`-`, `+`, clip) and explicit indentation
/// indicators. `\r\n` and `\n` line breaks both decode to `\n`. Raw offsets
/// in the result are offsets into `source`.
///
/// `parent_indent` is the column of the block collection holding the value:
/// the column of its mapping key, or of the `-` of its sequence entry. It
/// fixes the content indentation of a block scalar with an explicit
/// indentation indicator and the least indentation a plain scalar's
/// continuation line needs.
///
/// A tag (`!!str`, `!<tag:yaml.org,2002:str>`) or anchor (`&name`) before the
/// scalar, in either order, is skipped, so projected ranges never include it.
/// An alias (`*name`) decodes the scalar that defines its anchor, and the map
/// points at that defining scalar, the only place the text is authored. That
/// happens only when `&name` occurs exactly once in the text before the alias:
/// a second occurrence may be a redefinition or may sit in a comment or a
/// string, and telling those apart takes a full YAML parse.
///
/// ## Returns
///
/// The decoded scalar and the offset just past its last authored content byte
/// (for a quoted scalar, past the closing quote; for an alias, past the alias
/// token). `None` when `start` does not begin a scalar this decoder models (a
/// flow collection, an alias whose anchor is not provably unique, a tag or
/// anchor whose scalar starts on a later line), or when the text is not a
/// well-formed scalar there (an unterminated quote, a malformed block header,
/// a tab used as indentation).
///
/// ## Notes
///
/// This is block-context decoding, and it never interprets a tag. A caller
/// must still compare the decoded text with the value its YAML parser produced
/// before trusting the projection: any disagreement (a flow-context value, a
/// tag that changes the value such as `!!int`, a document the parser only
/// accepted after normalizing it) means the map describes other text.
pub fn decode_scalar_node(
    source: &str,
    start: usize,
    parent_indent: usize,
) -> Option<(DecodedScalar, usize)> {
    let bytes = source.as_bytes();
    if bytes.get(start) == Some(&b'*') {
        return decode_alias_target(source, start);
    }
    let start = skip_node_properties(bytes, start)?;
    match *bytes.get(start)? {
        b'|' | b'>' => decode_block_scalar(source, start, parent_indent),
        b'\'' | b'"' => decode_flow_quoted(source, start),
        b'[' | b'{' | b'&' | b'*' | b'!' | b'%' | b'@' | b'`' | b'#' => None,
        b'-' | b'?' | b':' if is_blankz(bytes, start + 1) => None,
        _ => decode_multiline_plain(source, start, parent_indent),
    }
}

/// The offset of a node's content after its tag and anchor, which may come in
/// either order. `None` when the content does not follow on the same line.
fn skip_node_properties(bytes: &[u8], start: usize) -> Option<usize> {
    let mut pos = start;
    let (mut tag, mut anchor) = (false, false);
    loop {
        match bytes.get(pos) {
            Some(b'!') if !tag => tag = true,
            Some(b'&') if !anchor => anchor = true,
            _ => return Some(pos),
        }
        while !is_blankz(bytes, pos) {
            pos += 1;
        }
        while is_blank(bytes, pos) {
            pos += 1;
        }
        if pos >= bytes.len() || is_break(bytes, pos) || bytes[pos] == b'#' {
            return None;
        }
    }
}

/// The authored range of the alias token (`*name`) starting at `start`, without
/// anything after the name. `None` when `start` does not begin an alias.
pub(crate) fn alias_token(source: &str, start: usize) -> Option<Range<usize>> {
    let bytes = source.as_bytes();
    if bytes.get(start) != Some(&b'*') {
        return None;
    }
    let mut end = start + 1;
    while !is_blankz(bytes, end) && !matches!(bytes[end], b',' | b'[' | b']' | b'{' | b'}') {
        end += 1;
    }
    (end > start + 1).then_some(start..end)
}

/// Decodes the scalar defining the anchor of the alias at `start`, reporting
/// the end of the alias token.
fn decode_alias_target(source: &str, start: usize) -> Option<(DecodedScalar, usize)> {
    let bytes = source.as_bytes();
    let end = alias_token(source, start)?.end;
    let anchor = format!("&{}", &source[start + 1..end]);
    // An occurrence continuing into a longer name is another anchor. Every
    // other one counts, even in a comment or a string: the parser resolved the
    // alias, so a sole occurrence is the definition.
    let continues_name = |at: usize| {
        bytes
            .get(at)
            .is_some_and(|next| next.is_ascii_alphanumeric() || matches!(next, b'_' | b'-'))
    };
    note_alias_search(start);
    let mut occurrences = source[..start]
        .match_indices(&anchor)
        .filter(|(at, _)| !continues_name(at + anchor.len()));
    let (defined_at, _) = occurrences.next()?;
    if occurrences.next().is_some() {
        return None;
    }
    Some((decode_alias_definition(source, defined_at)?, end))
}

/// Decodes the scalar an alias resolves to, for a caller whose YAML parser
/// already knows the node that defines the anchor.
///
/// `definition_start` is where that node's tag, anchor, or scalar text starts.
/// The parent column [`decode_scalar_node`] needs is read from the node's own
/// line, so the work is bounded by that line and the scalar: nothing before
/// the line is read, however many aliases a document holds.
///
/// ## Returns
///
/// `None` wherever [`decode_scalar_node`] returns `None`, and when
/// `definition_start` begins an alias, which defines nothing.
///
/// ## Notes
///
/// The comparison [`decode_scalar_node`] asks of its caller applies here too:
/// keep the projection only when the decoded text equals the parser's value
/// for the alias.
pub fn decode_alias_definition(source: &str, definition_start: usize) -> Option<DecodedScalar> {
    if source.as_bytes().get(definition_start) == Some(&b'*') {
        return None;
    }
    let parent_indent = column(source, holder_start(source, definition_start));
    decode_scalar_node(source, definition_start, parent_indent).map(|(scalar, _)| scalar)
}

/// Where the mapping key or sequence `-` holding a node starts, read from the
/// node's own line. `at` is anywhere from the node's first tag or anchor to the
/// start of its scalar text.
fn holder_start(source: &str, at: usize) -> usize {
    let bytes = source.as_bytes();
    let mut pos = source[..at].rfind(['\n', '\r']).map_or(0, |index| index + 1);
    while pos < at && bytes[pos] == b' ' {
        pos += 1;
    }
    let mut holder = pos;
    while pos < at && bytes[pos] == b'-' && is_blank(bytes, pos + 1) {
        holder = pos;
        pos += 1;
        while pos < at && is_blank(bytes, pos) {
            pos += 1;
        }
    }
    // Tags and anchors running up to `at` are the node's own; anything else
    // after the last `-` is its mapping key, which starts at its first property.
    let key = pos;
    while pos < at && matches!(bytes[pos], b'!' | b'&') {
        while pos < at && !is_blank(bytes, pos) {
            pos += 1;
        }
        while pos < at && is_blank(bytes, pos) {
            pos += 1;
        }
    }
    if pos < at {
        holder = key;
    }
    holder
}

#[cfg(any(test, feature = "work-counters"))]
thread_local! {
    static ALIAS_SEARCH_WORK: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

#[inline]
#[cfg_attr(not(any(test, feature = "work-counters")), allow(unused_variables))]
fn note_alias_search(bytes: usize) {
    #[cfg(any(test, feature = "work-counters"))]
    ALIAS_SEARCH_WORK.with(|work| work.set(work.get() + bytes));
}

/// Source bytes this thread has searched for the anchors of aliases, which
/// only [`decode_scalar_node`] on an alias token does. Complexity regressions
/// read a before/after delta: a consumer that resolves aliases through its own
/// YAML parser and [`decode_alias_definition`] must add nothing.
#[cfg(any(test, feature = "work-counters"))]
pub fn alias_search_work() -> usize {
    ALIAS_SEARCH_WORK.with(std::cell::Cell::get)
}

/// Accumulates a decoded scalar together with its authored byte map.
struct ScalarBuilder {
    decoded: String,
    decoded_to_raw: Vec<usize>,
    decoded_starts: Vec<usize>,
}

impl ScalarBuilder {
    fn new(raw_start: usize) -> Self {
        Self {
            decoded: String::new(),
            decoded_to_raw: vec![raw_start],
            decoded_starts: Vec::new(),
        }
    }

    /// Appends `value`, produced by the authored bytes `raw`.
    fn push(&mut self, value: &str, raw: Range<usize>) {
        if self.decoded.is_empty() {
            self.decoded_to_raw[0] = raw.start;
        }
        let first = self.decoded.len();
        self.decoded.push_str(value);
        for index in first..self.decoded.len() {
            self.decoded_starts
                .push(if index == first { raw.start } else { raw.end });
            self.decoded_to_raw.push(raw.end);
        }
    }

    fn push_char(&mut self, source: &str, at: usize) -> usize {
        let ch = source[at..].chars().next().unwrap_or('\0');
        let end = at + ch.len_utf8();
        self.push(&source[at..end], at..end);
        end
    }

    fn finish(self) -> DecodedScalar {
        DecodedScalar {
            decoded: self.decoded,
            decoded_to_raw: self.decoded_to_raw,
            decoded_starts: self.decoded_starts,
        }
    }
}

/// Line breaks and blanks pending between two runs of a multi-line flow or
/// plain scalar, joined by YAML's line-folding rule.
#[derive(Default)]
struct FoldState {
    /// A line break has been seen since the last content.
    leading_blanks: bool,
    /// Blanks after the last content on its line (kept only if no break follows).
    whitespaces: Vec<Range<usize>>,
    /// The first line break after the last content.
    leading_break: Option<Range<usize>>,
    /// Each further (empty-line) break.
    trailing_breaks: Vec<Range<usize>>,
}

impl FoldState {
    fn has_pending(&self) -> bool {
        self.leading_blanks || !self.whitespaces.is_empty()
    }

    /// Records a blank at `at`. Blanks after a line break are indentation.
    fn blank(&mut self, at: usize) {
        if !self.leading_blanks {
            self.whitespaces.push(at..at + 1);
        }
    }

    /// Records a line break spanning `raw`.
    fn line_break(&mut self, raw: Range<usize>) {
        if self.leading_blanks {
            self.trailing_breaks.push(raw);
        } else {
            self.whitespaces.clear();
            self.leading_break = Some(raw);
            self.leading_blanks = true;
        }
    }

    /// Emits the pending separator: one break folds to a space, each further
    /// break is a `\n`, and blanks survive only within a line.
    fn flush(&mut self, out: &mut ScalarBuilder, source: &str) {
        if self.leading_blanks {
            match self.leading_break.take() {
                Some(first) if self.trailing_breaks.is_empty() => out.push(" ", first),
                _ => {
                    for raw in self.trailing_breaks.drain(..) {
                        out.push("\n", raw);
                    }
                }
            }
            self.trailing_breaks.clear();
            self.leading_blanks = false;
        } else {
            for raw in self.whitespaces.drain(..) {
                out.push(&source[raw.clone()], raw);
            }
        }
    }
}

fn is_break(bytes: &[u8], at: usize) -> bool {
    matches!(bytes.get(at), Some(b'\n' | b'\r'))
}

fn is_blank(bytes: &[u8], at: usize) -> bool {
    matches!(bytes.get(at), Some(b' ' | b'\t'))
}

fn is_blankz(bytes: &[u8], at: usize) -> bool {
    at >= bytes.len() || is_blank(bytes, at) || is_break(bytes, at)
}

/// The authored bytes of the line break at `at` (`\r\n` counts as one).
fn break_at(bytes: &[u8], at: usize) -> Range<usize> {
    if bytes.get(at) == Some(&b'\r') && bytes.get(at + 1) == Some(&b'\n') {
        at..at + 2
    } else {
        at..at + 1
    }
}

/// The character column of `at` on its line.
fn column(source: &str, at: usize) -> usize {
    let line_start = source[..at]
        .rfind(['\n', '\r'])
        .map_or(0, |index| index + 1);
    source[line_start..at].chars().count()
}

/// A `---` or `...` document marker at the start of the line at `at`.
fn at_document_marker(source: &str, at: usize) -> bool {
    let bytes = source.as_bytes();
    column(source, at) == 0
        && (source[at..].starts_with("---") || source[at..].starts_with("..."))
        && is_blankz(bytes, at + 3)
}

fn decode_flow_quoted(source: &str, start: usize) -> Option<(DecodedScalar, usize)> {
    let bytes = source.as_bytes();
    let quote = bytes[start];
    let single = quote == b'\'';
    let mut out = ScalarBuilder::new(start + 1);
    let mut fold = FoldState::default();
    let mut pos = start + 1;
    loop {
        if at_document_marker(source, pos) || pos >= bytes.len() {
            return None;
        }
        fold.leading_blanks = false;
        while !is_blankz(bytes, pos) {
            if single && source[pos..].starts_with("''") {
                out.push("'", pos..pos + 2);
                pos += 2;
            } else if bytes[pos] == quote {
                break;
            } else if !single && bytes[pos] == b'\\' && is_break(bytes, pos + 1) {
                // An escaped line break joins the lines with nothing between.
                pos = break_at(bytes, pos + 1).end;
                fold.leading_blanks = true;
                break;
            } else if !single && bytes[pos] == b'\\' {
                let (value, consumed) = decode_yaml_escape(&source[pos..])?;
                out.push(&value, pos..pos + consumed);
                pos += consumed;
            } else {
                pos = out.push_char(source, pos);
            }
        }
        if pos >= bytes.len() {
            return None;
        }
        if bytes[pos] == quote {
            return Some((out.finish(), pos + 1));
        }
        while is_blank(bytes, pos) || is_break(bytes, pos) {
            if is_blank(bytes, pos) {
                fold.blank(pos);
                pos += 1;
            } else {
                let raw = break_at(bytes, pos);
                pos = raw.end;
                fold.line_break(raw);
            }
        }
        fold.flush(&mut out, source);
    }
}

fn decode_multiline_plain(
    source: &str,
    start: usize,
    parent_indent: usize,
) -> Option<(DecodedScalar, usize)> {
    let bytes = source.as_bytes();
    let indent = parent_indent + 1;
    let mut out = ScalarBuilder::new(start);
    let mut fold = FoldState::default();
    let mut pos = start;
    let mut end = start;
    loop {
        if at_document_marker(source, pos) || bytes.get(pos) == Some(&b'#') {
            break;
        }
        while !is_blankz(bytes, pos) {
            if bytes[pos] == b':' && is_blankz(bytes, pos + 1) {
                break;
            }
            if fold.has_pending() {
                fold.flush(&mut out, source);
            }
            pos = out.push_char(source, pos);
            end = pos;
        }
        if !(is_blank(bytes, pos) || is_break(bytes, pos)) {
            break;
        }
        while is_blank(bytes, pos) || is_break(bytes, pos) {
            if is_blank(bytes, pos) {
                if fold.leading_blanks && bytes[pos] == b'\t' && column(source, pos) < indent {
                    return None;
                }
                fold.blank(pos);
                pos += 1;
            } else {
                let raw = break_at(bytes, pos);
                pos = raw.end;
                fold.line_break(raw);
            }
        }
        if column(source, pos) < indent {
            break;
        }
    }
    (end > start).then(|| (out.finish(), end))
}

/// Chomping indicator of a block scalar header.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Chomping {
    Strip,
    Clip,
    Keep,
}

fn decode_block_scalar(
    source: &str,
    start: usize,
    parent_indent: usize,
) -> Option<(DecodedScalar, usize)> {
    let bytes = source.as_bytes();
    let folded = bytes[start] == b'>';
    let mut pos = start + 1;
    let mut chomping = None;
    let mut increment = None;
    for _ in 0..2 {
        match bytes.get(pos) {
            Some(b'+') if chomping.is_none() => chomping = Some(Chomping::Keep),
            Some(b'-') if chomping.is_none() => chomping = Some(Chomping::Strip),
            Some(digit @ b'1'..=b'9') if increment.is_none() => {
                increment = Some(usize::from(digit - b'0'));
            }
            _ => break,
        }
        pos += 1;
    }
    let chomping = chomping.unwrap_or(Chomping::Clip);
    while is_blank(bytes, pos) {
        pos += 1;
    }
    if bytes.get(pos) == Some(&b'#') {
        while pos < bytes.len() && !is_break(bytes, pos) {
            pos += 1;
        }
    }
    let header_end = pos;
    if pos >= bytes.len() {
        return Some((ScalarBuilder::new(header_end).finish(), header_end));
    }
    if !is_break(bytes, pos) {
        return None;
    }
    pos = break_at(bytes, pos).end;

    let mut indent = increment.map_or(0, |increment| parent_indent + increment);
    let mut out = ScalarBuilder::new(pos);
    let mut end = header_end;
    let mut trailing_breaks = block_scalar_breaks(source, &mut pos, &mut indent, parent_indent)?;
    let mut leading_break: Option<Range<usize>> = None;
    let mut leading_blank = false;
    while pos < bytes.len() && column(source, pos) == indent {
        let trailing_blank = is_blank(bytes, pos);
        match leading_break.take() {
            // A break between two lines that do not start with a blank folds
            // to a space, or disappears when empty lines follow it.
            Some(raw) if folded && !leading_blank && !trailing_blank => {
                if trailing_breaks.is_empty() {
                    out.push(" ", raw);
                }
            }
            Some(raw) => out.push("\n", raw),
            None => {}
        }
        for raw in trailing_breaks.drain(..) {
            out.push("\n", raw);
        }
        leading_blank = trailing_blank;
        while pos < bytes.len() && !is_break(bytes, pos) {
            pos = out.push_char(source, pos);
        }
        end = pos;
        if pos >= bytes.len() {
            break;
        }
        let raw = break_at(bytes, pos);
        pos = raw.end;
        leading_break = Some(raw);
        trailing_breaks = block_scalar_breaks(source, &mut pos, &mut indent, parent_indent)?;
    }
    if chomping != Chomping::Strip
        && let Some(raw) = leading_break
    {
        out.push("\n", raw);
    }
    if chomping == Chomping::Keep {
        for raw in trailing_breaks {
            out.push("\n", raw);
        }
    }
    Some((out.finish(), end))
}

/// Consumes the indentation and empty lines before the next block-scalar
/// content line, returning the empty lines' breaks. Fixes an undetected
/// `indent` from the deepest of those lines, as libyaml does.
fn block_scalar_breaks(
    source: &str,
    pos: &mut usize,
    indent: &mut usize,
    parent_indent: usize,
) -> Option<Vec<Range<usize>>> {
    let bytes = source.as_bytes();
    let mut breaks = Vec::new();
    let mut max_indent = 0;
    loop {
        let mut col = column(source, *pos);
        while (*indent == 0 || col < *indent) && bytes.get(*pos) == Some(&b' ') {
            *pos += 1;
            col += 1;
        }
        max_indent = max_indent.max(col);
        if (*indent == 0 || col < *indent) && bytes.get(*pos) == Some(&b'\t') {
            return None;
        }
        if !is_break(bytes, *pos) {
            break;
        }
        let raw = break_at(bytes, *pos);
        *pos = raw.end;
        breaks.push(raw);
    }
    if *indent == 0 {
        *indent = max_indent.max(parent_indent + 1);
    }
    Some(breaks)
}

fn plain_scalar(content: &str, base: usize) -> DecodedScalar {
    let mut map = Vec::with_capacity(content.len() + 1);
    map.extend(base..=base + content.len());
    DecodedScalar::contiguous(content.to_string(), map)
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
            return Some((DecodedScalar::contiguous(decoded, map), cursor + 1));
        } else {
            let ch = raw[cursor..].chars().next()?;
            cursor += ch.len_utf8();
            push_mapped(&mut decoded, &mut map, &ch.to_string(), base + cursor);
        }
    }
    tolerant.then_some((DecodedScalar::contiguous(decoded, map), cursor))
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
                return Some((DecodedScalar::contiguous(decoded, map), cursor + 1));
            }
            b'\\' => {
                // A half-typed escape (`"\` or `"\u00`) has no value yet; the
                // tolerant caller keeps what decoded cleanly before it.
                let Some((value, consumed)) = decode_yaml_escape(&raw[cursor..]) else {
                    return tolerant
                        .then_some((DecodedScalar::contiguous(decoded, map), cursor));
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
    tolerant.then_some((DecodedScalar::contiguous(decoded, map), cursor))
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

    /// Decodes the value of `key` in `yaml` with [`decode_scalar_node`] and
    /// asserts it equals what the frontmatter parser (`serde_yaml_ng`) reads
    /// for the same key, then returns it.
    fn node_matching_parser(yaml: &str, path: &[&str]) -> (DecodedScalar, usize) {
        let mut parsed: serde_yaml_ng::Value =
            serde_yaml_ng::from_str(yaml).expect("fixture is valid YAML");
        for key in path {
            parsed = parsed.get(*key).expect("fixture has the key").clone();
        }
        let expected = parsed.as_str().expect("fixture value is a string").to_string();

        let leaf = path.last().expect("a key path");
        let needle = format!("{leaf}:");
        let key_start = yaml.find(&needle).expect("key in fixture");
        let mut start = key_start + needle.len();
        while yaml.as_bytes()[start] == b' ' {
            start += 1;
        }
        let parent_indent = column(yaml, key_start);
        let (scalar, end) =
            decode_scalar_node(yaml, start, parent_indent).expect("value decodes");
        assert_eq!(scalar.decoded(), expected, "decoded text must match the parser for {yaml:?}");
        (scalar, end)
    }

    /// Projects the first occurrence of `needle` in the decoded text.
    fn projected<'a>(yaml: &'a str, scalar: &DecodedScalar, needle: &str) -> &'a str {
        let at = scalar.decoded().find(needle).expect("needle decoded");
        let range = scalar.project(at..at + needle.len()).expect("projects");
        &yaml[range]
    }

    #[test]
    fn literal_block_chomping_matches_the_parser() {
        for header in ["|", "|-", "|+"] {
            let yaml = format!("a: {header}\n  one {{{{ x }}}}\n    two\n\n\nb: 1\n");
            let (scalar, end) = node_matching_parser(&yaml, &["a"]);
            assert_eq!(projected(&yaml, &scalar, "{{ x }}"), "{{ x }}");
            assert_eq!(projected(&yaml, &scalar, "two"), "two");
            // The scalar's authored text ends after its last content line.
            assert_eq!(&yaml[..end], format!("a: {header}\n  one {{{{ x }}}}\n    two"));
        }
    }

    #[test]
    fn folded_block_chomping_and_folding_match_the_parser() {
        for header in [">", ">-", ">+"] {
            let yaml = format!(
                "a: {header}\n  one\n  two {{{{ y }}}}\n\n  three\n    indented\n  four\n\nb: 1\n"
            );
            let (scalar, _) = node_matching_parser(&yaml, &["a"]);
            assert_eq!(projected(&yaml, &scalar, "{{ y }}"), "{{ y }}");
            assert_eq!(projected(&yaml, &scalar, "four"), "four");
        }
    }

    #[test]
    fn a_folded_expression_across_lines_projects_from_open_to_close() {
        let yaml = "a: >\n  {{ first -\n  second }}\n";
        let (scalar, _) = node_matching_parser(yaml, &["a"]);
        assert_eq!(scalar.decoded(), "{{ first - second }}\n");
        assert_eq!(projected(yaml, &scalar, "{{ first - second }}"), "{{ first -\n  second }}");
    }

    #[test]
    fn explicit_indentation_indicators_match_the_parser() {
        let yaml = "a: |2\n    two leading {{ x }}\n  none\nb: 1\n";
        let (scalar, _) = node_matching_parser(yaml, &["a"]);
        assert_eq!(scalar.decoded(), "  two leading {{ x }}\nnone\n");
        assert_eq!(projected(yaml, &scalar, "{{ x }}"), "{{ x }}");

        // The indicator counts from the nested mapping's own column.
        let nested = "outer:\n  a: >1-\n     three {{ z }}\n   one\n";
        let (scalar, _) = node_matching_parser(nested, &["outer", "a"]);
        assert_eq!(projected(nested, &scalar, "{{ z }}"), "{{ z }}");
    }

    #[test]
    fn crlf_block_scalars_decode_like_lf_and_project_exactly() {
        for header in ["|", ">", "|-", ">+"] {
            let yaml = format!("a: {header}\r\n  one\r\n  two {{{{ x }}}}\r\n\r\n  three\r\nb: 1\r\n");
            let (scalar, _) = node_matching_parser(&yaml, &["a"]);
            assert!(!scalar.decoded().contains('\r'), "{header}: {:?}", scalar.decoded());
            assert_eq!(projected(&yaml, &scalar, "{{ x }}"), "{{ x }}");
            assert_eq!(projected(&yaml, &scalar, "three"), "three");
        }
    }

    #[test]
    fn multibyte_text_before_the_expression_projects_by_byte() {
        let yaml = "a: >\n  café — naïve\n  {{ x }}\n";
        let (scalar, _) = node_matching_parser(yaml, &["a"]);
        let at = scalar.decoded().find("{{").expect("decoded");
        let range = scalar.project(at..at + "{{ x }}".len()).expect("projects");
        assert_eq!(range.start, yaml.find("{{").expect("authored"));
        assert_eq!(&yaml[range], "{{ x }}");
    }

    #[test]
    fn multi_line_double_quoted_scalars_fold_and_unescape_like_the_parser() {
        let yaml = "a: \"one\\t{{ x }}\n  two \\\n  three\n\n  caf\\u00e9 {{ y }}\"\nb: 1\n";
        let (scalar, end) = node_matching_parser(yaml, &["a"]);
        assert_eq!(projected(yaml, &scalar, "{{ x }}"), "{{ x }}");
        assert_eq!(projected(yaml, &scalar, "{{ y }}"), "{{ y }}");
        assert_eq!(&yaml[end - 1..end], "\"");

        let crlf = yaml.replace('\n', "\r\n");
        let (scalar, _) = node_matching_parser(&crlf, &["a"]);
        assert_eq!(projected(&crlf, &scalar, "{{ y }}"), "{{ y }}");
    }

    #[test]
    fn multi_line_single_quoted_and_plain_scalars_fold_like_the_parser() {
        let single = "a: 'it''s\n  {{ x }}\n\n  end'\nb: 1\n";
        let (scalar, _) = node_matching_parser(single, &["a"]);
        assert_eq!(projected(single, &scalar, "{{ x }}"), "{{ x }}");

        let plain = "a: one\n  two {{ x }}\n\n  three # trailing comment\nb: 1\n";
        let (scalar, end) = node_matching_parser(plain, &["a"]);
        assert_eq!(projected(plain, &scalar, "{{ x }}"), "{{ x }}");
        assert_eq!(&plain[..end], "a: one\n  two {{ x }}\n\n  three");

        let crlf = plain.replace('\n', "\r\n");
        let (scalar, _) = node_matching_parser(&crlf, &["a"]);
        assert_eq!(projected(&crlf, &scalar, "{{ x }}"), "{{ x }}");
    }

    #[test]
    fn nodes_the_decoder_does_not_model_yield_none() {
        assert!(decode_scalar_node("a: [x]\n", 3, 0).is_none());
        assert!(decode_scalar_node("a: *undefined\n", 3, 0).is_none());
        assert!(decode_scalar_node("a: !!seq [x]\n", 3, 0).is_none());
        assert!(decode_scalar_node("a: !!str !!str x\n", 3, 0).is_none());
        assert!(decode_scalar_node("a: !!str\n  x\n", 3, 0).is_none());
        assert!(decode_scalar_node("a: &anchor # comment\n  x\n", 3, 0).is_none());
        assert!(decode_scalar_node("a: \"unterminated\n", 3, 0).is_none());
        assert!(decode_scalar_node("a: |0\n  x\n", 3, 0).is_none());
        assert!(decode_scalar_node("a: | trailing\n  x\n", 3, 0).is_none());
    }

    #[test]
    fn a_tag_stays_outside_the_projection_of_the_scalar_behind_it() {
        for (yaml, expression) in [
            ("a: !!str \"café — {{ x }}\"\nb: 1\n", "{{ x }}"),
            ("a: !!str café — {{ x }} # comment\nb: 1\n", "{{ x }}"),
            ("a: !<tag:yaml.org,2002:str> 'it''s {{ x }}'\n", "{{ x }}"),
            ("a: !!str |\n  one\n  café {{ x }}\nb: 1\n", "{{ x }}"),
            ("a: !!str >-\n  {{ first -\n  second }}\n", "{{ first - second }}"),
            ("a: !!str \"one\n  two {{ x }}\"\n", "{{ x }}"),
            ("a: !!binary aGk= {{ x }}\n", "{{ x }}"),
        ] {
            for yaml in [yaml.to_string(), yaml.replace('\n', "\r\n")] {
                let (scalar, _) = node_matching_parser(&yaml, &["a"]);
                let at = scalar.decoded().find("{{").expect("decoded");
                let range = scalar.project(at..at + expression.len()).expect("projects");
                assert_eq!(range.start, yaml.find("{{").expect("authored"), "{yaml:?}");
                assert_eq!(range.end, yaml.find("}}").expect("authored") + 2, "{yaml:?}");
            }
        }
    }

    #[test]
    fn an_anchor_and_a_tag_in_either_order_stay_outside_the_projection() {
        for yaml in [
            "a: &name \"x {{ y }}\"\n",
            "a: &name !!str x {{ y }}\n",
            "a: !!str &name x {{ y }}\n",
            "outer:\n  a: &name |2\n      {{ y }}\n",
        ] {
            let path: &[&str] = if yaml.starts_with("outer") { &["outer", "a"] } else { &["a"] };
            let (scalar, _) = node_matching_parser(yaml, path);
            let at = scalar.decoded().find("{{").expect("decoded");
            let range = scalar.project(at..at + "{{ y }}".len()).expect("projects");
            assert_eq!(range.start, yaml.find("{{").expect("authored"), "{yaml:?}");
            assert_eq!(&yaml[range], "{{ y }}");
        }
    }

    /// The decoder never interprets a tag; the caller's comparison with its
    /// parser's value is what rejects a tag that changes the value.
    #[test]
    fn a_tag_that_changes_the_value_disagrees_with_the_parser() {
        for yaml in ["a: !!int \"5\"\n", "a: !!float \"1.5\"\n", "a: !!bool \"true\"\n", "a: !custom \"5\"\n"] {
            let (scalar, _) = decode_scalar_node(yaml, 3, 0).expect("the scalar behind the tag");
            assert_eq!(scalar.decoded(), yaml.split('"').nth(1).expect("quoted"));
            // The frontmatter parser rejects a local tag outright.
            let parsed = serde_yaml_ng::from_str::<crate::markdown::FrontmatterMap>(yaml).ok();
            let value = parsed.as_ref().and_then(|map| map.get("a")).and_then(|value| value.as_str());
            assert_eq!(value, None, "{yaml:?} is not a string to the parser");
        }
    }

    /// Decodes the alias value of `key` and asserts it equals the parser's
    /// resolved value.
    fn alias_matching_parser(yaml: &str, key: &str) -> Option<(DecodedScalar, usize)> {
        let parsed: serde_yaml_ng::Value = serde_yaml_ng::from_str(yaml).expect("valid YAML");
        let expected = parsed.get(key).and_then(|value| value.as_str()).expect("a string").to_string();
        let start = yaml.find(&format!("{key}: *")).expect("alias in fixture") + key.len() + 2;
        let decoded = decode_scalar_node(yaml, start, 0)?;
        assert_eq!(decoded.0.decoded(), expected, "{yaml:?}");
        Some(decoded)
    }

    #[test]
    fn an_alias_projects_into_the_scalar_defining_its_anchor() {
        for yaml in [
            "a: &x \"café {{ bad( }}\"\nb: *x\n",
            "a: &x !!str café {{ bad( }}\nb: *x # comment\n",
            "list:\n  - key: &x |-\n      café {{ bad( }}\n    other: 1\nb: *x\n",
            "list:\n  - &x café\n    {{ bad( }}\nb: *x\n",
            "a: &x-long other\nc: &x \"{{ bad( }}\"\nb: *x\n",
        ] {
            for yaml in [yaml.to_string(), yaml.replace('\n', "\r\n")] {
                let (scalar, end) = alias_matching_parser(&yaml, "b").expect("a unique anchor resolves");
                let at = scalar.decoded().find("{{").expect("decoded");
                let range = scalar.project(at..at + "{{ bad( }}".len()).expect("projects");
                assert_eq!(range.start, yaml.find("{{").expect("authored"), "{yaml:?}");
                assert_eq!(&yaml[range], "{{ bad( }}");
                assert_eq!(&yaml[..end], &yaml[..yaml.find("*x").expect("alias") + 2]);
            }
        }
    }

    /// The last definition before the alias wins in YAML. Picking between two
    /// occurrences takes a full parse, so neither is projected.
    #[test]
    fn an_alias_whose_anchor_is_not_provably_unique_yields_none() {
        for yaml in [
            "a: &x \"{{ one( }}\"\nc: &x \"{{ one( }}\"\nb: *x\n",
            "a: &x \"{{ one( }}\"\nc: \"mentions &x here\"\nb: *x\n",
            "a: &x \"{{ one( }}\" # not &x again\nb: *x\n",
        ] {
            assert!(alias_matching_parser(yaml, "b").is_none(), "{yaml:?}");
        }
        // A definition after the alias is not what the alias refers to.
        let later = "a: &x one\nb: *x\nc: &x two\n";
        let (scalar, _) = alias_matching_parser(later, "b").expect("one definition precedes it");
        assert_eq!(projected(later, &scalar, "one"), "one");
    }

    /// The alias value of `key` as the parser resolves it.
    fn parsed_alias(yaml: &str, key: &str) -> String {
        let parsed: serde_yaml_ng::Value = serde_yaml_ng::from_str(yaml).expect("valid YAML");
        parsed.get(key).and_then(|value| value.as_str()).expect("a string").to_string()
    }

    /// A caller that knows the defining node gets the map the text search
    /// finds, from the node's first property or from its scalar text alike.
    #[test]
    fn a_known_definition_decodes_like_the_searched_one() {
        for (yaml, node, text) in [
            ("a: &x \"café {{ bad( }}\"\nb: *x\n", "&x", "\"café"),
            ("a: !!str &x café {{ bad( }}\nb: *x # comment\n", "!!str", "café"),
            ("list:\n  - key: &x |-\n      café {{ bad( }}\n    other: 1\nb: *x\n", "&x", "|-"),
            ("list:\n  - &x café\n    {{ bad( }}\nb: *x\n", "&x", "café"),
            ("list:\n  - &x !!str |1-\n     café {{ bad( }}\nb: *x\n", "&x", "|1-"),
            ("list:\n  - - !!str &x >2-\n        café\n        {{ bad( }}\nb: *x\n", "!!str", ">2-"),
            ("map:\n  &k key: &x |2-\n      café {{ bad( }}\nb: *x\n", "&x", "|2-"),
        ] {
            for yaml in [yaml.to_string(), yaml.replace('\n', "\r\n")] {
                let (searched, _) = alias_matching_parser(&yaml, "b").expect("a unique anchor resolves");
                for start in [node, text] {
                    let start = yaml.find(start).expect("in fixture");
                    assert_eq!(decode_alias_definition(&yaml, start).as_ref(), Some(&searched), "{yaml:?}");
                }
                assert_eq!(projected(&yaml, &searched, "{{ bad( }}"), "{{ bad( }}");
            }
        }
    }

    /// The parser resolves what the text cannot prove, so a caller holding the
    /// last definition projects a redefined or merely mentioned anchor too.
    #[test]
    fn a_known_definition_needs_no_unique_anchor_name() {
        for (yaml, definition) in [
            ("a: &x \"{{ one( }}\"\nc: &x \"{{ two( }}\"\nb: *x\n", "\"{{ two"),
            ("a: &x \"{{ two( }}\"\nc: \"mentions &x here\"\nb: *x\n", "\"{{ two"),
            ("a: &x \"{{ two( }}\" # not &x again\nb: *x\n", "\"{{ two"),
        ] {
            let start = yaml.find(definition).expect("in fixture");
            let scalar = decode_alias_definition(yaml, start).expect("decodes");
            assert_eq!(scalar.decoded(), parsed_alias(yaml, "b"), "{yaml:?}");
            assert_eq!(projected(yaml, &scalar, "{{ two( }}"), "{{ two( }}");
        }
        let alias = "a: &x one\nb: *x\n";
        assert!(decode_alias_definition(alias, alias.find("*x").expect("alias")).is_none());
    }

    /// `N` distinct anchors, each aliased once. Searching for every anchor
    /// reads the text before its alias, ~N²/2 lines in all; decoding from
    /// known definitions reads none of it.
    #[test]
    fn known_definitions_are_decoded_without_searching_the_document() {
        for count in [200, 400] {
            let mut yaml = String::new();
            for index in 0..count {
                yaml.push_str(&format!("a{index}: &n{index} \"café {{{{ f{index}( }}}}\"\n"));
            }
            let aliases = yaml.len();
            for index in 0..count {
                yaml.push_str(&format!("b{index}: *n{index}\n"));
            }
            let starts = |needle: char, from: usize| -> Vec<usize> {
                yaml[from..].match_indices(needle).map(|(at, _)| from + at).collect()
            };
            let definitions: Vec<usize> = starts('"', 0).into_iter().step_by(2).collect();
            assert_eq!(definitions.len(), count);

            let before = alias_search_work();
            let searched: Vec<DecodedScalar> = starts('*', aliases)
                .into_iter()
                .map(|alias| decode_scalar_node(&yaml, alias, 0).expect("a unique anchor resolves").0)
                .collect();
            let search_work = alias_search_work() - before;
            assert!(search_work >= count * aliases, "{search_work} bytes searched for {count} aliases");

            let before = alias_search_work();
            let known: Vec<DecodedScalar> = definitions
                .iter()
                .map(|start| decode_alias_definition(&yaml, *start).expect("decodes"))
                .collect();
            assert_eq!(alias_search_work() - before, 0, "{count} known definitions");
            assert_eq!(known, searched);
        }
    }
}
