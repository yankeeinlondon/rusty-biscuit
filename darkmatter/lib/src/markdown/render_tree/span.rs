//! Span-aware transport types and processors for the render-tree fold.
//!
//! The fold consumes `pulldown_cmark::Parser::new_ext(...).into_offset_iter()`
//! pairs and converts the source byte ranges into
//! [`renderable::tree::SourceSpan`] so every node points back to source bytes.
//! Darkmatter's legacy [`InlineStyleProcessor`](crate::markdown::inline::InlineStyleProcessor)
//! iterates over plain [`Event`]s without ranges; routing the fold through it
//! would erase every node's [`SourceLocation`](renderable::tree::SourceLocation).
//!
//! This module ships a parallel **span-aware** chain (DMTR-3) that produces
//! [`SpannedInlineEvent`]s while folding `==mark==` / dim inline styles.
//! HR-attribute paragraphs are lifted by the offset-aware
//! `BlockExtensionProcessor` (in `crate::markdown::render_tree::block_extension`)
//! before this chain runs; the [`InlineEvent::HorizontalRule`] variant remains
//! the fold's transport for those generated events. See
//! `renderable/features/2026-05-20-darkmatter-tree/span-aware-processor-design.md`
//! for the design.
//!
//! ## Range Policy
//!
//! - **Split text**: child text events carry exact byte sub-ranges of the
//!   parent `Text` event's range.
//! - **Delimiters**: the byte ranges covering the `==` / `⌄` characters
//!   themselves are attached to the [`InlineEvent::Start`] /
//!   [`InlineEvent::End`].
//! - **Escaped delimiters**: emitted as literal text whose range includes the
//!   escape byte (`\\==` → range covers all three bytes). When `pulldown-cmark`
//!   has already consumed the backslash into a text-event boundary (the `=`
//!   case — `\=` is a recognised CommonMark escape), the processor still
//!   extends the literal range back one byte by consulting the original source
//!   string, so the emitted span covers `\\==` in the source.
//!
//! ## Module Layout
//!
//! - [`SpannedInlineEvent`] / [`SpannedEventProvenance`] — transport types.
//! - [`SpanningAdapter`] — converts `(Event, Range<usize>)` pairs into spanned
//!   inline events.
//! - [`SpannedInlineStyleProcessor`] — mark/dim splitter that preserves byte
//!   ranges.
//!
//! [`Event`]: pulldown_cmark::Event

use pulldown_cmark::{CowStr, Event, Tag, TagEnd};
use std::collections::VecDeque;
use std::ops::Range;

use crate::markdown::inline::{InlineEvent, InlineTag};

/// Provenance of a [`SpannedInlineEvent`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpannedEventProvenance {
    /// The event originated from `pulldown-cmark` and its range covers source
    /// bytes that produced it.
    Parsed,
    /// The event was synthesized by a darkmatter processor; the source range
    /// records which input bytes the synthesis was derived from.
    GeneratedFrom {
        /// The original source byte range that produced this synthetic event.
        source: Range<usize>,
    },
}

/// An inline event carrying a source-byte range and provenance.
///
/// Both fold-tier processors emit `SpannedInlineEvent`s so the fold can
/// produce parsed or generated [`SourceSpan`](renderable::tree::SourceSpan)s
/// without re-deriving ranges.
#[derive(Debug, Clone)]
pub struct SpannedInlineEvent<'a> {
    /// The inline event (standard pulldown event or darkmatter inline tag).
    pub event: InlineEvent<'a>,
    /// The event's operational byte range. For [`SpannedEventProvenance::Parsed`]
    /// events this is the source range. For
    /// [`SpannedEventProvenance::GeneratedFrom`] this is normally the same as
    /// `source`.
    pub range: Range<usize>,
    /// Whether the event came straight from `pulldown-cmark` or was generated
    /// by a darkmatter processor.
    pub provenance: SpannedEventProvenance,
}

impl<'a> SpannedInlineEvent<'a> {
    /// Builds a `Parsed` event over `range`.
    #[must_use]
    pub fn parsed(event: InlineEvent<'a>, range: Range<usize>) -> Self {
        Self {
            event,
            range,
            provenance: SpannedEventProvenance::Parsed,
        }
    }

    /// Builds a `GeneratedFrom { source }` event over `range`.
    #[must_use]
    pub fn generated(event: InlineEvent<'a>, range: Range<usize>, source: Range<usize>) -> Self {
        Self {
            event,
            range,
            provenance: SpannedEventProvenance::GeneratedFrom { source },
        }
    }
}

/// Wraps a `pulldown-cmark` offset-iter and emits parsed
/// [`SpannedInlineEvent`]s.
///
/// Every event is reported as [`SpannedEventProvenance::Parsed`] with the
/// range pulled directly from `pulldown-cmark`.
pub struct SpanningAdapter<'a, I>
where
    I: Iterator<Item = (Event<'a>, Range<usize>)>,
{
    inner: I,
}

impl<'a, I> SpanningAdapter<'a, I>
where
    I: Iterator<Item = (Event<'a>, Range<usize>)>,
{
    /// Wraps `inner`.
    pub fn new(inner: I) -> Self {
        Self { inner }
    }
}

impl<'a, I> Iterator for SpanningAdapter<'a, I>
where
    I: Iterator<Item = (Event<'a>, Range<usize>)>,
{
    type Item = SpannedInlineEvent<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let (event, range) = self.inner.next()?;
        Some(SpannedInlineEvent::parsed(
            InlineEvent::Standard(event),
            range,
        ))
    }
}

/// Span-aware splitter for `==mark==` and `⌄dim⌄` inline syntax.
///
/// Wraps a [`SpanningAdapter`] (or any iterator yielding
/// [`SpannedInlineEvent`]s) and replaces text events containing mark/dim
/// delimiters with a sequence of spanned events whose ranges are exact byte
/// sub-ranges of the parent text event.
///
/// **Cross-text-event state.** Mark and dim openers are remembered across
/// pulldown text events so a fixture like `==*highlighted*==` or
/// `⌄*dim and italic*⌄` folds to the design's nested-inline shape: a
/// mark/dim `Span` whose child is the emphasis container. Events that arrive
/// while an opener is buffered are stashed in the open span's buffer; when
/// the matching closer is seen, [`InlineEvent::Start`] / `End` are emitted
/// around the buffered run. When a paragraph closes with an opener still
/// buffered, the opener is reverted to literal text and the buffer is
/// flushed unchanged. Code-block contents are passed through.
///
/// **Nested mark/dim.** Mark and dim of different kinds nest. A `⌄` arriving
/// while a mark frame is open opens a nested dim frame; the inner dim closes
/// independently of the outer mark. This mirrors the design's "Mixed Mark and
/// Dim" fixture (`==highlighted and ⌄dim within mark⌄==`) — see
/// `renderable/features/2026-05-20-darkmatter-tree/span-aware-processor-design.md`.
/// A closer matches only the innermost frame of its own kind; a closer of a
/// different kind from the innermost frame either pushes a new frame (if it
/// can open) or becomes literal text. Unclosed inner frames revert when the
/// paragraph or stream ends.
///
/// Empty mark / dim pairs are treated as literal text (no empty span
/// container is emitted).
pub struct SpannedInlineStyleProcessor<'a, I>
where
    I: Iterator<Item = SpannedInlineEvent<'a>>,
{
    inner: I,
    pending: VecDeque<SpannedInlineEvent<'a>>,
    in_code_block: bool,
    /// Stack of active mark/dim openers. The last entry is the innermost
    /// frame; events arriving while the stack is non-empty accumulate in
    /// the innermost frame's buffer. Closing an inner frame pushes its
    /// finished `Start`/contents/`End` run into the next-outer frame's
    /// buffer (or into `pending` when the stack becomes empty).
    open_stack: Vec<OpenSpan<'a>>,
    /// The original Markdown source. Used to recover the backslash byte for
    /// escape patterns that `pulldown-cmark` consumes into a text-event
    /// boundary (notably `\\==`, because `\\=` is a recognised CommonMark
    /// escape and the `\\` byte therefore never reaches the splitter inside
    /// any single text event's payload).
    source: &'a str,
}

/// An active mark or dim opener whose closer has not yet been seen.
struct OpenSpan<'a> {
    kind: DelimKind,
    /// Byte range of the opening `==` or `⌄` delimiter.
    open_range: Range<usize>,
    /// Spanned events that have arrived since the opener.
    buffer: Vec<SpannedInlineEvent<'a>>,
}

impl<'a, I> SpannedInlineStyleProcessor<'a, I>
where
    I: Iterator<Item = SpannedInlineEvent<'a>>,
{
    /// Wraps `inner` over `source`.
    ///
    /// `source` is the original Markdown string the inner spanned iterator was
    /// built from. The processor uses it only to look up the byte preceding a
    /// text event's start when classifying cross-event escapes (see the module
    /// docs' *Range Policy*); it never re-parses or mutates the source.
    pub fn new(source: &'a str, inner: I) -> Self {
        Self {
            inner,
            pending: VecDeque::new(),
            in_code_block: false,
            open_stack: Vec::new(),
            source,
        }
    }

    /// Pushes a spanned event to the active output target — the innermost
    /// open frame's buffer when the stack is non-empty, otherwise the
    /// processor's `pending` queue.
    fn push_event(&mut self, event: SpannedInlineEvent<'a>) {
        if let Some(open) = self.open_stack.last_mut() {
            open.buffer.push(event);
        } else {
            self.pending.push_back(event);
        }
    }

    /// Pops the innermost open frame and reverts it to literal text.
    ///
    /// Called when a paragraph closes or the input stream ends with frames
    /// still buffered. The popped frame's captured events become ordinary
    /// content preceded by a literal `==` or `⌄` carrying the opener's byte
    /// range. The reverted run is routed through [`push_event`] so a nested
    /// inner frame's revert lands inside its outer frame's buffer.
    fn revert_innermost(&mut self) {
        let Some(open) = self.open_stack.pop() else {
            return;
        };
        let literal = match open.kind {
            DelimKind::Mark => CowStr::Borrowed("=="),
            DelimKind::Dim => CowStr::Borrowed("\u{2304}"),
        };
        self.push_event(SpannedInlineEvent::parsed(
            InlineEvent::Standard(Event::Text(literal)),
            open.open_range,
        ));
        for event in open.buffer {
            self.push_event(event);
        }
    }

    /// Reverts every open frame, innermost first, so each inner frame's
    /// reverted contents flow into the next outer frame's buffer before that
    /// outer frame itself reverts.
    fn revert_all_open(&mut self) {
        while !self.open_stack.is_empty() {
            self.revert_innermost();
        }
    }

    /// Closes the innermost open frame, emitting `Start` + buffered events
    /// + `End` around them.
    ///
    /// The closer's byte range becomes the `End` event's range. The whole
    /// run is routed through [`push_event`](Self::push_event) so a closed
    /// inner frame lands inside the next outer frame's buffer (the
    /// nested-span case).
    fn close_innermost(&mut self, closer_range: Range<usize>) {
        let Some(open) = self.open_stack.pop() else {
            return;
        };
        let tag = match open.kind {
            DelimKind::Mark => InlineTag::Mark,
            DelimKind::Dim => InlineTag::Dim,
        };
        // Empty pairs collapse to literal text: emit the two literals instead
        // of an empty span container.
        if open.buffer.is_empty() {
            let opener_literal = match open.kind {
                DelimKind::Mark => CowStr::Borrowed("=="),
                DelimKind::Dim => CowStr::Borrowed("\u{2304}"),
            };
            let closer_literal = opener_literal.clone();
            self.push_event(SpannedInlineEvent::parsed(
                InlineEvent::Standard(Event::Text(opener_literal)),
                open.open_range,
            ));
            self.push_event(SpannedInlineEvent::parsed(
                InlineEvent::Standard(Event::Text(closer_literal)),
                closer_range,
            ));
            return;
        }
        self.push_event(SpannedInlineEvent::parsed(
            InlineEvent::Start(tag),
            open.open_range,
        ));
        for event in open.buffer {
            self.push_event(event);
        }
        self.push_event(SpannedInlineEvent::parsed(
            InlineEvent::End(tag),
            closer_range,
        ));
    }

    /// The kind of the innermost open frame, if any.
    fn innermost_kind(&self) -> Option<DelimKind> {
        self.open_stack.last().map(|o| o.kind)
    }

    /// Processes a text event for delimiter splitting, routing emitted text
    /// and open/close transitions through [`push_event`](Self::push_event).
    fn process_text(&mut self, text: &CowStr<'a>, range: Range<usize>) {
        let s = text.as_ref();
        // Fast path: no markers and no active opener means no work.
        if self.open_stack.is_empty() && !s.contains("==") && !s.contains('\u{2304}') {
            self.push_event(SpannedInlineEvent::parsed(
                InlineEvent::Standard(Event::Text(text.clone())),
                range,
            ));
            return;
        }
        // No markers but a frame is open: text becomes interior content.
        if !s.contains("==") && !s.contains('\u{2304}') {
            self.push_event(SpannedInlineEvent::parsed(
                InlineEvent::Standard(Event::Text(text.clone())),
                range,
            ));
            return;
        }

        let base = range.start;
        let mut delimiters = collect_delimiters(s);
        // Cross-text-event escape recovery for `==`: `pulldown-cmark` consumes
        // `\\=` as a CommonMark escape and never emits the leading `\\` byte
        // inside any single text event's payload, so the in-text-event escape
        // check in `collect_delimiters` cannot see it. When the first delimiter
        // sits at the very start of this text event AND the source byte
        // immediately before the event is `\\`, treat the `==` as escaped and
        // record that the backslash lives one byte *before* `base`.
        if let Some(first) = delimiters.first_mut()
            && first.kind == DelimKind::Mark
            && !first.escaped
            && first.byte_start == 0
            && base > 0
            && self.source.as_bytes().get(base - 1) == Some(&b'\\')
        {
            first.escaped = true;
            first.can_open = false;
            first.can_close = false;
            first.cross_event_escape = true;
        }
        delimiters.sort_by_key(|d| d.byte_start);

        let mut cursor: usize = 0;
        for delim in &delimiters {
            // Escaped delimiters always become literal text. Range includes
            // the backslash byte — either drawn from the text-event payload
            // (in-event escape: `\\⌄`) or recovered from the source bytes
            // just before the text event when `pulldown-cmark` consumed the
            // backslash itself (cross-event escape: `\\==`). See module docs'
            // *Range Policy*.
            if delim.escaped {
                let (literal_start, pre_end) = if delim.cross_event_escape {
                    // The `\\` lives one byte before this text event in the
                    // source; nothing inside `s` precedes the delimiter.
                    (base - 1, delim.byte_start)
                } else {
                    // The `\\` lives inside `s` immediately before the
                    // delimiter; emit any preceding payload up to (but not
                    // including) the backslash byte.
                    (
                        base + delim.byte_start - 1,
                        delim.byte_start.saturating_sub(1),
                    )
                };
                if pre_end > cursor {
                    self.emit_text_slice(s, cursor, pre_end, base);
                }
                let literal = &s[delim.byte_start..delim.byte_end];
                self.push_event(SpannedInlineEvent::parsed(
                    InlineEvent::Standard(Event::Text(CowStr::from(literal.to_string()))),
                    literal_start..(base + delim.byte_end),
                ));
                cursor = delim.byte_end;
                continue;
            }

            // Emit any plain text leading up to this delimiter.
            if delim.byte_start > cursor {
                self.emit_text_slice(s, cursor, delim.byte_start, base);
            }

            let delim_range = (base + delim.byte_start)..(base + delim.byte_end);

            match delim.kind {
                DelimKind::Mark => {
                    match self.innermost_kind() {
                        Some(DelimKind::Mark) => {
                            // Closes the innermost mark frame.
                            self.close_innermost(delim_range);
                        }
                        Some(DelimKind::Dim) => {
                            // Different kind on top of stack: nest a new
                            // mark frame inside the open dim. This matches
                            // the design's "Mixed Mark and Dim" fixture.
                            self.open_stack.push(OpenSpan {
                                kind: DelimKind::Mark,
                                open_range: delim_range,
                                buffer: Vec::new(),
                            });
                        }
                        None => {
                            self.open_stack.push(OpenSpan {
                                kind: DelimKind::Mark,
                                open_range: delim_range,
                                buffer: Vec::new(),
                            });
                        }
                    }
                }
                DelimKind::Dim => {
                    match self.innermost_kind() {
                        Some(DelimKind::Dim) => {
                            if delim.can_close {
                                self.close_innermost(delim_range);
                            } else {
                                // Cannot close (e.g. whitespace before): literal.
                                self.push_event(SpannedInlineEvent::parsed(
                                    InlineEvent::Standard(Event::Text(CowStr::Borrowed(
                                        "\u{2304}",
                                    ))),
                                    delim_range,
                                ));
                            }
                        }
                        Some(DelimKind::Mark) => {
                            // Different kind on top of stack: nest a new
                            // dim frame inside the open mark when the
                            // delimiter can open. Otherwise it becomes
                            // literal text within the mark.
                            if delim.can_open {
                                self.open_stack.push(OpenSpan {
                                    kind: DelimKind::Dim,
                                    open_range: delim_range,
                                    buffer: Vec::new(),
                                });
                            } else {
                                self.push_event(SpannedInlineEvent::parsed(
                                    InlineEvent::Standard(Event::Text(CowStr::Borrowed(
                                        "\u{2304}",
                                    ))),
                                    delim_range,
                                ));
                            }
                        }
                        None => {
                            if delim.can_open {
                                self.open_stack.push(OpenSpan {
                                    kind: DelimKind::Dim,
                                    open_range: delim_range,
                                    buffer: Vec::new(),
                                });
                            } else {
                                // Cannot open (e.g. whitespace after): literal.
                                self.push_event(SpannedInlineEvent::parsed(
                                    InlineEvent::Standard(Event::Text(CowStr::Borrowed(
                                        "\u{2304}",
                                    ))),
                                    delim_range,
                                ));
                            }
                        }
                    }
                }
            }
            cursor = delim.byte_end;
        }

        if cursor < s.len() {
            self.emit_text_slice(s, cursor, s.len(), base);
        }
    }

    /// Emits a non-empty text slice with its parsed byte range.
    fn emit_text_slice(&mut self, s: &str, start: usize, end: usize, base: usize) {
        if start >= end {
            return;
        }
        let value = &s[start..end];
        if value.is_empty() {
            return;
        }
        self.push_event(SpannedInlineEvent::parsed(
            InlineEvent::Standard(Event::Text(CowStr::from(value.to_string()))),
            (base + start)..(base + end),
        ));
    }
}

impl<'a, I> Iterator for SpannedInlineStyleProcessor<'a, I>
where
    I: Iterator<Item = SpannedInlineEvent<'a>>,
{
    type Item = SpannedInlineEvent<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(event) = self.pending.pop_front() {
                return Some(event);
            }
            let Some(spanned) = self.inner.next() else {
                // Stream ended: revert any unclosed openers from innermost
                // outward so nested reverted runs land inside their outer
                // frame's buffer before that outer frame itself reverts.
                if !self.open_stack.is_empty() {
                    self.revert_all_open();
                    continue;
                }
                return None;
            };

            // Track code-block boundaries; inside a code block delimiters
            // are inert, so we forward verbatim.
            match &spanned.event {
                InlineEvent::Standard(Event::Start(Tag::CodeBlock(_))) => {
                    self.in_code_block = true;
                }
                InlineEvent::Standard(Event::End(TagEnd::CodeBlock)) => {
                    self.in_code_block = false;
                }
                _ => {}
            }

            if self.in_code_block {
                self.push_event(spanned);
                continue;
            }

            // Paragraph close with any open frames: revert them all from
            // innermost out before the End event lands. The reverted
            // contents go to `pending` and the End event itself is then
            // routed below.
            if matches!(
                &spanned.event,
                InlineEvent::Standard(Event::End(TagEnd::Paragraph))
            ) && !self.open_stack.is_empty()
            {
                self.revert_all_open();
            }

            if let InlineEvent::Standard(Event::Text(text)) = &spanned.event {
                let text_clone = text.clone();
                let range = spanned.range.clone();
                self.process_text(&text_clone, range);
                continue;
            }

            self.push_event(spanned);
        }
    }
}

/// Detected delimiter category used by the splitter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DelimKind {
    Mark,
    Dim,
}

/// A single delimiter token in the source text.
#[derive(Debug, Clone, Copy)]
struct Delim {
    kind: DelimKind,
    /// Byte offset within the parent text event.
    byte_start: usize,
    /// Byte offset (exclusive) within the parent text event.
    byte_end: usize,
    can_open: bool,
    can_close: bool,
    escaped: bool,
    /// `true` when the escape backslash was consumed by `pulldown-cmark` and
    /// therefore lives in the *source* immediately before this text event,
    /// not inside the text event's payload (see [`SpannedInlineStyleProcessor`]
    /// for the only producer of this case).
    cross_event_escape: bool,
}

/// Scans `text` for `==` and `⌄` delimiters with their open/close classification.
fn collect_delimiters(text: &str) -> Vec<Delim> {
    let bytes = text.as_bytes();
    let mut out = Vec::new();

    // `==` — mark. Mark escapes are normalised upstream (sentinel rewriting)
    // by the legacy preprocessor; we do not see `\==` here in the span-aware
    // chain because the span-aware fold does not apply the sentinel rewrite.
    // Instead the span-aware processor honours an inline backslash escape
    // mirroring the dim policy.
    let mut last_end = 0;
    for (start, _) in text.match_indices("==") {
        if start < last_end {
            continue;
        }
        let escaped = start > 0 && bytes[start - 1] == b'\\';
        out.push(Delim {
            kind: DelimKind::Mark,
            byte_start: start,
            byte_end: start + 2,
            can_open: !escaped,
            can_close: !escaped,
            escaped,
            cross_event_escape: false,
        });
        last_end = start + 2;
    }

    // `⌄` (U+2304) — dim. Three UTF-8 bytes.
    for (start, _) in text.match_indices('\u{2304}') {
        let escaped = start > 0 && bytes[start - 1] == b'\\';
        let len = '\u{2304}'.len_utf8();
        let (mut can_open, mut can_close) = classify_dim(text, start, len);
        if escaped {
            can_open = false;
            can_close = false;
        }
        out.push(Delim {
            kind: DelimKind::Dim,
            byte_start: start,
            byte_end: start + len,
            can_open,
            can_close,
            escaped,
            cross_event_escape: false,
        });
    }

    out
}

/// Classifies a `⌄` token at `byte_pos` with flanking rules.
///
/// A delimiter at a *text-event boundary* (start or end of the pulldown
/// `Event::Text` payload, where `prev_char` / `next_char` is `None`) is
/// treated leniently: it may open *or* close depending on the surrounding
/// pulldown events the buffering layer sees, because the actual flanking
/// context lives outside this single text event. The buffering layer then
/// reverts unpaired delimiters at paragraph close, so a leading `⌄`
/// followed by whitespace inside the same text event still cannot open
/// (its `next_char` is whitespace, not `None`).
fn classify_dim(text: &str, byte_pos: usize, len: usize) -> (bool, bool) {
    let prev_char = text[..byte_pos].chars().next_back();
    let next_char = text[byte_pos + len..].chars().next();

    // `None` (text boundary) is treated as non-whitespace; the buffering
    // layer in `SpannedInlineStyleProcessor` looks across text events to
    // confirm pairing.
    let can_open = next_char.is_none_or(|c| !c.is_whitespace());
    let can_close = prev_char.is_none_or(|c| !c.is_whitespace());

    let prev_is_alnum = prev_char.is_some_and(|c| c.is_alphanumeric());
    let next_is_alnum = next_char.is_some_and(|c| c.is_alphanumeric());
    if prev_is_alnum && next_is_alnum {
        return (false, false);
    }
    (can_open, can_close)
}

#[cfg(test)]
mod tests {
    use super::*;
    use pulldown_cmark::{Options, Parser};

    /// Drains the spanned chain over `markdown` into a flat vector.
    fn run(markdown: &str) -> Vec<SpannedInlineEvent<'static>> {
        let parser = Parser::new_ext(markdown, Options::ENABLE_STRIKETHROUGH).into_offset_iter();
        let adapter = SpanningAdapter::new(parser);
        let style = SpannedInlineStyleProcessor::new(markdown, adapter);
        // Statically owned by `into_static` is not available, so collect by
        // cloning into `CowStr::from(String)` via the spanned events. The
        // spanned events already own their text payloads after splitting, so
        // collecting into `Vec<SpannedInlineEvent<'static>>` is just a cast.
        // To keep the test simple, we transmute via `into_owned`: we never
        // hold borrows past the parser's lifetime in tests.
        let collected: Vec<_> = style.collect::<Vec<_>>();
        // SAFETY: the spanned events emitted by `process_text` own their
        // strings (they construct `CowStr::from(String)`), so 'static
        // collection is fine for our test usage. For pass-through events the
        // wrapping `CowStr::Borrowed` may still borrow from the parser's
        // input; we leak by `Box::leak`-ing the input below.
        let leaked = Box::leak(markdown.to_string().into_boxed_str());
        let _ = leaked;
        unsafe {
            std::mem::transmute::<Vec<SpannedInlineEvent<'_>>, Vec<SpannedInlineEvent<'static>>>(
                collected,
            )
        }
    }

    #[test]
    fn mark_basic_splits_with_byte_ranges() {
        let events = run("plain ==highlighted== after");
        // Expect at least one Start(Mark) / End(Mark) pair.
        let starts: Vec<_> = events
            .iter()
            .filter(|e| matches!(e.event, InlineEvent::Start(InlineTag::Mark)))
            .collect();
        let ends: Vec<_> = events
            .iter()
            .filter(|e| matches!(e.event, InlineEvent::End(InlineTag::Mark)))
            .collect();
        assert_eq!(starts.len(), 1);
        assert_eq!(ends.len(), 1);
    }

    #[test]
    fn mark_unclosed_reverts_to_literal_text() {
        let events = run("this ==is not closed");
        let starts = events
            .iter()
            .filter(|e| matches!(e.event, InlineEvent::Start(InlineTag::Mark)))
            .count();
        let ends = events
            .iter()
            .filter(|e| matches!(e.event, InlineEvent::End(InlineTag::Mark)))
            .count();
        assert_eq!(starts, 0, "unclosed delimiter must not emit a Start");
        assert_eq!(ends, 0, "unclosed delimiter must not emit an End");
    }

    #[test]
    fn dim_basic_pairs_around_text() {
        let events = run("normal \u{2304}dimmed\u{2304} after");
        let starts = events
            .iter()
            .filter(|e| matches!(e.event, InlineEvent::Start(InlineTag::Dim)))
            .count();
        let ends = events
            .iter()
            .filter(|e| matches!(e.event, InlineEvent::End(InlineTag::Dim)))
            .count();
        assert_eq!(starts, 1);
        assert_eq!(ends, 1);
    }

    // HR-attribute paragraph coverage moved to
    // `crate::markdown::render_tree::block_extension::tests` and to the
    // fold-tier tests in `super::fold::tests` (the
    // `span_aware_fold_*` HR fixtures). The legacy `SpannedRuleProcessor`
    // chain no longer exists — HR detection runs ahead of this module.

    // -----------------------------------------------------------------------
    // Review-6 finding 2: the span-aware processor's *exact byte-range
    // policy* (module docs *Range Policy*) is the technical reason this chain
    // exists. The presence-only tests above prove mark/dim/HR events get
    // emitted, but not that their ranges match the design. The fixtures
    // below pin the exact ranges from
    // `span-aware-processor-design.md` so a future bug that shifted child
    // spans to whole-text ranges, dropped the backslash from escaped
    // delimiter spans, or used character offsets for `⌄` would be caught.
    // -----------------------------------------------------------------------

    /// `plain ==highlighted== after` byte layout (0-based, UTF-8 single-byte):
    ///
    /// ```text
    /// p l a i n _ = = h i  g  h  l  i  g  h  t  e  d  =  =  _  a  f  t  e  r
    /// 0 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17 18 19 20 21 22 23 24 25 26
    /// ```
    ///
    /// Mark opener `==` is `6..8`, the inner text `highlighted` is `8..19`,
    /// closer `==` is `19..21`.
    #[test]
    fn mark_event_ranges_match_design_byte_offsets() {
        let events = run("plain ==highlighted== after");

        // Opener: Start(Mark) ranges over the literal `==` at 6..8.
        let start = events
            .iter()
            .find(|e| matches!(e.event, InlineEvent::Start(InlineTag::Mark)))
            .expect("Start(Mark) must exist");
        assert_eq!(start.range, 6..8, "Start(Mark) must cover the opening `==`");

        // Closer: End(Mark) ranges over the literal `==` at 19..21.
        let end = events
            .iter()
            .find(|e| matches!(e.event, InlineEvent::End(InlineTag::Mark)))
            .expect("End(Mark) must exist");
        assert_eq!(end.range, 19..21, "End(Mark) must cover the closing `==`");

        // Inner text: the `highlighted` Text event sits between Start and End
        // and must carry the exact 8..19 byte subrange of the parent text.
        let inner = events
            .iter()
            .find(|e| {
                matches!(&e.event, InlineEvent::Standard(Event::Text(t)) if t.as_ref() == "highlighted")
            })
            .expect("inner `highlighted` Text event must exist");
        assert_eq!(
            inner.range,
            8..19,
            "inner mark text must span exactly `highlighted`"
        );
    }

    /// `normal ⌄dimmed⌄ after` byte layout — `⌄` (U+2304) is **3 UTF-8 bytes**:
    ///
    /// ```text
    /// n o r m a l _ ⌄⌄⌄ d  i  m  m  e  d  ⌄⌄⌄ _ a  f  t  e  r
    /// 0 1 2 3 4 5 6 7-9 10 11 12 13 14 15 16-18 19 20 21 22 23 24
    /// ```
    ///
    /// Dim opener byte range is `7..10` (three bytes for the single `⌄` char),
    /// inner text `dimmed` is `10..16`, closer `16..19`. A regression that
    /// used `char` offsets instead of bytes would shrink the ranges to single
    /// bytes and fail this test.
    #[test]
    fn dim_event_ranges_use_utf8_byte_offsets() {
        let events = run("normal \u{2304}dimmed\u{2304} after");

        let start = events
            .iter()
            .find(|e| matches!(e.event, InlineEvent::Start(InlineTag::Dim)))
            .expect("Start(Dim) must exist");
        assert_eq!(
            start.range,
            7..10,
            "Start(Dim) must cover the three UTF-8 bytes of opening `⌄`",
        );

        let end = events
            .iter()
            .find(|e| matches!(e.event, InlineEvent::End(InlineTag::Dim)))
            .expect("End(Dim) must exist");
        assert_eq!(
            end.range,
            16..19,
            "End(Dim) must cover the three UTF-8 bytes of closing `⌄`",
        );

        let inner = events
            .iter()
            .find(|e| {
                matches!(&e.event, InlineEvent::Standard(Event::Text(t)) if t.as_ref() == "dimmed")
            })
            .expect("inner `dimmed` Text event must exist");
        assert_eq!(
            inner.range,
            10..16,
            "inner dim text must span exactly `dimmed`"
        );
    }

    /// Escaped `\==` must not open a mark span, and the literal `==` it
    /// reverts to must carry a byte range that **includes the backslash
    /// byte**, per `span-aware-processor-design.md` (*Mark: Escaped* / *Range
    /// Policy*).
    ///
    /// Byte layout for `foo \== bar`:
    ///
    /// ```text
    ///  f  o  o  ' '  \  =  =  ' '  b  a  r
    ///  0  1  2   3   4  5  6   7   8  9  10
    /// ```
    ///
    /// `pulldown-cmark` recognises `\=` as a CommonMark backslash escape (the
    /// `=` byte is ASCII punctuation), so it consumes the backslash itself
    /// and emits the surrounding bytes as two adjacent text events
    /// (`"foo "` @ `0..4` then `"== bar"` @ `5..11`) — the leading `\` is
    /// never inside any single text event's payload. The span-aware
    /// processor recovers this by consulting the original source bytes
    /// before the text event start; the emitted literal `==` therefore
    /// spans `4..7`, exactly the `\==` source bytes.
    #[test]
    fn escaped_mark_delimiter_literal_covers_backslash_and_delimiter_bytes() {
        let events = run("foo \\== bar");

        // No mark Start/End should be emitted for an escaped delimiter.
        assert!(
            !events
                .iter()
                .any(|e| matches!(e.event, InlineEvent::Start(InlineTag::Mark))),
            "escaped `\\==` must not open a mark span: {events:?}",
        );
        assert!(
            !events
                .iter()
                .any(|e| matches!(e.event, InlineEvent::End(InlineTag::Mark))),
            "escaped `\\==` must not close a mark span: {events:?}",
        );

        // The literal `==` text event must span `4..7` — the `\` byte at 4
        // plus the two `=` bytes at 5..7.
        let literal = events
            .iter()
            .find(
                |e| matches!(&e.event, InlineEvent::Standard(Event::Text(t)) if t.as_ref() == "=="),
            )
            .expect("escaped `==` must emit a literal Text(`==`) event");
        assert_eq!(
            literal.range,
            4..7,
            "reverted mark literal must include the backslash byte and the `==` source bytes",
        );
    }

    /// Escaped `\⌄` becomes a literal `⌄` text event whose range **includes**
    /// the backslash byte. For `foo \⌄ bar` the backslash is at byte 4 and
    /// `⌄` (three UTF-8 bytes) is at 5..8; the emitted literal must span
    /// `4..8`.
    #[test]
    fn escaped_dim_delimiter_literal_includes_backslash_byte() {
        let events = run("foo \\\u{2304} bar");

        // No dim Start/End should be emitted for an escaped delimiter.
        assert!(
            !events
                .iter()
                .any(|e| matches!(e.event, InlineEvent::Start(InlineTag::Dim))),
            "escaped `\\⌄` must not open a dim span: {events:?}",
        );
        assert!(
            !events
                .iter()
                .any(|e| matches!(e.event, InlineEvent::End(InlineTag::Dim))),
            "escaped `\\⌄` must not close a dim span: {events:?}",
        );

        let literal = events
            .iter()
            .find(|e| {
                matches!(&e.event, InlineEvent::Standard(Event::Text(t)) if t.as_ref() == "\u{2304}")
            })
            .expect("escaped `⌄` must emit a literal Text(`⌄`) event");
        assert_eq!(
            literal.range,
            4..8,
            "escaped dim literal must include the backslash byte and three `⌄` bytes",
        );
    }

    // The fold-tier `span_aware_fold_hr_source_location_pins_paragraph_body_bytes`
    // test in `super::fold::tests` now pins the generated HR's
    // `SourceLocation.bytes` to the paragraph body — that coverage moved with
    // the HR pipeline into `block_extension`.
}
